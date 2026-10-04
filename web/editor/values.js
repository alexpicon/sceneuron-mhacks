// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: parsing and validating what users type into numbers, names, texts, and colors. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Render = inNode ? require("../render-style.js") : window.SceneuronRender;
  const { MAX_NAME_CHARS, MAX_TEXT_CHARS } = inNode ? require("./constants.js") : window.SceneuronEditor.constants;

  function parseNumber(text, min, max, integer) {
    const trimmed = String(text === undefined || text === null ? "" : text).trim();
    if (trimmed === "") {
      return null;
    }
    const value = Number(trimmed);
    if (!Number.isFinite(value) || value < min || value > max || (integer && !Number.isInteger(value))) {
      return null;
    }
    return value;
  }

  /** Matches Rust's `char::is_control`: U+0000-U+001F and U+007F-U+009F. */
  function isControl(char) {
    const code = char.codePointAt(0);
    return code < 0x20 || (code >= 0x7f && code <= 0x9f);
  }

  function isValidName(name) {
    const chars = Array.from(String(name));
    return chars.length >= 1 && chars.length <= MAX_NAME_CHARS && !chars.some(isControl);
  }

  function isValidText(text) {
    const chars = Array.from(String(text));
    return chars.length <= MAX_TEXT_CHARS && !chars.some((char) => char !== "\n" && isControl(char));
  }

  /** Splits a hex color into the `#rrggbb` a color input accepts and a 0-1 alpha. */
  function splitColor(value) {
    const color = Render.parseColor(value);
    if (!color) {
      return { rgb: "#ffffff", alpha: 1 };
    }
    const hex = (channel) => channel.toString(16).padStart(2, "0");
    return { rgb: `#${hex(color.r)}${hex(color.g)}${hex(color.b)}`, alpha: color.a };
  }

  /** True when two hex colors are the same color, such as `#9b5cff` and `#9b5cffff`. */
  function sameColor(a, b) {
    const first = Render.parseColor(a);
    const second = Render.parseColor(b);
    const key = (color) => [color.r, color.g, color.b, Math.round(color.a * 255)].join();
    return Boolean(first && second) && key(first) === key(second);
  }

  function joinColor(rgb, alpha) {
    const base = /^#[0-9a-f]{6}$/i.test(rgb) ? rgb.toLowerCase() : "#ffffff";
    const byte = Math.round(Math.min(1, Math.max(0, Number(alpha) || 0)) * 255);
    return byte === 255 ? base : base + byte.toString(16).padStart(2, "0");
  }

  const values = { isControl, isValidName, isValidText, joinColor, parseNumber, sameColor, splitColor };
  if (inNode) {
    module.exports = values;
  } else {
    window.SceneuronEditor.values = values;
  }
})();
