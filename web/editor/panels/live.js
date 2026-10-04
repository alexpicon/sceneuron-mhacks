// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the live controls for the selected element: show or hide holds, text holds, effects,
   and release, plus releasing every hold. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_TEXT_CHARS } = Editor.constants;
  const { findElement } = Editor.project;
  const { isValidText } = Editor.values;

  /** Wires the live controls; returns the view with render. */
  Editor.live = function setupLive(editor) {
    const { app, ui, api } = editor;

    function render() {
      const element = findElement(app.draft, app.selectedId);
      const live = editor.liveElement(app.selectedId);
      let target = "Select an element to control it live.";
      if (element && live) {
        target = `Controlling ${element.name}. Holds stay until released, even when rules run.`;
      } else if (element) {
        target = `${element.name} is not saved yet. Save to control it live.`;
      }
      if (ui.liveTarget.textContent !== target) {
        ui.liveTarget.textContent = target;
      }
      for (const control of ui.liveControls.querySelectorAll("button, input")) {
        control.disabled = !live;
      }
      const textAllowed = Boolean(live) && live.kind === "text";
      ui.holdText.disabled = !textAllowed;
      ui.holdTextButton.disabled = !textAllowed;
      if (live && app.holdTextFor !== live.id && document.activeElement !== ui.holdText) {
        ui.holdText.value = live.props.text || "";
        app.holdTextFor = live.id;
      }
      const held = live && live.overridden.length ? `Held: ${live.overridden.join(", ")}` : "Nothing held";
      if (ui.liveHeld.textContent !== held) {
        ui.liveHeld.textContent = held;
      }
    }

    function holdText() {
      const text = ui.holdText.value;
      if (!isValidText(text)) {
        editor.toast(`Text must be at most ${MAX_TEXT_CHARS} characters.`, "error");
        return;
      }
      editor.act(api.override(app.selectedId, { text }), "Text held.");
    }

    ui.liveControls.addEventListener("click", (event) => {
      const button = event.target.closest("button");
      const id = app.selectedId;
      if (!button || !id) {
        return;
      }
      const visible = button.getAttribute("data-hold-visible");
      const effect = button.getAttribute("data-effect");
      if (visible !== null) {
        const show = visible === "true";
        editor.act(api.override(id, { visible: show }), show ? "Shown and held." : "Hidden and held.");
      } else if (effect !== null) {
        editor.act(api.effect(id, effect));
      } else if (button === ui.holdTextButton) {
        holdText();
      } else if (button === ui.releaseElement) {
        editor.act(api.release(id), "Released.");
      }
    });

    ui.holdText.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        holdText();
      }
    });

    ui.releaseAll.addEventListener("click", () => editor.act(api.release(null), "Released all overrides."));

    return { render };
  };
})();
