// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the Graphite theme's floods (flood-burst.js and its use by flood.js and flood-banner.js in a fake DOM):
// the planned icon burst starts at the bottom center, lands inside the canvas, and ends within the preset's length,
// each preset throws its own icon in amber and off-white, reduced motion keeps the icons still, a Graphite root
// draws icons and an emoji-free pill banner in the upper third, and every other theme still rains emojis.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { CANVAS, layers, snapshot, stage } from "./flood-fixtures.mjs";

const require = createRequire(import.meta.url);
const B = require("../flood-banner.js");
const Burst = require("../flood-burst.js");
const F = require("../flood.js");
const I = require("../icons.js");
const { PRESETS } = require("../reaction-presets.js");

const PICTOGRAPH = /\p{Extended_Pictographic}/u;
const preset = (id) => PRESETS.find((entry) => entry.id === id);

describe("planBurst", () => {
  const plan = Burst.planBurst(["spark"], CANVAS, 7, 3000);

  test("throws 15 to 25 icons from the bottom center that land inside the canvas within the duration", () => {
    assert.ok(plan.length >= 15 && plan.length <= 25, String(plan.length));
    for (const item of plan) {
      const startX = item.x + item.size / 2;
      const startY = item.y + item.size / 2;
      assert.ok(Math.abs(startX - CANVAS.width / 2) <= CANVAS.width * 0.05, `starts centered: ${startX}`);
      assert.ok(startY >= CANVAS.height * 0.9 && item.y + item.size <= CANVAS.height, `starts at the bottom`);
      assert.ok(item.dy < 0, "flies up");
      const endX = item.x + item.dx;
      const endY = item.y + item.dy;
      assert.ok(endX >= 0 && endX + item.size <= CANVAS.width, `lands inside: ${endX}`);
      assert.ok(endY >= 0 && endY + item.size <= CANVAS.height, `lands inside: ${endY}`);
      assert.ok(item.size >= 32 && item.size <= 96, String(item.size));
      assert.ok(item.delay >= 0 && item.duration > 0 && item.delay + item.duration <= 3000, JSON.stringify(item));
      assert.ok(Burst.COLORS.includes(item.color));
    }
    const angles = plan.map((item) => (Math.atan2(-item.dy, item.dx) * 180) / Math.PI);
    assert.ok(Math.min(...angles) < 70 && Math.max(...angles) > 110, "fans out both ways");
    assert.equal(new Set(plan.map((item) => item.color)).size, 2, "amber and off-white");
  });

  test("one flood always looks the same, another differs, and a smaller canvas gets smaller icons", () => {
    assert.deepEqual(Burst.planBurst(["spark"], CANVAS, 7, 3000), plan);
    assert.notDeepEqual(Burst.planBurst(["spark"], CANVAS, 8, 3000), plan);
    const small = Burst.planBurst(["spark"], { width: 1280, height: 720 }, 7, 3000);
    assert.ok(small.every((item) => item.size <= 64));
  });

  test("each preset throws its own drawn icon over its own length; others throw the spark", () => {
    const expected = {
      email: "envelope",
      "mind-blown": "spark",
      "ship-it": "rocket",
      coffee: "mug",
      tongue: "reaction",
      "absolutely-right": "check-circle",
      "more-tokens": "coin",
      "works-on-my-gpu": "chip",
      meatproxy: "proxy",
      clanker: "robot",
    };
    for (const entry of PRESETS) {
      const items = Burst.planPreset(entry, CANVAS, 3);
      assert.ok(items.every((item) => item.icon === expected[entry.id]), entry.id);
      assert.ok(items.every((item) => item.delay + item.duration <= entry.duration_ms), entry.id);
      assert.ok(expected[entry.id] in I.ICONS);
    }
    assert.equal(Burst.iconOf("no-such-preset"), "spark");
    assert.equal(Burst.iconOf("toString"), "spark");
  });

  test("keyframes fly out and fade; reduced motion only fades in place", () => {
    const [item] = plan;
    const frames = Burst.burstKeyframes(item, false);
    assert.deepEqual([frames[0].opacity, frames.at(-1).opacity], [0, 0]);
    assert.match(frames[0].transform, /^translate\(0px, 0px\)/);
    assert.notEqual(frames[0].transform, frames.at(-1).transform);
    const still = Burst.burstKeyframes(item, true);
    assert.deepEqual([still[0].opacity, still.at(-1).opacity], [0, 0]);
    assert.equal(new Set(still.map((frame) => frame.transform)).size, 1, "never moves");
  });
});

