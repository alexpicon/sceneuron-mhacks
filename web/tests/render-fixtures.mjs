// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Shared fixtures for the renderer tests: a "be right back" card and snapshot elements.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");

export const BRB = {
  ...R.DEFAULT_PROPS,
  x: 560,
  y: 440,
  width: 800,
  height: 200,
  visible: false,
  text: "Be right back",
  font_size: 96,
  background: "#141428cc",
  border_color: "#9b5cffff",
  border_width: 6,
  corner_radius: 32,
  idle: "pulse",
  enter: "pop",
};

export function element(id, kind, props, effect = null) {
  return { id, name: id, kind, props: { ...R.DEFAULT_PROPS, ...props }, overridden: [], automated: [], effect };
}
