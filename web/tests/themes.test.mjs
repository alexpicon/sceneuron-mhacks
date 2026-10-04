// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the theme files under themes/: every text color a theme draws reads at 4.5 to 1 or better on every
// panel it sits on, as the packs' cards do in their own colors, its second text color included; every font is one
// macOS installs, with fallbacks; only window themes title elements; and the CSS loads nothing from anywhere and
// styles nothing outside its own theme.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");
const { THEMES } = require("../render-themes.js");

/** WCAG 2's minimum contrast ratio for text. */
const MIN_CONTRAST = 4.5;
// Families every theme may name: those installed with macOS, then common web fallbacks, then generics.
const MAC_FONTS = ["Menlo", "Monaco", "Andale Mono", "Courier New", "Tahoma", "Verdana", "Georgia", "Times New Roman"]
  .concat(["Silom", "Geneva", "Helvetica Neue", "Avenir Next", "Futura", "SF Mono", "-apple-system"]);
const FALLBACK_FONTS = ["Consolas", "Segoe UI", "Roboto", "Arial", "system-ui"];
const GENERIC_FONTS = ["monospace", "sans-serif", "serif"];
const themed = THEMES.filter((theme) => theme.id !== "none");

/** WCAG 2's relative luminance of an sRGB color; alpha is ignored. */
function luminance({ r, g, b }) {
  const linear = (channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

function contrast(one, other) {
  const [a, b] = [luminance(one), luminance(other)];
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** `layers`, from the bottom up, painted over an opaque `backdrop`. */
function composite(backdrop, layers) {
  return layers.reduce((under, top) => {
    const mix = (channel) => top[channel] * top.a + under[channel] * (1 - top.a);
    return { r: mix("r"), g: mix("g"), b: mix("b"), a: 1 };
  }, backdrop);
}

const BLACK = { r: 0, g: 0, b: 0, a: 1 };
const WHITE = { r: 255, g: 255, b: 255, a: 1 };

/** The font families of a CSS font-family value, unquoted. */
function families(stack) {
  return stack.split(",").map((family) => family.trim().replace(/^"(.*)"$/, "$1"));
}

/** The text color the rule for exactly `selector` in `css` sets, or null. */
function textColor(css, selector) {
  const start = css.indexOf(`${selector}{`);
  const rule = start < 0 ? "" : css.slice(start + selector.length + 1, css.indexOf("}", start));
  const match = /(?:^|;)color:([^;]+)/.exec(rule.replace(/\n/g, ""));
  return match && match[1];
}

/**
 * The selector lists of a stylesheet's top-level rules, with each @keyframes rule as its prelude, and each @media
 * rule as its prelude followed by the selector lists of the rules inside it.
 */
function preludes(css) {
  const found = [];
  let depth = 0;
  let start = 0;
  let media = false;
  for (let index = 0; index < css.length; index += 1) {
    if (css[index] === "{") {
      const prelude = css.slice(start, index).trim();
      if (depth === 0 || (depth === 1 && media)) {
        found.push(prelude);
      }
      if (depth === 0 && prelude.startsWith("@media")) {
        media = true;
        start = index + 1;
      }
      depth += 1;
    } else if (css[index] === "}") {
      depth -= 1;
      start = index + 1;
      media = media && depth > 0;
    }
  }
  assert.equal(depth, 0, "balanced braces");
  return found;
}

describe("contrast", () => {
  test("the measure matches known pairs", () => {
    const hex = (value) => R.parseColor(value);
    assert.ok(Math.abs(contrast(hex("#000000"), hex("#ffffff")) - 21) < 1e-9);
    assert.ok(Math.abs(contrast(hex("#2563eb"), hex("#2563eb")) - 1) < 1e-9);
    assert.ok(contrast(hex("#767676"), hex("#ffffff")) >= MIN_CONTRAST);
    assert.ok(contrast(hex("#777777"), hex("#ffffff")) < MIN_CONTRAST);
    const glass = composite(WHITE, [{ r: 0, g: 0, b: 0, a: 0.5 }]);
    assert.deepEqual(glass, { r: 127.5, g: 127.5, b: 127.5, a: 1 });
  });

  test("every theme's text reads on every panel it sits on, over any picture behind it", () => {
    let pairs = 0;
    for (const theme of themed) {
      assert.ok(theme.readable.length > 0, `${theme.id} lists its text colors`);
      for (const [textKey, panels] of theme.readable) {
        const text = R.parseColor(theme.colors[textKey]);
        assert.ok(text, `${theme.id}.${textKey} is a color`);
        assert.equal(text.a, 1, `${theme.id}.${textKey} is solid`);
        for (const panel of panels) {
          const layers = panel.map((key) => R.parseColor(theme.colors[key]));
          assert.ok(layers.every(Boolean), `${theme.id}: ${panel.join(" + ")} are colors`);
          for (const backdrop of [BLACK, WHITE]) {
            const ratio = contrast(text, composite(backdrop, layers));
            const behind = backdrop === BLACK ? "black" : "white";
            const where = `${theme.id}: ${textKey} on ${panel.join(" + ")} over ${behind}`;
            assert.ok(ratio >= MIN_CONTRAST, `${where} is ${ratio.toFixed(2)} to 1`);
          }
          pairs += 1;
        }
      }
    }
    assert.equal(pairs, 56, "every listed pair is checked");
  });

  test("text whose own color has a hue is drawn in a second color the theme lists as readable", () => {
    for (const theme of themed) {
      const own = textColor(theme.css, `.sn-root[data-theme="${theme.id}"] .sn-body`);
      const second = textColor(theme.css, `.sn-root[data-theme="${theme.id}"] .sn-alt-text .sn-body`);
      assert.ok(own && second && own !== second, `${theme.id}: ${own} and ${second}`);
      const readable = theme.readable.map(([key]) => theme.colors[key]);
      assert.ok(readable.includes(own) && readable.includes(second), `${theme.id} checks ${own} and ${second}`);
    }
  });

  test("every color a theme lists is drawn by its stylesheet", () => {
    for (const theme of themed) {
      for (const [key, value] of Object.entries(theme.colors)) {
        assert.ok(theme.css.includes(value), `${theme.id}.${key} ${value} is unused`);
      }
    }
  });
});

describe("fonts", () => {
  test("every font is installed with macOS or a fallback, and every stack ends in a generic family", () => {
    const allowed = new Set([...MAC_FONTS, ...FALLBACK_FONTS, ...GENERIC_FONTS]);
    for (const theme of THEMES) {
      assert.ok(theme.fonts.length > 0, theme.id);
      for (const stack of theme.fonts) {
        const names = families(stack);
        assert.ok(MAC_FONTS.includes(names[0]) || names[0] === "system-ui", `${theme.id} leads with ${names[0]}`);
        assert.ok(GENERIC_FONTS.includes(names.at(-1)), `${theme.id}: ${stack} ends in a generic family`);
        for (const name of names) {
          assert.ok(allowed.has(name), `${theme.id} names ${name}`);
        }
      }
    }
  });

  test("the stylesheets name fonts only through their font stacks", () => {
    for (const theme of themed) {
      assert.doesNotMatch(theme.css, /[{;\s]font:/, `${theme.id} uses the font shorthand`);
      const used = Array.from(theme.css.matchAll(/font-family:([^;}]+)/g), (match) => match[1].trim());
      assert.ok(used.length > 0, theme.id);
      for (const stack of used) {
        assert.ok(theme.fonts.includes(stack), `${theme.id} uses ${stack}`);
      }
    }
  });
});

describe("stylesheets", () => {
  test("load nothing, from this server or anywhere else", () => {
    for (const theme of themed) {
      for (const banned of [/url\(/i, /@import/i, /@font-face/i, /https?:/i, /expression\(/i]) {
        assert.doesNotMatch(theme.css, banned, theme.id);
      }
    }
  });

  test("style only their own theme's root, and name their keyframes after it", () => {
    for (const theme of themed) {
      const root = `.sn-root[data-theme="${theme.id}"]`;
      for (const prelude of preludes(theme.css)) {
        if (prelude.startsWith("@media")) {
          assert.equal(prelude, "@media (prefers-reduced-motion:reduce)", `${theme.id}: ${prelude}`);
          continue;
        }
        if (prelude.startsWith("@keyframes")) {
          assert.match(prelude, new RegExp(`^@keyframes sn-${theme.id}-[a-z-]+$`), prelude);
          continue;
        }
        for (const selector of prelude.split(",")) {
          assert.ok(selector.trim().startsWith(`${root} `), `${theme.id}: ${selector}`);
        }
      }
    }
  });

  test("only window themes show title bars, which carry each element's name on stream", () => {
    const titled = THEMES.filter((theme) => theme.titles).map((theme) => theme.id);
    assert.deepEqual(titled, ["windows-95", "classic-mac"]);
    for (const theme of THEMES) {
      assert.equal(typeof theme.titles, "boolean", theme.id);
      assert.equal(/ \.sn-title\{display:(flex|block)/.test(theme.css), theme.titles, theme.id);
    }
  });

  test("swatches name colors the theme has", () => {
    for (const theme of THEMES) {
      assert.ok(R.parseColor(theme.colors[theme.swatch.text]), `${theme.id} swatch text`);
      assert.ok(R.parseColor(theme.colors[theme.swatch.panel]), `${theme.id} swatch panel`);
    }
  });
});
