// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the reconnection waits: doubling from the base, the ceiling, the random reduction of
// up to half, and sleeps that end early when the connection is reconfigured.

import assert from "node:assert/strict";
import { test } from "node:test";

import { RECONNECT, backoffDelay, sleep } from "../src/lib/backoff.ts";

test("waits double from half a second up to thirty seconds", () => {
  const full = () => 0;
  assert.deepEqual(
    [0, 1, 2, 3, 4, 5, 6, 7, 100].map((attempt) => backoffDelay(attempt, full)),
    [500, 1000, 2000, 4000, 8000, 16000, 30000, 30000, 30000],
  );
  assert.equal(RECONNECT.maxMs, 30_000);
});

test("randomness takes off at most half", () => {
  assert.equal(backoffDelay(3, () => 0.999999), 2000);
  assert.equal(backoffDelay(3, () => 0.5), 3000);
  assert.equal(backoffDelay(-4, () => 0), 500);
  assert.equal(backoffDelay(1, () => 0, { baseMs: 10, maxMs: 15 }), 15);
});

test("a sleep ends early when its signal aborts", async () => {
  const controller = new AbortController();
  const started = Date.now();
  const done = sleep(60_000, controller.signal);
  setTimeout(() => controller.abort(), 10);
  await done;
  assert.ok(Date.now() - started < 5_000);
  await sleep(60_000, controller.signal);
});
