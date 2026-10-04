// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how render-fit.js sizes text that shows live subtitles under a theme: the lines the subtitle settings
// show, the box's full lines as the core wraps subtitles for it, and fitText against a fake layout, where one line
// and two get the same size, words wider than the box's lines shrink alone, and new words cost one layout read;
// and for subtitle bars, the room a label leaves, Latin lines under the words, and the words put back structured.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { fakeState, look, shown } from "./fit-fixtures.mjs";

const require = createRequire(import.meta.url);
const F = require("../render-fit.js");

const ONE = "Good morning everyone.";
const TWO = "Good morning everyone, and welcome to the stream.\nToday we are planning the next sprint together.";
const lineCount = (words) => words.split("\n").length;
const ownLines = (scale, words) => lineCount(words);

/** The subtitles pack's bar under Windows 95: 40 px text in a 1160 by 100 box that leaves 62 px for it. */
function bar(options = {}) {
  return fakeState({ content: ONE, fontSize: 40, width: 1160, height: 100, room: 62, linesAt: ownLines, ...options });
}

describe("subtitleLines", () => {
  test("follows the subtitle settings, from 1 to 3 lines, and is 2 without them", () => {
    for (const snapshot of [undefined, null, {}, { subtitles: {} }, { subtitles: { status: {} } }]) {
      assert.equal(F.subtitleLines(snapshot), 2, JSON.stringify(snapshot));
    }
    const lines = (value) => F.subtitleLines({ subtitles: { status: { lines: value } } });
    assert.deepEqual([1, 2, 3].map(lines), [1, 2, 3]);
    assert.deepEqual([0, 7, 2.5, "3"].map(lines), [1, 3, 2, 2]);
  });
});

describe("subtitleSample", () => {
  const shape = (sample) => sample.split("\n").map((line) => line.length);

  test("fills the lines the core wraps subtitles to: as many as fit and are asked for, each a full line", () => {
    // The core fits (1160 - 0.8 * 40) / (0.55 * 40) = 51 characters on a line and 100 / (1.15 * 40) = 2 lines.
    assert.deepEqual(shape(F.subtitleSample(1160, 100, 40, 2)), [51, 51]);
    assert.deepEqual(shape(F.subtitleSample(1160, 100, 40, 1)), [51]);
    assert.deepEqual(shape(F.subtitleSample(1160, 100, 40, 3)), [51, 51], "the box holds only two");
    assert.deepEqual(shape(F.subtitleSample(1160, 138, 40, 3)), [51, 51, 51], "exactly three lines tall");
    assert.deepEqual(shape(F.subtitleSample(1160, 30, 40, 2)), [51], "a box shorter than a line still shows one");
  });

  test("keeps lines from 8 to 84 characters, none ending in a space", () => {
    assert.deepEqual(shape(F.subtitleSample(100, 100, 40, 2)), [8, 8]);
    assert.deepEqual(shape(F.subtitleSample(4000, 400, 40, 3)), [84, 84, 84]);
    for (let width = 100; width <= 2100; width += 11) {
      for (const line of F.subtitleSample(width, 50, 40, 1).split("\n")) {
        assert.ok(!line.endsWith(" ") && !line.startsWith(" "), `${width}: ${JSON.stringify(line)}`);
      }
    }
  });
});

