// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises flood.js: the planned emoji rain and bursts of each preset that cover the canvas and end within its
// length, their keyframes, and the lifecycle in a fake DOM: a new flood starts a layer above every element that
// covers the canvas, the first snapshot and stale or repeated floods never play, a newer flood replaces the playing
// one, and the layer removes itself when the flood ends.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { CANVAS, layers, snapshot, stage } from "./flood-fixtures.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const F = require("../flood.js");
const R = require("../render.js");
const { PRESETS } = require("../reaction-presets.js");
const TONGUE = PRESETS[0].emojis;

function tongue(nonce, atMs) {
  return { flood: "tongue", nonce, at_ms: atMs };
}

describe("planFlood", () => {
  const plan = F.planFlood("tongue", CANVAS, 7);

  test("rains in every column of the canvas and bursts across its middle", () => {
    const falls = plan.filter((item) => item.motion === "fall");
    const bursts = plan.filter((item) => item.motion === "burst");
    assert.equal(falls.length, 64);
    assert.equal(bursts.length, 30);
    const columns = new Set(falls.map((item) => Math.floor(((item.x + item.size / 2) / CANVAS.width) * 16)));
    assert.deepEqual([...columns].sort((a, b) => a - b), [...Array(16).keys()]);
    for (const item of falls) {
      assert.ok(item.y + item.size <= 0, "rain starts above the canvas");
      assert.ok(item.y + item.dy >= CANVAS.height, "rain falls past the bottom");
    }
    const centers = bursts.map((item) => (item.x + item.size / 2) / CANVAS.width);
    assert.ok(Math.min(...centers) < 0.35 && Math.max(...centers) > 0.65, "bursts spread across the canvas");
  });

  test("shows only tongue-out faces and ends every emoji within the flood's time", () => {
    assert.deepEqual(TONGUE, ["😜", "😛", "🤪", "😝"]);
    for (const item of plan) {
      assert.ok(TONGUE.includes(item.emoji), item.emoji);
      assert.ok(item.delay >= 0 && item.duration > 0, JSON.stringify(item));
      assert.ok(item.delay + item.duration <= F.durationOf("tongue"), JSON.stringify(item));
    }
    assert.equal(new Set(plan.map((item) => item.emoji)).size, 4, "every face shows");
  });

  test("every preset shows its own emojis, stretched over its own length", () => {
    for (const preset of PRESETS) {
      const items = F.planFlood(preset.id, CANVAS, 7);
      assert.equal(items.length, 94, preset.id);
      assert.ok(items.every((item) => preset.emojis.includes(item.emoji)), preset.id);
      assert.equal(new Set(items.map((item) => item.emoji)).size, preset.emojis.length, `all of ${preset.id}`);
      const ends = items.map((item) => item.delay + item.duration);
      assert.ok(Math.max(...ends) <= preset.duration_ms, `${preset.id} ends by ${preset.duration_ms}`);
      assert.ok(Math.max(...ends) > preset.duration_ms * 0.85, `${preset.id} fills its length`);
    }
    const email = F.planFlood("email", CANVAS, 7);
    assert.deepEqual(
      email.map((item) => [item.x, item.size]),
      plan.map((item) => [item.x, item.size]),
      "one nonce lays out every preset alike",
    );
    assert.equal(F.durationOf("email"), 4000);
  });

  test("one flood always looks the same, another differs, and smaller canvases get smaller emojis", () => {
    assert.deepEqual(F.planFlood("tongue", CANVAS, 7), plan);
    assert.notDeepEqual(F.planFlood("tongue", CANVAS, 8), plan);
    const small = F.planFlood("tongue", { width: 1280, height: 720 }, 7);
    assert.ok(small[0].size < plan[0].size);
    assert.equal(F.planFlood("unknown", CANVAS, 7)[0].emoji, plan[0].emoji, "unknown floods show tongues");
    assert.equal(F.presetOf("unknown").id, "tongue");
  });
});

describe("keyframes", () => {
  test("rain moves down and spins; a burst starts small and hidden and ends large and faded", () => {
    const [fall] = F.planFlood("tongue", CANVAS, 3);
    const frames = F.keyframes(fall);
    assert.equal(frames.length, 2);
    assert.match(frames[1].transform, /^translate\(-?[\d.]+px, [\d.]+px\) rotate\(-?[\d.]+deg\)$/);
    const burst = F.planFlood("tongue", CANVAS, 3).find((item) => item.motion === "burst");
    const [start, , end] = F.keyframes(burst);
    assert.deepEqual([start.opacity, end.opacity], [0, 0]);
    assert.match(start.transform, /scale\(0\.2\)/);
    assert.match(end.transform, /scale\(1\.4\)/);
  });
});

