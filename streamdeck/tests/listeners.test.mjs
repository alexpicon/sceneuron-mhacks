// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the listener set the link tells of its changes: every listener is called in order,
// one that throws is reported without stopping the rest, and a stopped listener is not called.

import assert from "node:assert/strict";
import { test } from "node:test";

import { Listeners } from "../src/lib/listeners.ts";

test("every listener hears each change, even after one throws", () => {
  const errors = [];
  const listeners = new Listeners((error) => errors.push(error.message));
  const heard = [];
  listeners.add(() => heard.push("first"));
  listeners.add(() => {
    throw new Error("broken");
  });
  listeners.add(() => heard.push("third"));
  listeners.notify();
  listeners.notify();
  assert.deepEqual(heard, ["first", "third", "first", "third"]);
  assert.deepEqual(errors, ["broken", "broken"]);
});

test("a stopped listener hears nothing more", () => {
  const listeners = new Listeners(() => assert.fail("no listener throws"));
  let heard = 0;
  const stop = listeners.add(() => (heard += 1));
  listeners.notify();
  stop();
  listeners.notify();
  assert.equal(heard, 1);
});
