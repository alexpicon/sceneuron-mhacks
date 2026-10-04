// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: keeping in step with the server: the live event stream, loading the project, saving the
   draft, discarding it, and noticing when another editor or the assistant saved first. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { isConflict } = Editor.api;
  const { clone, findElement, isDirty, projectForSave } = Editor.project;
  const { isSnapshot } = Editor.snapshot;

  const CHANGED_ELSEWHERE =
    "The project was changed in another editor or by the assistant, and your unsaved edits here are based on " +
    "the older version. Reload to get the latest project; your unsaved edits here will be lost.";
  const SAVE_REFUSED =
    "Not saved: the project was changed in another editor or by the assistant since you loaded it, and saving " +
    "would overwrite that change. Reload to get the latest project; your unsaved edits here will be lost.";

  /** Returns the editor's server functions: connectEvents, loadProject, adopt, save, discard, and watchRevision. */
  Editor.sync = function setupSync(editor) {
    const { app, ui, views, api, token } = editor;

    // Reconnects back off to 10 s; after this many in a row the editor stops and says why.
    const MAX_RECONNECTS = 30;
    let reconnects = 0;

    /** Opens the live event stream once the server proved it holds the token. */
    async function connectEvents(delay) {
      if (!(await editor.trusted())) {
        retryEvents(delay);
        return;
      }
      const source = new EventSource(`/api/events?token=${encodeURIComponent(token)}`);
      const onState = (event) => {
        let snapshot = null;
        try {
          snapshot = JSON.parse(event.data);
        } catch {
          return;
        }
        if (isSnapshot(snapshot)) {
          delay = 0;
          editor.applySnapshot(snapshot, true);
        }
      };
      source.addEventListener("open", () => {
        reconnects = 0;
        app.trustNextStream = true;
        views.status.setConnection("live");
        // A reconnect can follow a server restart that loaded a different project; unsaved edits are kept.
        if (app.streamOpened && (!app.draft || !isDirty(app.saved, app.draft))) {
          loadProject();
        }
        app.streamOpened = true;
      });
      source.addEventListener("state", onState);
      source.addEventListener("message", onState);
      source.addEventListener("error", () => {
        // EventSource would reconnect by itself and send the token again without asking who
        // answers, so it is closed and the server is proved again first.
        source.close();
        editor.distrust();
        retryEvents(delay);
      });
    }

    function retryEvents(delay) {
      reconnects += 1;
      if (reconnects > MAX_RECONNECTS) {
        views.status.setConnection("offline");
        editor.showBanner(
          "Sceneuron stopped answering at this address. When OBS is running again, try again, or open the " +
            "editor from OBS (Tools > Sceneuron).",
          () => {
            reconnects = 0;
            connectEvents(0);
          },
        );
        return;
      }
      views.status.setConnection(reconnects > 1 ? "offline" : "connecting");
      const next = Math.min(10000, delay ? delay * 2 : 1000);
      window.setTimeout(() => connectEvents(next), next);
    }

    /** Makes `project` both the saved project and the draft, at the revision `snapshot` reports. */
    function adopt(project, snapshot, revision) {
      app.saved = clone(project);
      app.draft = clone(project);
      app.baseRevision = isSnapshot(snapshot) ? snapshot.project_revision : revision;
      if (!findElement(app.draft, app.selectedId)) {
        app.selectedId = app.draft.elements.length ? app.draft.elements[0].id : null;
      }
      app.formFor = null;
      if (app.conflict) {
        app.conflict = false;
        ui.banner.hidden = true;
      }
      if (isSnapshot(snapshot)) {
        editor.applySnapshot(snapshot, false);
      }
      editor.refreshAll();
    }

    /** Loads the saved project; unless `force`, unsaved edits are kept instead. */
    async function loadProject(force) {
      const sequence = (app.loadSequence = (app.loadSequence || 0) + 1);
      try {
        // The revision is read before the project, so the project is at least that new: a save in
        // between can only cause a needless conflict, never an unnoticed overwrite.
        const state = await api.getState();
        const project = await api.getProject();
        if (sequence !== app.loadSequence) {
          return;
        }
        if (!app.conflict) {
          ui.banner.hidden = true;
        }
        // Edits made while the request was in flight win over a refresh.
        if (!force && app.draft && isDirty(app.saved, app.draft)) {
          return;
        }
        adopt(project, null, state && Number.isFinite(state.project_revision) ? state.project_revision : null);
      } catch (error) {
        editor.showBanner(`Could not load the project: ${error.message}`, () => loadProject(force));
      }
    }

    function showConflict(message) {
      app.conflict = true;
      editor.showBanner(message, async () => {
        await loadProject(true);
        if (!app.conflict) {
          editor.announce("Reloaded the latest project.");
          ui.save.focus();
        }
      }, "Reload");
    }

    /**
     * Follows saves made elsewhere: a clean draft quietly catches up, and a draft with unsaved
     * edits gets a warning before its save would be refused.
     */
    function watchRevision(snapshot) {
      const revision = snapshot.project_revision;
      const known = Number.isFinite(revision) && Number.isFinite(app.baseRevision);
      if (!known || revision <= app.baseRevision || app.saving || !app.draft) {
        return;
      }
      if (!isDirty(app.saved, app.draft)) {
        if (!app.catchingUp) {
          app.catchingUp = true;
          loadProject().finally(() => {
            app.catchingUp = false;
          });
        }
      } else if (!app.conflict) {
        showConflict(CHANGED_ELSEWHERE);
      }
    }

    async function save() {
      if (!app.draft || app.saving) {
        return;
      }
      // A rejected entry never reaches the draft, so saving now would store an older value than the one shown.
      const invalid = document.querySelector('[aria-invalid="true"]');
      if (invalid) {
        views.tabs.reveal(invalid);
        editor.toast("Fix the highlighted field before saving. Its entry has not been applied.", "error");
        return;
      }
      app.saving = true;
      ui.save.setAttribute("aria-busy", "true");
      ui.save.textContent = "Saving";
      const payload = projectForSave(app.draft, app.live);
      try {
        const snapshot = await api.saveProject(payload, app.baseRevision);
        app.saved = clone(payload);
        app.draft.automation_enabled = payload.automation_enabled;
        views.room.renderDecks();
        if (isSnapshot(snapshot)) {
          app.baseRevision = snapshot.project_revision;
          editor.applySnapshot(snapshot, false);
        }
        editor.toast("Saved.", "info");
      } catch (error) {
        if (isConflict(error)) {
          showConflict(SAVE_REFUSED);
        } else {
          editor.report(error);
        }
      } finally {
        app.saving = false;
        ui.save.removeAttribute("aria-busy");
        ui.save.textContent = "Save";
        editor.renderDirty();
        views.elements.render();
        views.live.render();
      }
    }

    async function discard() {
      if (!app.saved) {
        return;
      }
      // The saved copy is out of date after a save elsewhere, so discarding means taking the latest project.
      if (app.conflict) {
        await loadProject(true);
        ui.save.focus();
        editor.announce("Discarded unsaved changes and loaded the latest project.");
        return;
      }
      app.draft = clone(app.saved);
      if (!findElement(app.draft, app.selectedId)) {
        app.selectedId = app.draft.elements.length ? app.draft.elements[0].id : null;
      }
      app.formFor = null;
      editor.refreshAll();
      ui.save.focus();
      editor.announce("Discarded unsaved changes.");
    }

    return { adopt, connectEvents, discard, loadProject, save, watchRevision };
  };
})();