describe("fitText with live subtitles", () => {
  test("one line and two get the size the box's two lines need", () => {
    const { state } = bar();
    F.fitText(state, look("windows-95"), true);
    const size = shown(state);
    // Two lines of 40 px need 92 px; the theme leaves 62, and whole pixels allow half a pixel more.
    const limit = 62.5 / 92;
    assert.ok(parseFloat(size) <= limit && parseFloat(size) > limit - 0.01, size);
    for (const words of [TWO, "Good", ONE]) {
      state.content = state.text.textContent = words;
      F.fitText(state, look("windows-95"), true);
      assert.equal(shown(state), size, words);
    }
    const own = bar().state;
    F.fitText(own, look("windows-95"), false);
    assert.equal(shown(own), null, "a line that is not live keeps its own size");
  });

  test("measures the box's lines in place, then puts the words back", () => {
    const measured = [];
    const { state } = bar({ linesAt: (scale, words) => measured.push(words) && lineCount(words) });
    F.fitText(state, look("windows-95"), true);
    assert.deepEqual([...new Set(measured)], [F.subtitleSample(1160, 100, 40, 2), ONE]);
    assert.equal(state.text.textContent, ONE);
  });

  test("words wider than the box's lines shrink alone, and the next words get the box's size back", () => {
    const wide = "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW";
    const linesAt = (scale, words) => lineCount(words) + (words === wide && scale > 0.5 ? 1 : 0);
    const { state } = bar({ linesAt });
    F.fitText(state, look("windows-95"), true);
    const size = shown(state);
    state.content = state.text.textContent = wide;
    F.fitText(state, look("windows-95"), true);
    assert.ok(parseFloat(shown(state)) <= 0.5 && parseFloat(shown(state)) > 0.49, shown(state));
    state.content = state.text.textContent = TWO;
    F.fitText(state, look("windows-95"), true);
    assert.equal(shown(state), size);
  });

  test("new words cost one layout read, and the box is measured again only when it changes", () => {
    const { state, reads } = bar();
    F.fitText(state, look("windows-95"), true);
    const measured = reads.count;
    F.fitText(state, look("windows-95"), true);
    assert.equal(reads.count, measured, "the same words read nothing");
    state.content = state.text.textContent = TWO;
    F.fitText(state, look("windows-95"), true);
    assert.equal(reads.count, measured + 1);
    // With one line a language, the core shows the last line only.
    state.content = state.text.textContent = ONE;
    F.fitText(state, { ...look("windows-95"), lines: 1 }, true);
    assert.ok(reads.count > measured + 2, "one line a language measures again");
    assert.equal(shown(state), null, "one line fits the room at its own size");
  });

  test("hidden between sentences and shown again, the text gets the same size", () => {
    const { state } = bar();
    F.fitText(state, look("windows-95"), true);
    const size = shown(state);
    state.applied = { ...state.applied, display: "none" };
    F.fitText(state, look("windows-95"), true);
    assert.equal(shown(state), null);
    state.applied = { ...state.applied, display: "block" };
    state.content = state.text.textContent = "Today";
    F.fitText(state, look("windows-95"), true);
    assert.equal(shown(state), size);
  });

  test("without a theme, live text keeps its own size", () => {
    const { state, reads } = bar({ content: TWO });
    F.fitText(state, look("none"), true);
    assert.equal(shown(state), null);
    assert.equal(reads.count, 0);
  });
});

describe("subtitle bars", () => {
  test("a label's room narrows the sample's lines and Latin lines under them leave fewer", () => {
    const shape = (sample) => sample.split("\n").map((line) => line.length);
    // Beside a label that takes 5.21 em, (1160 - 208.4 - 32) / 22 = 41 characters fit; a line and its Latin line
    // take 1.65 lines, so 100 px holds one pair.
    assert.deepEqual(shape(F.subtitleSample(1160 - 5.21 * 40, 100, 40, 2)), [41, 41]);
    assert.deepEqual(shape(F.subtitleSample(1160, 100, 40, 3, 1.65)), [51]);
    assert.deepEqual(shape(F.subtitleSample(1160, 180, 40, 3, 1.65)), [51, 51]);
  });

  test("fitText measures the room a label leaves and puts structured words back after the sample", () => {
    const measured = [];
    const { state } = bar({ linesAt: (scale, words) => measured.push(words) && lineCount(words) });
    state.subtitleFit = { roomEm: 5.21, lineSize: 1 };
    let refills = 0;
    state.refill = () => {
      refills += 1;
      state.text.textContent = ONE;
    };
    F.fitText(state, look("windows-95"), true);
    assert.equal(measured[0], F.subtitleSample(1160 - 5.21 * 40, 100, 40, 2));
    assert.equal(refills, 1, "the words are put back the way render-subtitles.js wrote them");
    assert.equal(state.text.textContent, ONE);
  });

  test("words with Latin lines under them may take the lines render-subtitles.js counted", () => {
    // Two lines, each with a Latin line 0.65 of one under it, take 3.3 lines of the box.
    const glossed = "Elen\nsíla";
    const linesAt = (scale, words) => (words === glossed ? 3.3 : lineCount(words));
    const fit = (lineUnits) => {
      const { state } = bar({ content: glossed, room: 200, linesAt });
      Object.assign(state, { subtitleFit: { roomEm: 0, lineSize: 1.65 }, lineUnits });
      F.fitText(state, look("windows-95"), true);
      return shown(state);
    };
    assert.equal(fit(3.3), null, "they fit at their own size, Latin lines and all");
    assert.notEqual(fit(null), null, "counted as two lines, they would seem to wrap");
  });
});
