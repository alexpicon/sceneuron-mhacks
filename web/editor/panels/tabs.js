// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the side panel's tabs (properties, rules, cards, AI, settings, and camera and mic), with arrow
   key moves. The camera preview runs only while its tab is shown. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;

  /** Wires the tabs; returns the view with activate and reveal. */
  Editor.tabs = function setupTabs(editor) {
    const { app, byId, views } = editor;
    const tabs = Array.from(document.querySelectorAll('[role="tab"]'));

    function activate(tab, focus) {
      for (const other of tabs) {
        const selected = other === tab;
        other.setAttribute("aria-selected", String(selected));
        other.tabIndex = selected ? 0 : -1;
        byId(other.getAttribute("aria-controls")).hidden = !selected;
      }
      if (focus) {
        tab.focus();
      }
      if (tab.id === "tab-camera" && !app.sourcesLoaded) {
        app.sourcesLoaded = true;
        views.camera.load();
      }
      // The AI tab's Voice and Subtitles sections say which microphone they hear, so both tabs bring it up to date.
      if (tab.id === "tab-camera" || tab.id === "tab-ai") {
        views.microphones.load();
      }
      views.cameraPreview.watch(tab.id === "tab-camera");
      if (tab.id === "tab-cards") {
        views.cards.refresh();
        views.packs.load();
        views.room.load();
      }
    }

    /** Shows a control on its tab, opening a collapsed section around it, and focuses it. */
    function reveal(control) {
      const panel = control.closest('[role="tabpanel"]');
      const tab = panel && tabs.find((candidate) => candidate.getAttribute("aria-controls") === panel.id);
      if (tab && panel.hidden) {
        activate(tab, false);
      }
      const details = control.closest("details");
      if (details) {
        details.open = true;
      }
      control.focus();
    }

    for (const tab of tabs) {
      tab.addEventListener("click", () => activate(tab, false));
    }
    document.querySelector('[role="tablist"]').addEventListener("keydown", (event) => {
      const index = tabs.indexOf(document.activeElement);
      const moves = { ArrowRight: index + 1, ArrowLeft: index - 1, Home: 0, End: tabs.length - 1 };
      if (index < 0 || !(event.key in moves)) {
        return;
      }
      event.preventDefault();
      activate(tabs[(moves[event.key] + tabs.length) % tabs.length], true);
    });

    return { activate, reveal };
  };
})();
