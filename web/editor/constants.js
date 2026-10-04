// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: limits and defaults shared by the editor scripts. The page loads it right after the
   renderer, and it creates the window.SceneuronEditor namespace; each later editor script adds its entry. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Render = inNode ? require("../render-style.js") : window.SceneuronRender;
  if (!Render || (!inNode && typeof Render.render !== "function")) {
    throw new Error("The Sceneuron editor needs render-style.js and render.js, loaded before the editor scripts.");
  }

  // Limits mirror the validation in sceneuron-core so problems show before saving.
  const constants = {
    MAX_ELEMENTS: 64,
    MAX_CONDITIONS: 16,
    MAX_RULES: 64,
    MAX_ACTIONS: 16,
    MAX_DECKS: 16,
    MAX_ID_CHARS: 40,
    MAX_NAME_CHARS: 80,
    MAX_TEXT_CHARS: 1000,
    MAX_PROMPT_CHARS: 300,
    MAX_DETAIL_CHARS: 120,
    MAX_REQUEST_CHARS: 2000,
    // The placeholders for the scene caption, the live description, the shared room's votes, live subtitles, OBS's
    // scene and whether it is live, and the time on air, so no condition may use them as ids.
    SCENE_PLACEHOLDER: "scene",
    SEEN_PLACEHOLDER: "seen",
    ROOM_PLACEHOLDER: "room",
    SUBTITLES_PLACEHOLDER: "subtitles",
    ON_AIR_PLACEHOLDER: "on-air",
    ON_AIR_TIME_PLACEHOLDER: "on-air-time",
    CONDITION_DEFAULTS: { enabled: true, min_confidence: 0.6, confirm: 1, release: 2 },
    KINDS: ["text", "box", "frame"],
    KIND_NAMES: { text: "Text", box: "Box", frame: "Frame" },
    EFFECTS: ["shake", "pop", "flash", "bounce"],
    PRESENCE_STATES: ["unknown", "present", "away"],
    PROP_FIELDS: Object.keys(Render.DEFAULT_PROPS),
    NUMBER_LIMITS: {
      x: [-10000, 10000],
      y: [-10000, 10000],
      width: [1, 10000],
      height: [1, 10000],
      font_size: [1, 1000],
      border_width: [0, 200],
      corner_radius: [0, 5000],
      opacity: [0, 1],
    },
    KIND_DEFAULTS: {
      text: { width: 640, height: 140, text: "New text", font_size: 64, background: "#141428cc", corner_radius: 16 },
      box: { width: 400, height: 240, background: "#9b5cffcc", corner_radius: 24 },
      frame: { width: 640, height: 360, border_color: "#9b5cffff", border_width: 8, corner_radius: 24 },
    },
  };

  if (inNode) {
    module.exports = constants;
  } else {
    window.SceneuronEditor = { constants };
  }
})();
