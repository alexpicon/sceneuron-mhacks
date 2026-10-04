// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: one place where the streamer asks the assistant for a plan, used by the assistant box and
   the setup wizard: waiting with a way to stop, showing the plan, and applying it with the revision it was
   made for, so a plan never overwrites a newer project. The server cannot cancel a request; stopping only
   makes the editor ignore its answer. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { isConflict } = Editor.api;
  const { isDirty } = Editor.project;
  const { planErrorMessage } = Editor.aiText;
  const { describePlan } = Editor.planText;
  const { errorView, planView, progressView, stoppedView } = Editor.planView;

  /**
   * Creates a session that draws into `container` and asks for plans in `mode` ("edit" or "setup").
   * `onSettled(outcome)` hears "busy", "idle", "applied", and "discarded".
   */
  Editor.planSession = function createPlanSession(editor, { container, mode, onSettled }) {
    const { app, api } = editor;
    let ticket = 0;
    let timer = null;
    let request = null;
    let plan = null;
    let lines = [];
    const settled = (outcome) => onSettled && onSettled(outcome);
    // The preview shows the plan's result while it waits for review.
    const preview = (shown) => editor.views.preview && editor.views.preview.showPlan(shown);

    function show(node, focusSelector) {
      container.replaceChildren(node);
      const target = focusSelector ? container.querySelector(focusSelector) : null;
      if (target) {
        target.focus();
      }
    }

    function stopTimer() {
      window.clearInterval(timer);
      timer = null;
    }

    function renderPlan(state, focusSelector) {
      const dirty = Boolean(app.draft) && isDirty(app.saved, app.draft);
      show(planView(plan, lines, mode, Object.assign({ dirty }, state)), focusSelector);
    }

    async function propose(text, useCamera) {
      const mine = (ticket += 1);
      request = { text, useCamera };
      plan = null;
      preview(null);
      const started = Date.now();
      stopTimer();
      show(progressView(mode), '[data-plan-action="cancel"]');
      timer = window.setInterval(() => {
        const elapsed = container.querySelector("[data-elapsed]");
        if (elapsed) {
          elapsed.textContent = `${Math.round((Date.now() - started) / 1000)} s`;
        }
      }, 1000);
      editor.announce("Asking the assistant. This can take up to two minutes.");
      settled("busy");
      try {
        const result = await api.plan(text, mode, useCamera);
        if (mine !== ticket) {
          return;
        }
        plan = result;
        lines = describePlan(result, app.saved);
        renderPlan({}, "h3");
        preview(plan);
        const changes = lines.length === 1 ? "1 change" : `${lines.length} changes`;
        editor.announce(`The plan is ready with ${changes}. Review it, then apply or discard it.`);
      } catch (error) {
        if (mine !== ticket) {
          return;
        }
        show(errorView(planErrorMessage(error)), '[data-plan-action="again"]');
      } finally {
        if (mine === ticket) {
          stopTimer();
          settled("idle");
        }
      }
    }

    function cancel() {
      if (timer === null) {
        return;
      }
      ticket += 1;
      stopTimer();
      show(stoppedView());
      editor.announce("Stopped waiting for the assistant.");
      settled("idle");
    }

    async function apply(confirmed) {
      if (!plan || app.saving) {
        return;
      }
      if (!confirmed && app.draft && isDirty(app.saved, app.draft)) {
        renderPlan({ confirm: true }, '[data-plan-action="confirm"]');
        return;
      }
      app.saving = true;
      renderPlan({ busy: true });
      try {
        const snapshot = await api.saveProject(plan.project, plan.base_revision);
        const added = plan.operations.find((op) => op.op === "add_element");
        if (added) {
          app.selectedId = added.element.id;
        }
        const changes = lines.length === 1 ? "1 change" : `${lines.length} changes`;
        editor.adopt(plan.project, snapshot);
        plan = null;
        preview(null);
        container.replaceChildren();
        editor.toast(`Plan applied: ${changes}.`, "info");
        settled("applied");
      } catch (error) {
        const state = isConflict(error) ? { conflict: true } : { error: error.message };
        renderPlan(state, isConflict(error) ? '[data-plan-action="again"]' : '[data-plan-action="apply"]');
      } finally {
        app.saving = false;
        editor.renderDirty();
      }
    }

    function discard() {
      const had = plan !== null;
      plan = null;
      preview(null);
      container.replaceChildren();
      editor.announce(had ? "Plan discarded." : "Dismissed.");
      settled("discarded");
    }

    container.addEventListener("click", (event) => {
      const button = event.target.closest("[data-plan-action]");
      const name = button ? button.getAttribute("data-plan-action") : null;
      if (name === "cancel") {
        cancel();
      } else if (name === "again" && request) {
        propose(request.text, request.useCamera);
      } else if (name === "apply") {
        apply(false);
      } else if (name === "confirm") {
        apply(true);
      } else if (name === "keep") {
        renderPlan({}, '[data-plan-action="apply"]');
      } else if (name === "discard") {
        discard();
      }
    });

    return { busy: () => timer !== null, cancel, propose };
  };
})();
