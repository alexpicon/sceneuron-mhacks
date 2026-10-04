// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the scaled live preview, which also plays the overlay's emoji floods, with moving the selected
   element by pointer drag or arrow keys. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const Render = window.SceneuronRender;
  const { findElement, moveElement } = Editor.project;
  const { composePreview, plannedIds } = Editor.snapshot;
  const { dragTarget } = Editor.drag;

  /** Wires the preview's pointer and keyboard moves; returns the view with schedule and showPlan. */
  Editor.preview = function setupPreview(editor) {
    const { app, ui } = editor;

    function schedule() {
      if (app.previewQueued) {
        return;
      }
      app.previewQueued = true;
      window.requestAnimationFrame(() => {
        app.previewQueued = false;
        render();
      });
    }

    /** Shows the project `plan` would produce until it is applied or dropped; null shows the draft. */
    function showPlan(plan) {
      app.planPreview = plan && plan.project ? { project: plan.project, ids: plannedIds(plan) } : null;
      ui.preview.classList.toggle("is-plan-preview", Boolean(app.planPreview));
      ui.preview.setAttribute("aria-label", app.planPreview ? "Overlay preview of the proposed plan" : "Overlay preview");
      schedule();
    }

    function render() {
      const view = composePreview(app.planPreview ? app.planPreview.project : app.draft, app.live);
      if (!view.canvas) {
        return;
      }
      ui.preview.style.setProperty("aspect-ratio", `${view.canvas.width} / ${view.canvas.height}`);
      app.fit = Render.applyFit(ui.previewRoot, view.canvas, ui.preview.clientWidth, ui.preview.clientHeight);
      ui.preview.style.setProperty("--preview-scale", String(app.fit.scale || 1));
      Render.render(ui.previewRoot, view, { ghostHidden: true });
      if (app.live) {
        window.SceneuronFlood.update(ui.previewRoot, app.live);
      }
      for (const node of ui.previewRoot.children) {
        const id = node.getAttribute("data-element-id");
        if (!id) {
          continue;
        }
        node.classList.toggle("is-selected", !app.planPreview && id === app.selectedId);
        node.classList.toggle("is-planned", Boolean(app.planPreview) && app.planPreview.ids.has(id));
      }
    }

    ui.preview.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || !app.draft || app.planPreview) {
        return;
      }
      ui.preview.focus({ preventScroll: true });
      const node = event.target.closest("[data-element-id]");
      const element = node && findElement(app.draft, node.getAttribute("data-element-id"));
      if (!element) {
        return;
      }
      event.preventDefault();
      editor.select(element.id);
      app.drag = {
        id: element.id,
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        x: element.props.x,
        y: element.props.y,
        moving: false,
      };
      ui.preview.setPointerCapture(event.pointerId);
      ui.preview.classList.add("dragging");
    });

    ui.preview.addEventListener("pointermove", (event) => {
      const drag = app.drag;
      if (!drag || drag.pointerId !== event.pointerId || !app.fit || !app.fit.scale) {
        return;
      }
      const target = dragTarget(drag, event.clientX, event.clientY, app.fit.scale);
      if (!target) {
        return;
      }
      drag.moving = true;
      if (moveElement(app.draft, drag.id, target.x, target.y)) {
        editor.draftChanged({ form: true });
      }
    });

    const endDrag = (event) => {
      if (app.drag && app.drag.pointerId === event.pointerId) {
        app.drag = null;
        ui.preview.classList.remove("dragging");
      }
    };
    ui.preview.addEventListener("pointerup", endDrag);
    ui.preview.addEventListener("pointercancel", endDrag);

    ui.preview.addEventListener("keydown", (event) => {
      const deltas = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
      const delta = deltas[event.key];
      const element = findElement(app.draft, app.selectedId);
      if (!delta || !element || app.planPreview || event.altKey || event.ctrlKey || event.metaKey) {
        return;
      }
      event.preventDefault();
      const step = event.shiftKey ? 10 : 1;
      const x = element.props.x + delta[0] * step;
      const y = element.props.y + delta[1] * step;
      if (moveElement(app.draft, element.id, x, y)) {
        editor.draftChanged({ form: true });
        editor.announce(`${element.name} at x ${element.props.x}, y ${element.props.y}.`);
      }
    });

    if (typeof ResizeObserver === "function") {
      new ResizeObserver(schedule).observe(ui.preview);
    } else {
      window.addEventListener("resize", schedule);
    }

    return { schedule, showPlan };
  };
})();
