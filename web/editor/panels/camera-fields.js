// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the Camera & mic tab: "Set up OBS for me" and its report, the camera preview,
   what on-device analysis finds, and its live description, the camera source list, the microphone list, and the
   overlay controls. main.js builds it before the editor looks up its controls. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { field, h } = Editor.dom;

  function setUpObs() {
    return [
      h("h3", {}, "Set up OBS"),
      h(
        "p",
        { class: "hint" },
        "One click puts what Sceneuron needs into OBS: the Sceneuron Overlay in the current scene, camera analysis " +
          "on your camera, and the Sceneuron Voice filter on Mic/Aux; it also pairs your Stream Deck when " +
          "Sceneuron's plugin is installed in the Stream Deck app. Whatever is already set up stays as it is.",
      ),
      h("button", { id: "setup-obs", type: "button" }, "Set up OBS for me"),
      h("ul", { id: "setup-obs-report", class: "obs-setup-report", "aria-live": "polite", hidden: true }),
    ];
  }

  function preview() {
    const frame = h(
      "div",
      { id: "camera-frame", class: "camera-frame", "data-simulated": "false", hidden: true },
      h("img", { id: "camera-image", alt: "What Sceneuron's camera analysis sees now" }),
      h("div", { id: "camera-faces", class: "camera-faces", "aria-hidden": "true" }),
    );
    return [
      h("h3", {}, "Camera preview"),
      h(
        "figure",
        { class: "camera-preview" },
        frame,
        h("p", { id: "camera-status", class: "camera-status", "aria-live": "polite" }, "The preview starts when this " +
          "tab is open."),
        h("figcaption", { id: "camera-caption", class: "hint" }),
      ),
      // Rewritten several times a second, so it is no live region.
      h("p", { id: "camera-seen", class: "hint", hidden: true }),
      // Rewritten up to a few times a second, so it is no live region either.
      h("p", { id: "camera-live-description" }, "Live description: none while no camera is analyzed."),
      h(
        "p",
        { class: "hint" },
        "The live description is what {{seen}} shows in overlay text, made on this computer from every analyzed " +
          "frame: free, and nothing is sent. Add live description, under Elements, puts it on the overlay.",
      ),
    ];
  }

  function cameraSources() {
    return [
      h("h3", {}, "Camera source"),
      h("p", { class: "hint" }, "Pick the OBS source whose video Sceneuron analyzes for presence."),
      h("ul", { id: "source-list", class: "source-list" }),
      h(
        "p",
        { id: "sources-empty", class: "hint", hidden: true },
        "No video sources found. Add a camera source in OBS, then refresh.",
      ),
      h("button", { id: "refresh-sources", type: "button" }, "Refresh sources"),
    ];
  }

  function microphones() {
    return [
      h("h3", {}, "Microphone"),
      h(
        "p",
        { class: "hint" },
        "Pick the OBS source Sceneuron listens to for voice control and subtitles. Global audio devices such as " +
          "Mic/Aux come from OBS's Settings > Audio.",
      ),
      h("p", { id: "mic-status", class: "hint" }),
      h("ul", { id: "mic-list", class: "source-list" }),
      h(
        "p",
        { id: "mics-empty", class: "hint", hidden: true },
        "No sources with audio found. Set a microphone in OBS's Settings > Audio, or add an Audio Input Capture " +
          "source, then refresh.",
      ),
      h("button", { id: "refresh-microphones", type: "button" }, "Refresh microphones"),
    ];
  }

  function overlay() {
    const url = h("input", { id: "overlay-url", type: "text", readonly: true });
    const copy = h("button", { id: "copy-overlay-url", type: "button" }, "Copy");
    return [
      h("h3", {}, "Overlay"),
      h("button", { id: "add-overlay", type: "button" }, "Add overlay to current scene"),
      field("overlay-url", "Overlay URL for a browser source", h("div", { class: "inline-field" }, url, copy)),
    ];
  }

  /** Fills the Camera & mic tab's panel. */
  function buildCameraPanel(panel) {
    panel.replaceChildren(...setUpObs(), ...preview(), ...cameraSources(), ...microphones(), ...overlay());
  }

  Editor.cameraFields = { buildCameraPanel };
})();
