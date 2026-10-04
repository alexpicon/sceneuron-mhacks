// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-fit.js: finding the largest text scale that fits by bisection, counting the lines a text takes
// in its own look, and fitText against a fake layout: text that fits keeps its size, text too tall for its theme's
// room or wrapped by the theme alone shrinks, text that wraps in its own look may wrap, and measuring waits for
// layout and happens only when the layout, the canvas's decoration unit included, can have changed. Live
// subtitles are in render-fit-subtitles.test.mjs.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { fakeState, look, shown } from "./fit-fixtures.mjs";

const require = createRequire(import.meta.url);
const F = require("../render-fit.js");

const SUBTITLES = "Good morning everyone, and welcome to the stream.\nToday we are planning the next sprint together.";

describe("fitScale", () => {
  test("keeps full size when it fits, after one try", () => {
    const tried = [];
    assert.equal(F.fitScale((scale) => tried.push(scale) > 0), 1);
    assert.deepEqual(tried, [1]);
  });

  test("finds the largest scale that fits, within a hundredth, in eight tries", () => {
    const tried = [];
    const scale = F.fitScale((candidate) => tried.push(candidate) && candidate <= 0.7);
    assert.ok(scale <= 0.7 && scale > 0.69, String(scale));
    assert.equal(tried.length, 8);
  });

  test("never goes below the smallest scale", () => {
    assert.equal(F.fitScale(() => false), 0.4);
  });
});

describe("ownLines", () => {
  test("counts the lines of text whose every line fits on one", () => {
    assert.equal(F.ownLines("Wrap it up", 1160, 72), 1);
    assert.equal(F.ownLines(SUBTITLES, 1160, 40), 2);
    assert.equal(F.ownLines("", 300, 130), 1);
    assert.equal(F.ownLines("8", 300, 130), 1);
  });

  test("is null when a line wraps in the element's own look", () => {
    assert.equal(F.ownLines("Let's take this offline", 300, 130), null);
    assert.equal(F.ownLines(`short\n${"x".repeat(52)}`, 1160, 40), null);
  });
});

describe("fitText", () => {
  test("text that fits keeps its size", () => {
    const { state } = fakeState();
    F.fitText(state, look("hacker"));
    assert.equal(shown(state), null);
  });

  test("text too tall for the room its theme leaves shrinks to the largest size that fits", () => {
    // Two lines of 40 px need 92 px; the theme leaves 62.
    const { state } = fakeState({ content: SUBTITLES, fontSize: 40, room: 62, linesAt: () => 2 });
    F.fitText(state, look("windows-95"));
    // Measurements are whole pixels, so half a pixel more still fits.
    const scale = parseFloat(shown(state));
    const limit = 62.5 / 92;
    assert.ok(scale <= limit && scale > limit - 0.01, shown(state));
  });

  test("a line the theme alone wraps shrinks until it is one line again, though two would fit the room", () => {
    const linesAt = (scale) => (scale > 0.55 ? 2 : 1);
    const { state } = fakeState({ content: "8", fontSize: 130, width: 300, room: 400, linesAt });
    F.fitText(state, look("hacker"));
    const scale = parseFloat(shown(state));
    assert.ok(scale <= 0.55 && scale > 0.54, shown(state));
  });

  test("a note under the words, as a vote card's count, takes a line of its own and does not shrink them", () => {
    const voted = { content: "✓ Voted", fontSize: 28, width: 441, height: 130, room: 100 };
    const twoLines = () => 1.6;
    const { state } = fakeState({ ...voted, note: "Voted: 3 of 5", linesAt: twoLines });
    F.fitText(state, look("graphite"));
    assert.equal(shown(state), null);
    const { state: bare } = fakeState({ ...voted, linesAt: twoLines });
    F.fitText(bare, look("graphite"));
    assert.ok(Number.parseFloat(shown(bare)) < 1, "without a note the same height is a wrapped line");
  });

  test("text that wraps in its own look may wrap in the theme too", () => {
    const content = "Let's take this offline";
    const { state } = fakeState({ content, fontSize: 130, width: 300, room: 400, linesAt: () => 2 });
    F.fitText(state, look("hacker"));
    assert.equal(shown(state), null);
  });

  test("measures again only when the theme, text, or size changed", () => {
    const { state, reads } = fakeState({ room: 60 });
    F.fitText(state, look("hacker"));
    const measured = reads.count;
    assert.ok(measured > 1);
    F.fitText(state, look("hacker"));
    assert.equal(reads.count, measured);
    state.content = state.text.textContent = "Wrap it up now";
    F.fitText(state, look("hacker"));
    assert.ok(reads.count > measured);
  });

  test("measures again when the canvas changes the size of the theme's decorations", () => {
    // The room the decorations leave follows the unit: 60 px at 1080 lines, 66 px at 720.
    const { state, reads } = fakeState({ room: 60, linesAt: () => 1 });
    F.fitText(state, look("windows-95", "1px"));
    const at1080 = parseFloat(shown(state));
    const measured = reads.count;
    state.body.clientHeight = 66 + 20;
    F.fitText(state, look("windows-95", "1px"));
    assert.equal(reads.count, measured, "the same unit keeps the fit");
    F.fitText(state, look("windows-95", "0.6667px"));
    assert.ok(reads.count > measured);
    assert.ok(parseFloat(shown(state)) > at1080, `${shown(state)} after ${at1080}em`);
  });

  test("waits for layout before measuring", () => {
    const { state, reads } = fakeState({ room: 60 });
    state.body.laidOut = false;
    F.fitText(state, look("hacker"));
    assert.equal(reads.count, 0);
    assert.equal(shown(state), null);
    state.body.laidOut = true;
    F.fitText(state, look("hacker"));
    assert.ok(reads.count > 0 && shown(state) !== null);
  });

  test("without a theme, hidden, or with no layout at all, text keeps its own size", () => {
    const { state, reads } = fakeState({ room: 60 });
    F.fitText(state, look("hacker"));
    assert.notEqual(shown(state), null);
    F.fitText(state, look("none"));
    assert.equal(shown(state), null);
    F.fitText(state, look("hacker"));
    state.applied = { ...state.applied, display: "none" };
    F.fitText(state, look("hacker"));
    assert.equal(shown(state), null);
    const box = fakeState({ room: 60 }).state;
    box.kind = "box";
    F.fitText(box, look("hacker"));
    assert.equal(shown(box), null);
    const node = fakeState({ room: 60 }).state;
    node.body.ownerDocument = { defaultView: null };
    const before = reads.count;
    F.fitText(node, look("hacker"));
    assert.equal(shown(node), null);
    assert.equal(reads.count, before);
  });
});