describe("graphite floods", () => {
  function play(theme, flood, reducedMotion = false) {
    const scene = stage();
    if (theme) {
      scene.root.setAttribute("data-theme", theme);
    }
    const options = { ...scene.options, reducedMotion };
    F.update(scene.root, snapshot(null, 0), options);
    assert.equal(F.update(scene.root, snapshot({ flood, nonce: 4, at_ms: 100 }, 100), options), true);
    return { ...scene, layer: layers(scene.root)[0] };
  }

  test("a Graphite root throws line icons, not emojis, and removes them when the flood ends", () => {
    const { layer, animations, fire, root } = play("graphite", "ship-it");
    const icons = layer.children.filter((child) => child.classList.contains("sn-flood-icon"));
    assert.ok(icons.length >= 15 && icons.length <= 25);
    assert.ok(icons.every((node) => node.getAttribute("data-icon") === "rocket" && node.children.length > 0));
    assert.ok(layer.children.every((node) => !PICTOGRAPH.test(node.textContent || "")), "no emoji anywhere");
    assert.ok(animations.every(({ timing }) => (timing.delay || 0) + timing.duration <= F.durationOf("ship-it")));
    assert.ok(animations.some(({ frames }) => new Set(frames.map((frame) => frame.transform)).size > 1));
    fire();
    assert.deepEqual(layers(root), []);
  });

  test("with reduced motion the icons fade where they are", () => {
    const { animations } = play("graphite", "tongue", true);
    const icons = animations.filter(({ node }) => node.classList.contains("sn-flood-icon"));
    assert.ok(icons.length > 0);
    assert.ok(icons.every(({ frames }) => new Set(frames.map((frame) => frame.transform)).size === 1));
  });

  test("the banner is an emoji-free pill with an amber edge in the upper third", () => {
    const plan = B.planBanner(preset("email"), CANVAS, "graphite");
    assert.equal(plan.text, "This meeting could have been an email");
    assert.ok(plan.top + plan.fontSize * 2.6 / 2 + plan.paddingY < CANVAS.height / 3, "two lines stay above a third");
    assert.ok(plan.top - plan.fontSize * 2.6 / 2 - plan.paddingY > 0);
    assert.ok(plan.border >= 1 && plan.border <= 2);
    assert.equal(plan.edge, "#d6a76c");
    const { layer } = play("graphite", "email");
    const banner = layer.children.find((child) => child.classList.contains("sn-flood-banner"));
    assert.equal(banner.textContent, "This meeting could have been an email");
    assert.equal(banner.style.getPropertyValue("top"), `${plan.top}px`);
    assert.equal(banner.style.getPropertyValue("color"), "#f2f0ed");
    assert.match(banner.style.getPropertyValue("border"), /solid #d6a76c$/);
  });

  test("off-white text on the graphite pill reads at better than 4.5 to 1", () => {
    const luminance = (hex) => {
      const channels = [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16) / 255);
      const linear = channels.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
      return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    };
    for (const panel of ["#1c1c1e", "#2a2a2c"]) {
      const ratio = (luminance("#f2f0ed") + 0.05) / (luminance(panel) + 0.05);
      assert.ok(ratio >= 4.5, `${panel}: ${ratio}`);
    }
  });

  test("other themes still rain the preset's emojis with the emoji banner in the middle", () => {
    for (const theme of [null, "none", "vaporwave"]) {
      const { layer } = play(theme, "email");
      const emojis = layer.children.filter((child) => !child.classList.contains("sn-flood-banner"));
      assert.equal(emojis.length, 94, String(theme));
      assert.ok(emojis.every((node) => preset("email").emojis.includes(node.textContent)));
      const banner = layer.children.find((child) => child.classList.contains("sn-flood-banner"));
      assert.equal(banner.style.getPropertyValue("top"), "50%");
      assert.ok(PICTOGRAPH.test(banner.textContent));
    }
  });
});

test("every preset throws its own Graphite icon from the line icon set", () => {
  for (const preset of PRESETS) {
    const icon = Burst.ICON_BY_PRESET[preset.id];
    assert.ok(icon, `${preset.id} has no Graphite icon`);
    assert.ok(I.ICONS[icon] && I.ICONS[icon].length > 0, `${preset.id} names the unknown icon ${icon}`);
  }
});
