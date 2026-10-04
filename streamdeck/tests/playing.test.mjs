// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how long a reaction counts as playing: from when a view naming it arrives for as long as
// that view says, a later view of the same reaction only shortening it, a new reaction starting
// over, and a view without one, or no view, ending it at once.

import assert from "node:assert/strict";
import { test } from "node:test";

import { PlayingReaction } from "../src/lib/playing.ts";
import { sampleView } from "./fake-engine.mjs";

const playing = (flood, nonce, left_ms) => ({ ...sampleView(), flood: { flood, nonce, left_ms } });

/** A tracker on mocked timers, counting the times a reaction ended by itself. */
function tracker(t) {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const ends = { count: 0 };
  return { reaction: new PlayingReaction(() => (ends.count += 1)), ends };
}

test("a reaction plays for as long as the view said, then ends and says so", (t) => {
  const { reaction, ends } = tracker(t);
  assert.equal(reaction.flood, null);
  assert.equal(reaction.follow(playing("email", 1, 4000)), true);
  assert.equal(reaction.flood, "email");
  t.mock.timers.tick(3999);
  assert.deepEqual([reaction.flood, ends.count], ["email", 0]);
  t.mock.timers.tick(1);
  assert.deepEqual([reaction.flood, ends.count], [null, 1]);
});

test("a later view of the same reaction only shortens it, and a new one starts over", (t) => {
  const { reaction, ends } = tracker(t);
  reaction.follow(playing("email", 1, 4000));
  t.mock.timers.tick(1000);
  assert.equal(reaction.follow(playing("email", 1, 4000)), false, "the same reaction, no longer");
  t.mock.timers.tick(2999);
  assert.equal(reaction.flood, "email");
  t.mock.timers.tick(1);
  assert.equal(reaction.flood, null);
  assert.equal(reaction.follow(playing("email", 1, 4000)), false, "an old view does not bring it back");
  assert.equal(reaction.follow(playing("ship-it", 2, 500)), true);
  reaction.follow(playing("ship-it", 2, 100));
  t.mock.timers.tick(100);
  assert.deepEqual([reaction.flood, ends.count], [null, 2]);
});

test("a view without a reaction, or none at all, ends it at once", (t) => {
  const { reaction, ends } = tracker(t);
  reaction.follow(playing("coffee", 3, 4000));
  assert.equal(reaction.follow(sampleView()), true);
  assert.equal(reaction.flood, null);
  reaction.follow(playing("coffee", 4, 4000));
  assert.equal(reaction.follow(null), true);
  t.mock.timers.tick(5000);
  assert.equal(ends.count, 0, "an ended reaction does not end again");
  assert.equal(reaction.follow(playing("tongue", 5, 0)), false, "a reaction that already ended");
});