describe("isFresh", () => {
  test("a flood plays for its preset's length after it began", () => {
    assert.equal(F.durationOf("tongue"), 3000);
    assert.ok(F.isFresh(tongue(1, 1000), 1000));
    assert.ok(F.isFresh(tongue(1, 1000), 3999));
    assert.ok(!F.isFresh(tongue(1, 1000), 4000));
    assert.ok(F.isFresh({ flood: "email", nonce: 1, at_ms: 1000 }, 4999));
    assert.ok(!F.isFresh({ flood: "email", nonce: 1, at_ms: 1000 }, 5000));
    assert.ok(!F.isFresh(null, 1000));
    assert.ok(!F.isFresh({ flood: "tongue", nonce: 1 }, 1000));
  });
});

describe("update", () => {
  test("a new flood covers the canvas above every element, animates each emoji, and removes itself", () => {
    const { root, animations, timers, options, fire } = stage();
    const label = element("label", "text", { text: "Hello" });
    R.render(root, snapshot(null, 0, [label]));
    assert.equal(F.update(root, snapshot(null, 0), options), false, "the first snapshot only sets the baseline");
    assert.equal(F.update(root, snapshot(tongue(5, 2000), 2010), options), true);
    const [layer] = layers(root);
    assert.equal(root.children.at(-1), layer, "the flood is drawn after every element");
    const style = (name) => layer.style.getPropertyValue(name);
    assert.deepEqual(
      ["position", "left", "top", "width", "height", "pointer-events", "z-index"].map(style),
      ["absolute", "0px", "0px", "1920px", "1080px", "none", "2147483647"],
    );
    assert.equal(layer.getAttribute("aria-hidden"), "true");
    assert.equal(layer.children.length, 94);
    assert.ok(layer.children.every((child) => TONGUE.includes(child.textContent)));
    assert.equal(layer.getAttribute("data-flood"), "tongue");
    assert.equal(animations.length, 94);
    assert.ok(animations.every(({ timing }) => timing.fill === "both" && timing.delay + timing.duration <= 3000));
    R.render(root, snapshot(null, 2100, [label, element("frame", "frame", {})]));
    assert.equal(root.children.at(-1), layer, "a render keeps the flood above a new element");
    assert.deepEqual([...timers.values()].map((timer) => timer.ms), [3000]);
    fire();
    assert.deepEqual(layers(root), [], "the flood removed itself");
    assert.equal(root.children.length, 2, "the elements stay");
  });

  test("the first snapshot, a repeated nonce, and a stale flood never play", () => {
    const { root, options } = stage();
    assert.equal(F.update(root, snapshot(tongue(5, 2000), 2000), options), false, "a page loading mid-flood");
    assert.equal(F.update(root, snapshot(tongue(5, 2000), 2100), options), false, "the same flood again");
    assert.equal(F.update(root, snapshot(tongue(6, 2000), 9000), options), false, "a flood that already ended");
    assert.equal(F.update(root, snapshot(tongue(6, 2000), 2500), options), false, "seen once, never again");
    assert.equal(F.update(root, snapshot(null, 9100), options), false);
    assert.deepEqual(layers(root), []);
  });

  test("a newer flood replaces the one still playing", () => {
    const { root, timers, options, fire } = stage();
    F.update(root, snapshot(null, 0), options);
    F.update(root, snapshot(tongue(1, 1000), 1000), options);
    const first = layers(root)[0];
    F.update(root, snapshot(tongue(2, 1500), 1500), options);
    assert.equal(layers(root).length, 1);
    assert.notEqual(layers(root)[0], first);
    assert.equal(timers.size, 1, "the first flood's timer was cancelled");
    fire();
    assert.deepEqual(layers(root), []);
  });

  test("each root keeps its own floods", () => {
    const one = stage();
    const two = stage();
    F.update(one.root, snapshot(null, 0), one.options);
    F.update(one.root, snapshot(tongue(1, 100), 100), one.options);
    assert.equal(layers(one.root).length, 1);
    assert.equal(F.update(two.root, snapshot(tongue(1, 100), 100), two.options), false);
    assert.deepEqual(layers(two.root), []);
  });
});
