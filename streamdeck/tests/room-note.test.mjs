// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how long the room keys say "No room" after a press found none: 4 s, started over by
// another such press, and ended sooner by a press that finds a room or a change of the link's
// state, but not by a view change that leaves the state as it was.

import assert from "node:assert/strict";
import { test } from "node:test";

import { NoRoomNote } from "../src/lib/room-note.ts";

/** A note on mocked timers for a link in `state`, counting the times it ended by itself. */
function noteFor(t, state = "connected") {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const lapses = { count: 0 };
  const note = new NoRoomNote(state, () => (lapses.count += 1));
  return { note, lapses };
}

test("a press that found no room is noted for 4 s, then the note ends and says so", (t) => {
  const { note, lapses } = noteFor(t);
  assert.equal(note.active, false);
  assert.equal(note.note(false), true, "the note changed");
  t.mock.timers.tick(3_999);
  assert.deepEqual([note.active, lapses.count], [true, 0]);
  t.mock.timers.tick(1);
  assert.deepEqual([note.active, lapses.count], [false, 1]);
  t.mock.timers.tick(10_000);
  assert.equal(lapses.count, 1);
});

test("another press that found no room starts the 4 s over", (t) => {
  const { note, lapses } = noteFor(t);
  note.note(false);
  t.mock.timers.tick(3_000);
  assert.equal(note.note(false), false, "still noted, so nothing to draw again");
  t.mock.timers.tick(3_999);
  assert.equal(note.active, true);
  t.mock.timers.tick(1);
  assert.deepEqual([note.active, lapses.count], [false, 1]);
});

test("a press that found a room ends the note at once", (t) => {
  const { note, lapses } = noteFor(t);
  assert.equal(note.note(true), false, "nothing was noted");
  note.note(false);
  assert.equal(note.note(true), true);
  assert.equal(note.active, false);
  t.mock.timers.tick(4_000);
  assert.equal(lapses.count, 0, "an ended note does not end again");
});

test("a change of the link's state ends the note, and a view change does not", (t) => {
  const { note, lapses } = noteFor(t);
  note.note(false);
  note.follow("connected");
  assert.equal(note.active, true, "the same state, as after a view change");
  note.follow("connecting");
  assert.equal(note.active, false);
  note.note(false);
  note.follow("connected");
  assert.equal(note.active, false);
  t.mock.timers.tick(4_000);
  assert.equal(lapses.count, 0);
});
