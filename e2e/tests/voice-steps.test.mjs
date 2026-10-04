// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the spoken-card browser step breaks each card's time into phases from the overlay's
// notes: raise to its audio being ready, to play(), and to playing, and "never" for a missing phase.

import assert from "node:assert/strict";
import { test } from "node:test";

import { phases } from "../voice-steps.mjs";

test("each raised card's phases come from the next notes after it", () => {
  const events = [
    "100 raise agree", "130 ready 1", "131 play() 1234", "140 loadstart", "200 canplay", "350 playing",
    "400 raise disagree", "420 ready 2", "425 play() abcd", "430 canplay", "7425 playing",
  ];
  assert.deepEqual(phases(events), [
    "agree: ready 30 ms, play() 1 ms, canplay 69 ms, playing 219 ms",
    "disagree: ready 20 ms, play() 5 ms, canplay 5 ms, playing 7000 ms",
  ]);
});

test("a phase that never happened reads never", () => {
  assert.deepEqual(phases(["10 raise agree", "40 ready 1"]), ["agree: ready 30 ms, play() never, canplay never, playing never"]);
  assert.deepEqual(phases(["10 raise agree"]), ["agree: ready never, play() never, canplay never, playing never"]);
});
