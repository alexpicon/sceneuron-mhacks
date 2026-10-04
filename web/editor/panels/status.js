// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the status bar (connection, presence, camera, and automation) and the controls that
   need a live connection: pause or resume automation, simulated presence, and release all. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { PRESENCE_STATES } = Editor.constants;
  const { presenceLabel, visionLabel } = Editor.snapshot;

  /** Wires the status controls; returns the view with render and setConnection. */
  Editor.status = function setupStatus(editor) {
    const { app, ui, api } = editor;
    const simulateButtons = Array.from(document.querySelectorAll("[data-simulate]"));

    function render() {
      const live = app.live;
      const presence = live ? live.presence : null;
      ui.presenceChip.dataset.state = presence && PRESENCE_STATES.includes(presence.state) ? presence.state : "unknown";
      ui.presenceText.textContent = presenceLabel(presence);
      const vision = live ? live.vision : null;
      ui.vision.dataset.state = vision && vision.state ? vision.state : "idle";
      ui.vision.textContent = visionLabel(vision);
      ui.vision.title = vision && vision.message ? String(vision.message) : "";
      const enabled = !live || live.automation_enabled !== false;
      ui.automationChip.dataset.state = enabled ? "on" : "paused";
      ui.automationChip.textContent = enabled ? "Automation on" : "Automation paused";
      ui.automationToggle.textContent = enabled ? "Pause automation" : "Resume automation";
      ui.automationToggle.disabled = !live;
      for (const button of simulateButtons) {
        button.disabled = !live;
      }
      ui.releaseAll.disabled = !live;
    }

    function setConnection(state) {
      const labels = { live: "Live", connecting: "Reconnecting", offline: "Offline" };
      ui.connection.dataset.state = state;
      ui.connection.textContent = labels[state];
    }

    ui.automationToggle.addEventListener("click", () => {
      const enabled = !(app.live && app.live.automation_enabled !== false);
      editor.act(api.automation(enabled), enabled ? "Automation resumed." : "Automation paused.");
    });
    for (const button of simulateButtons) {
      button.addEventListener("click", () => editor.act(api.simulate(button.getAttribute("data-simulate"))));
    }

    return { render, setConnection };
  };
})();
