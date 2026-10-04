// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the banner some presets show (flood-banner.js) and its life on the overlay (flood.js in a fake DOM): its
// text between the preset's first and last emoji, centered and at most 80% of the canvas wide, sized to the canvas,
// in the preset's color, popping in and fading out over the preset's length; a banner flood draws its emojis and
// then the banner above them, above every element, and removes it all when the flood ends; and the tongue has none.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { CANVAS, layers, snapshot, stage } from "./flood-fixtures.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const B = require("../flood-banner.js");
const F = require("../flood.js");
const R = require("../render.js");
const { PRESETS } = require("../reaction-presets.js");

const preset = (id) => PRESETS.find((entry) => entry.id === id);
const email = (nonce, atMs) => ({ flood: "email", nonce, at_ms: atMs });

describe("planBanner", () => {
  test("the email's banner says it between its first and last emoji, centered and sized to the canvas", () => {
    const plan = B.planBanner(preset("email"), CANVAS);
    assert.equal(plan.text, "📧 This meeting could have been an email 🤦");
    assert.equal(plan.fontSize, 86.4);
    assert.ok(plan.width <= CANVAS.width * 0.8 && plan.width > CANVAS.width * 0.6, String(plan.width));
    assert.equal(plan.left, Math.round(((CANVAS.width - plan.width) / 2) * 10) / 10);
    assert.equal(plan.background, "#1d4ed8");
    assert.ok(plan.border >= 2 && plan.radius > 0 && plan.paddingX > plan.paddingY);
  });

  test("a short line makes a narrower banner, and a smaller canvas a smaller one", () => {
    const ship = B.planBanner(preset("ship-it"), CANVAS);
    assert.equal(ship.text, "🚀 Ship it! 🙌");
    assert.ok(ship.width < B.planBanner(preset("email"), CANVAS).width * 0.6, String(ship.width));
    const small = B.planBanner(preset("email"), { width: 1280, height: 720 });
    assert.equal(small.fontSize, 57.6);
    assert.ok(small.width <= 1280 * 0.8);
  });

  test("every meeting preset has a banner, and the tongue and a missing preset have none", () => {
    for (const id of ["email", "mind-blown", "ship-it", "coffee"]) {
      assert.ok(B.planBanner(preset(id), CANVAS).text.includes(preset(id).banner), id);
    }
    assert.equal(B.planBanner(preset("tongue"), CANVAS), null);
    assert.equal(B.planBanner(null, CANVAS), null);
    assert.equal(B.planBanner({ banner: "Hi", emojis: [], color: "red" }, CANVAS).background, "#1d4ed8");
  });
});

describe("bannerKeyframes", () => {
  test("the banner pops in from small and hidden, holds, and shrinks away fading", () => {
    const frames = B.bannerKeyframes();
    assert.deepEqual([frames[0].opacity, frames.at(-1).opacity], [0, 0]);
    assert.match(frames[0].transform, /^translateY\(-50%\) scale\(0\.2\)/);
    const held = frames.filter((frame) => frame.opacity === 1).map((frame) => frame.offset);
    assert.deepEqual(held, [0.12, 0.2, 0.85]);
    assert.ok(frames.every((frame) => frame.transform.startsWith("translateY(-50%)")), "kept centered");
    assert.equal(frames[0].easing, "cubic-bezier(.2,.8,.3,1)", "a quick pop in");
  });

  test("the banner's animation runs linearly, so it holds until 85% of the flood", () => {
    const { root, animations, options } = stage();
    F.update(root, snapshot(null, 0), options);
    F.update(root, snapshot(email(2, 1000), 1000), options);
    const banner = layers(root)[0].children.at(-1);
    const timing = animations.find((entry) => entry.node === banner).timing;
    assert.equal(timing.easing, undefined, "no easing over the whole animation, which would move the offsets");
  });
});

describe("a banner flood on the overlay", () => {
  test("draws the emojis, then the banner above them and every element, and removes it all after 4 s", () => {
    const { root, animations, timers, options, fire } = stage();
    R.render(root, snapshot(null, 0, [element("label", "text", { text: "Hello" })]));
    F.update(root, snapshot(null, 0), options);
    assert.equal(F.update(root, snapshot(email(9, 1000), 1020), options), true);
    const [layer] = layers(root);
    assert.equal(root.children.at(-1), layer, "the flood is drawn after every element");
    assert.equal(layer.getAttribute("data-flood"), "email");
    assert.equal(layer.children.length, 95, "94 emojis and the banner");
    const banner = layer.children.at(-1);
    assert.ok(banner.classList.contains("sn-flood-banner"), "the banner is drawn last, above the emojis");
    assert.equal(banner.textContent, "📧 This meeting could have been an email 🤦");
    const emojis = preset("email").emojis;
    assert.ok(layer.children.slice(0, -1).every((child) => emojis.includes(child.textContent)));
    const style = (name) => banner.style.getPropertyValue(name);
    assert.deepEqual(
      ["position", "top", "font-size", "color", "text-align"].map(style),
      ["absolute", "50%", "86.4px", "var(--sn-banner-text, #ffffff)", "center"],
    );
    assert.equal(style("background"), "var(--sn-banner-panel, #1d4ed8)", "the preset's color unless a theme sets one");
    assert.equal(style("border"), "6px solid var(--sn-banner-edge, #ffffff)");
    const timing = animations.find((entry) => entry.node === banner).timing;
    assert.deepEqual([timing.duration, timing.fill], [4000, "both"]);
    assert.ok(animations.every((entry) => (entry.timing.delay ?? 0) + entry.timing.duration <= 4000));
    assert.deepEqual([...timers.values()].map((timer) => timer.ms), [4000]);
    fire();
    assert.deepEqual(layers(root), [], "the flood and its banner removed themselves");
    assert.equal(root.children.length, 1, "the element stays");
  });

  test("a banner flood that ended before the page learned of it never plays", () => {
    const { root, options } = stage();
    F.update(root, snapshot(null, 0), options);
    assert.equal(F.update(root, snapshot(email(3, 1000), 5000), options), false);
    assert.equal(F.update(root, snapshot(email(4, 1000), 4900), options), true, "still within its 4 s");
  });
});
