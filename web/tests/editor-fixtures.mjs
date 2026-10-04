// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Shared fixtures for the editor tests: a small project like the default one, and live snapshot elements.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");

/** A two-element project with a rule for each presence state. */
export function project() {
  return {
    version: 1,
    canvas: { width: 1920, height: 1080 },
    elements: [
      { id: "brb", name: "Be right back", kind: "text", props: { ...R.DEFAULT_PROPS, visible: false, text: "BRB" } },
      { id: "cam-frame", name: "Camera frame", kind: "frame", props: { ...R.DEFAULT_PROPS, border_width: 8 } },
    ],
    rules: [
      {
        id: "away-show-brb",
        name: "Show BRB when I leave",
        enabled: true,
        when: "away",
        actions: [{ type: "set_props", element: "brb", props: { visible: true } }],
      },
      {
        id: "present-hide-brb",
        name: "Hide BRB when I return",
        enabled: true,
        when: "present",
        actions: [
          { type: "set_props", element: "brb", props: { visible: false } },
          { type: "effect", element: "cam-frame", effect: "flash" },
        ],
      },
    ],
    detection: { min_confidence: 0.7, present_after_ms: 500, away_after_ms: 3000, stale_after_ms: 2000, sample_fps: 5 },
    automation_enabled: true,
  };
}

/** A live snapshot element with optional held, rule, and effect layers. */
export function liveElement(id, kind, props, layers = {}) {
  return {
    id,
    name: id,
    kind,
    props: { ...R.DEFAULT_PROPS, ...props },
    overridden: layers.overridden ?? [],
    automated: layers.automated ?? [],
    effect: layers.effect ?? null,
  };
}
