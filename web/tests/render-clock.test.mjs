// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-clock.js and the time on air in render() on a fake DOM: the time's format, counting from the
// snapshot's start on the server's clock plus the time the page has seen pass, 00:00:00 off air, and drawing again
// when the shown second changes, only while a shown element counts and the page has a window to time it.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");
const C = require("../render-clock.js");

const TIMER = { ...element("on-air-time", "text", { text: "On air {{on-air-time}}" }), clock: true };

/** A root in a page whose window runs timers by hand; `now` is the page's clock. */
function stage() {
  const doc = createDocument({ constructable: true });
  const timers = new Map();
  let next = 1;
  doc.defaultView.setTimeout = (callback, ms) => {
    timers.set(next, { callback, ms });
    return next++;
  };
  doc.defaultView.clearTimeout = (id) => timers.delete(id);
  const root = doc.body.appendChild(doc.createElement("div"));
  const clock = { now: 50_000 };
  const options = { now: () => clock.now };
  const text = () => root.children[0].children[0].children[0].children[0].children[0].children[0].textContent;
  const fire = () => {
    const [[id, timer]] = timers;
    timers.delete(id);
    timer.callback();
  };
  return { root, timers, clock, options, text, fire };
}

const snapshot = (since, nowMs, elements = [TIMER]) => ({
  revision: 1,
  now_ms: nowMs,
  theme: "graphite",
  on_air: { scene: "Scene 01", since_ms: since },
  elements,
});

describe("formatTime and fillTime", () => {
  test("show hours, minutes, and seconds, rounding down", () => {
    assert.equal(C.formatTime(0), "00:00:00");
    assert.equal(C.formatTime(2_538_999), "00:42:18");
    assert.equal(C.formatTime(100 * 3_600_000 + 61_000), "100:01:01");
    for (const odd of [-5, NaN, undefined]) {
      assert.equal(C.formatTime(odd), "00:00:00", String(odd));
    }
    assert.equal(C.fillTime("{{on-air-time}} and {{on-air-time}}", 61_000), "00:01:01 and 00:01:01");
    assert.equal(C.fillTime("Live {{on-air-time}}", null), "Live 00:00:00");
  });
});

describe("the time on air in render", () => {
  test("counts from the start on the server's clock plus the time the page saw pass, every second", () => {
    const { root, timers, clock, options, text, fire } = stage();
    R.render(root, snapshot(1_000, 2_539_500), options);
    assert.equal(text(), "On air 00:42:18");
    assert.deepEqual([...timers.values()].map((timer) => timer.ms), [500], "until the next second shows");
    clock.now += 500;
    fire();
    assert.equal(text(), "On air 00:42:19");
    assert.deepEqual([...timers.values()].map((timer) => timer.ms), [1000]);
    R.render(root, snapshot(1_000, 2_600_000), options);
    assert.equal(text(), "On air 00:43:19", "a newer snapshot counts from its own time");
    assert.equal(timers.size, 1, "one timer, the newest");
  });

  test("off air it shows 00:00:00 and stops counting", () => {
    const { root, timers, options, text } = stage();
    R.render(root, snapshot(1_000, 5_000), options);
    assert.equal(timers.size, 1);
    R.render(root, snapshot(null, 6_000), options);
    assert.equal(text(), "On air 00:00:00");
    assert.equal(timers.size, 0);
  });

  test("counts only while a counting element shows, and never where there is no window to time it", () => {
    const { root, timers, options } = stage();
    const hidden = { ...TIMER, props: { ...TIMER.props, visible: false } };
    R.render(root, snapshot(1_000, 5_000, [hidden]), options);
    assert.equal(timers.size, 0, "a hidden timer");
    R.render(root, snapshot(1_000, 5_000, [element("label", "text", { text: "{{on-air-time}}" })]), options);
    assert.equal(timers.size, 0, "text the server did not mark as the time on air");
    const doc = createDocument();
    const bare = doc.body.appendChild(doc.createElement("div"));
    R.render(bare, snapshot(1_000, 5_000), options);
    const body = bare.children[0].children[0].children[0].children[0].children[0];
    assert.equal(body.children[0].textContent, "On air 00:00:04");
  });
});
