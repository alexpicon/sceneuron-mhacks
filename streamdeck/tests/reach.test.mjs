// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the reach the controls show for the link's state: live, unpaired, refused, and offline
// at once, a connecting link that keeps the reach shown before it until it has been connecting for
// 1.5 s, so a revoke goes from live to "Pair again" without "Offline" on the way, and a connecting
// link that shows offline rather than unpaired, as at the plugin's start and after pairing.

import assert from "node:assert/strict";
import { test } from "node:test";

import { ShownReach } from "../src/lib/reach.ts";

/** A reach that follows the link through `states` on mocked timers, counting its lapses. */
function follow(t, states) {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const lapses = { count: 0 };
  const shown = new ShownReach(states[0], () => (lapses.count += 1));
  for (const state of states.slice(1)) {
    shown.follow(state);
  }
  return { shown, lapses };
}

test("connected shows live, and unpaired, refused, and offline show at once", (t) => {
  const { shown } = follow(t, ["unpaired"]);
  assert.equal(shown.reach, "unpaired");
  for (const [state, reach] of [["connected", "live"], ["refused", "refused"], ["offline", "offline"]]) {
    shown.follow(state);
    assert.equal(shown.reach, reach, state);
  }
});

test("a revoke keeps the live faces while the link proves the server, then shows refused at once", (t) => {
  const { shown, lapses } = follow(t, ["unpaired", "connecting", "connected", "connecting"]);
  assert.equal(shown.reach, "live");
  t.mock.timers.tick(1_499);
  assert.equal(shown.reach, "live");
  shown.follow("refused");
  assert.equal(shown.reach, "refused");
  t.mock.timers.tick(1_500);
  assert.deepEqual([shown.reach, lapses.count], ["refused", 0], "the grace ended with the connecting");
});

test("a link still connecting after 1.5 s shows offline and says so once", (t) => {
  const { shown, lapses } = follow(t, ["connected", "connecting"]);
  t.mock.timers.tick(1_000);
  shown.follow("connecting");
  assert.equal(shown.reach, "live", "a view change while connecting does not restart the grace");
  t.mock.timers.tick(499);
  assert.deepEqual([shown.reach, lapses.count], ["live", 0]);
  t.mock.timers.tick(1);
  assert.deepEqual([shown.reach, lapses.count], ["offline", 1]);
  t.mock.timers.tick(10_000);
  assert.equal(lapses.count, 1);
  shown.follow("connected");
  assert.equal(shown.reach, "live");
});

test("a link that reaches offline shows it at once, and connecting from there keeps it", (t) => {
  const { shown, lapses } = follow(t, ["connected", "connecting", "offline"]);
  assert.equal(shown.reach, "offline");
  shown.follow("connecting");
  assert.equal(shown.reach, "offline");
  assert.equal(lapses.count, 0);
});

test("at the plugin's start a paired link shows offline while it connects, never unpaired", (t) => {
  const { shown, lapses } = follow(t, ["unpaired", "connecting"]);
  assert.equal(shown.reach, "offline");
  t.mock.timers.tick(1_500);
  assert.deepEqual([shown.reach, lapses.count], ["offline", 1]);
  shown.follow("connected");
  assert.equal(shown.reach, "live");
});

test("after pairing the faces show offline rather than unpaired while the link connects", (t) => {
  const { shown } = follow(t, ["connected", "unpaired", "connecting"]);
  assert.equal(shown.reach, "offline");
  shown.follow("connected");
  assert.equal(shown.reach, "live");
});

test("a link that starts out connecting has nothing to keep and waits as offline", (t) => {
  const { shown, lapses } = follow(t, ["connecting"]);
  assert.equal(shown.reach, "offline");
  t.mock.timers.tick(1_500);
  assert.deepEqual([shown.reach, lapses.count], ["offline", 1]);
});
