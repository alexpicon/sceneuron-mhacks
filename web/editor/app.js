// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the shared editor object, which holds the state and the page's shared controls, and the
   flows that keep the draft, the live snapshot, and every panel in step. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { createApi } = Editor.api;
  const { isDirty } = Editor.project;
  const { acceptSnapshot, isSnapshot } = Editor.snapshot;

  /**
   * Creates the editor object that every panel receives. main.js completes it: each panel adds its
   * view to `editor.views`, and notifications.js and sync.js add their functions to the object itself.
   */
  function createEditor() {
    const byId = (id) => document.getElementById(id);
    const link = window.SceneuronLink;
    const token = link.tokenFrom(window.location);
    const moved = link.fragmentAddress(window.location);
    if (moved) {
      window.history.replaceState(null, "", moved);
    }
    // One proof at a time; a failed or lost connection asks again before the token is sent again.
    const trust = { verified: false, pending: null };
    const fetchImpl = (input, init) => window.fetch(input, init);
    const trusted = () => {
      if (trust.verified) {
        return Promise.resolve(true);
      }
      if (!trust.pending) {
        const port = Number(link.pagePort(window.location));
        trust.pending = link.verifyServer(token, port, fetchImpl, window.crypto).then((ok) => {
          trust.verified = ok;
          trust.pending = null;
          return ok;
        });
      }
      return trust.pending;
    };
    const distrust = () => {
      trust.verified = false;
    };
    const ui = {
      connection: byId("connection-chip"),
      presenceChip: byId("presence-chip"),
      presenceText: byId("presence-text"),
      vision: byId("vision-chip"),
      automationChip: byId("automation-chip"),
      automationToggle: byId("automation-toggle"),
      dirty: byId("dirty-indicator"),
      discard: byId("discard-button"),
      save: byId("save-button"),
      banner: byId("banner"),
      bannerText: byId("banner-text"),
      bannerRetry: byId("banner-retry"),
      list: byId("element-list"),
      deleteElement: byId("delete-element"),
      preview: byId("preview"),
      previewRoot: byId("preview-root"),
      liveTarget: byId("live-target"),
      liveControls: byId("live-controls"),
      holdText: byId("hold-text"),
      holdTextButton: byId("hold-text-button"),
      liveHeld: byId("live-held"),
      releaseElement: byId("release-element"),
      releaseAll: byId("release-all"),
      eventLog: byId("event-log"),
      eventEmpty: byId("event-empty"),
      propsEmpty: byId("props-empty"),
      propsForm: byId("props-form"),
      rulesList: byId("rules-list"),
      rulesEmpty: byId("rules-empty"),
      addRule: byId("add-rule"),
      settingsForm: byId("settings-form"),
      sourceList: byId("source-list"),
      sourcesEmpty: byId("sources-empty"),
      refreshSources: byId("refresh-sources"),
      addOverlay: byId("add-overlay"),
      overlayUrl: byId("overlay-url"),
      copyOverlayUrl: byId("copy-overlay-url"),
      toasts: byId("toasts"),
      announcer: byId("announcer"),
    };
    const app = {
      saved: null,
      draft: null,
      live: null,
      trustNextStream: true,
      streamOpened: false,
      selectedId: null,
      formFor: null,
      holdTextFor: null,
      listKey: "",
      eventsKey: "",
      fit: null,
      drag: null,
      saving: false,
      previewQueued: false,
      sourcesLoaded: false,
      // The name of the video source camera analysis follows, once the Camera & mic tab listed the sources.
      analyzedSource: null,
      // The /api/audio-sources reply, or null until it loads.
      microphones: null,
      bannerRetry: null,
      // The project revision the draft is based on; saves send it so a newer project is never overwritten.
      baseRevision: null,
      conflict: false,
      catchingUp: false,
      loadSequence: 0,
      // The AI settings from /api/ai, or null until they load.
      ai: null,
      liveReceivedAt: 0,
    };
    const views = {};
    const editor = { token, byId, ui, app, views, trusted, distrust, api: createApi(token, fetchImpl, trusted) };

    editor.liveElement = (id) => (app.live && app.live.elements.find((element) => element.id === id)) || null;

    editor.applySnapshot = function applySnapshot(snapshot, fromStream) {
      const trusted = fromStream && app.trustNextStream;
      if (!trusted && !acceptSnapshot(app.live, snapshot)) {
        return;
      }
      if (fromStream) {
        app.trustNextStream = false;
      }
      const previous = app.live;
      app.live = snapshot;
      app.liveReceivedAt = Date.now();
      const before = previous && previous.presence ? previous.presence.state : null;
      if (before && snapshot.presence && before !== snapshot.presence.state) {
        editor.announce(`Presence is now ${snapshot.presence.state}.`);
      }
      views.status.render();
      views.aiStatus.render();
      views.elements.render();
      views.live.render();
      views.properties.renderBadges();
      views.conditions.renderLive();
      views.cards.renderLive();
      views.voice.renderLive();
      views.subtitles.renderLive();
      views.cameraPreview.renderLive();
      views.events.render();
      views.reactions.render();
      views.preview.schedule();
      editor.watchRevision(snapshot);
    };

    /** Applies the snapshot a live control's request returns, or reports its failure. */
    editor.act = async function act(request, success) {
      try {
        const snapshot = await request;
        if (isSnapshot(snapshot)) {
          editor.applySnapshot(snapshot, false);
        }
        if (success) {
          editor.announce(success);
        }
      } catch (error) {
        editor.report(error);
      }
    };

    editor.renderDirty = function renderDirty() {
      const dirty = Boolean(app.draft) && isDirty(app.saved, app.draft);
      if (ui.dirty.dataset.dirty !== String(dirty)) {
        ui.dirty.dataset.dirty = String(dirty);
        ui.dirty.textContent = dirty ? "Unsaved changes" : "All changes saved";
      }
      ui.discard.disabled = !dirty;
      document.title = dirty ? "* Sceneuron editor" : "Sceneuron editor";
    };

    /**
     * Redraws after a draft edit; `parts` names the views it changed: list, rules, conditions, decks, form,
     * chips, or theme. The decks name their card slot elements, so a change to the element list redraws them too,
     * and the property form's notes follow the theme and which elements are card slots.
     */
    editor.draftChanged = function draftChanged(parts) {
      editor.renderDirty();
      views.preview.schedule();
      if (parts.list) {
        views.elements.render();
      }
      if (parts.rules) {
        views.rules.render();
      }
      if (parts.conditions) {
        views.conditions.render();
        views.assistant.render();
      }
      if (parts.list || parts.decks) {
        views.cards.render();
        views.packs.render();
      }
      if (parts.form) {
        views.properties.syncValues(false);
      }
      if (parts.theme || parts.decks) {
        views.properties.render();
      }
      if (parts.form || parts.chips) {
        views.placeholderChips.render();
      }
    };

    editor.refreshAll = function refreshAll() {
      views.status.render();
      editor.renderDirty();
      views.elements.render();
      views.properties.render();
      views.placeholderChips.render();
      views.live.render();
      views.rules.render();
      views.conditions.render();
      views.cards.render();
      views.packs.render();
      views.room.renderDecks();
      views.settings.sync();
      views.theme.sync();
      views.reactions.render();
      views.aiSettings.render();
      views.events.render();
      views.preview.schedule();
    };

    /** Selects the element with this id in the list, the preview, the form, and the live controls. */
    editor.select = function select(id) {
      if (app.selectedId === id) {
        return;
      }
      app.selectedId = id;
      views.elements.render();
      views.properties.render();
      views.placeholderChips.render();
      views.live.render();
      views.preview.schedule();
    };

    return editor;
  }

  Editor.createEditor = createEditor;
})();
