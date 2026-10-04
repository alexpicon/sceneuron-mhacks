// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises a session, one proven event stream: views admitted in revision order, replies that
// must be strictly newer, and ending, which aborts only that session's signal.

import assert from "node:assert/strict";
import { test } from "node:test";

import { Session } from "../src/lib/session.ts";

const view = (revision) => ({ revision, automation_enabled: true, decks: [], elements: [] });

test("views are admitted in revision order, and equal ones only when not strictly newer", () => {
  const session = new Session(new AbortController().signal);
  assert.equal(session.admit(view(0), true), true, "a new session takes any revision");
  assert.equal(session.admit(view(3), false), true);
  assert.equal(session.admit(view(3), false), true);
  assert.equal(session.admit(view(3), true), false);
  assert.equal(session.admit(view(2), false), false);
  assert.equal(session.admit(view(4), true), true);
  assert.equal(session.admit(view(3), false), false);
});

test("ending a session aborts its own signal, and stopping the run aborts every session's", () => {
  const run = new AbortController();
  const first = new Session(run.signal);
  const second = new Session(run.signal);
  first.end();
  assert.equal(first.ended, true);
  assert.equal(first.signal.aborted, true);
  assert.equal(second.signal.aborted, false);
  run.abort();
  assert.equal(second.signal.aborted, true);
  assert.equal(second.ended, false, "only end() marks a session as ended");
});
