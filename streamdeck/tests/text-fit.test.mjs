// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the text fitting behind key drawings: width estimates for emoji and narrow letters,
// wrapping without split words, and the ellipsis when nothing fits.

import assert from "node:assert/strict";
import { test } from "node:test";

import { LINE_HEIGHT, ellipsize, fitText, graphemeWidth, measure } from "../src/lib/text-fit.ts";

test("emoji sequences and flags measure as one wide glyph", () => {
  assert.equal(measure("⏱️"), graphemeWidth("⏱️"));
  assert.equal(measure("👍🏽"), 1.25);
  assert.equal(measure("🇺🇸"), 1.25);
  assert.ok(measure("il") < measure("MW"));
});

const KEY = { width: 116, height: 112, maxSize: 52, minSize: 13, maxLines: 4 };

test("a short card fills the key at the largest size", () => {
  assert.deepEqual(fitText("5", KEY), { size: 52, lines: ["5"] });
});

test("longer text wraps between words and shrinks until every line fits", () => {
  const { size, lines } = fitText("Let's take this offline", KEY);
  assert.ok(size < 52 && size >= 13);
  assert.ok(lines.length > 1 && lines.length <= 4);
  assert.equal(lines.join(" "), "Let's take this offline");
  for (const line of lines) {
    assert.ok(measure(line) * size <= KEY.width, line);
  }
  assert.ok(lines.length * size * LINE_HEIGHT <= KEY.height);
});

test("words are not split while a smaller size keeps them whole", () => {
  const { lines } = fitText("Disagree", KEY);
  assert.deepEqual(lines, ["Disagree"]);
});

test("text too long for the smallest size ends with an ellipsis", () => {
  const long = Array.from({ length: 60 }, (_, index) => `word${index}`).join(" ");
  const { size, lines } = fitText(long, KEY);
  assert.equal(size, 13);
  assert.equal(lines.length, 4);
  assert.ok(lines.at(-1).endsWith("…"));
  for (const line of lines) {
    assert.ok(measure(line) * size <= KEY.width, line);
  }
});

test("blank text draws no lines", () => {
  assert.deepEqual(fitText("   ", KEY).lines, []);
});

test("ellipsize keeps a line that fits and cuts one that does not", () => {
  assert.equal(ellipsize("Agree", 10), "Agree");
  const cut = ellipsize("Planning poker deck", 5);
  assert.ok(cut.endsWith("…"));
  assert.ok(measure(cut) <= 5);
});
