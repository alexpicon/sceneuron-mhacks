// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises push to talk shared by every talk key. Against a fake Sceneuron: held while any key
// holds it, requests in the order pressed however fast, a hold that cannot go out that lets the
// key go, a device's keys let go together, and a let-go owed while unpaired that goes out on the
// next connection. Against a stub link: a failed hold that Sceneuron may have taken is let go, an
// owed let-go that fails while the link stays connected is tried again a few times, and a held key
// sends the hold again before Sceneuron's own minute runs out.

import assert from "node:assert/strict";
import { test } from "node:test";

import { ApiError } from "../src/lib/api.ts";
import { NotConnectedError } from "../src/lib/link.ts";
import { PushToTalk } from "../src/lib/talk.ts";
import { CONTROL_TOKEN } from "./fake-sceneuron.mjs";
import { startLink, startSceneuron, waitFor } from "./harness.mjs";

const HELD = { down: true };
const LET_GO = { down: false };
/** Waits short enough for a test: a hold sent again every 40 ms, and let-go tries 10, 20, 40 ms apart. */
const TIMES = { refreshMs: 40, retryMs: [10, 20, 40] };
const NOTHING_ANSWERED = () => new ApiError(0, "nothing answered");

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * A stand-in for the link that stays in its `state` until a test changes it, records each body
 * posted, and answers each with `answer(body)`: a promise, so a test decides when and how each
 * request ends. Like the link, it sends nothing while not connected.
 */
function stubLink(answer = async () => undefined) {
  const listeners = new Set();
  const link = {
    state: "connected",
    posted: [],
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async send(path, body) {
      assert.equal(path, "/api/voice/talk");
      if (link.state !== "connected") {
        throw new NotConnectedError();
      }
      link.posted.push(body);
      return answer(body);
    },
    /** Moves to `state` and tells the listeners, as the link does. */
    become(state) {
      link.state = state;
      for (const listener of listeners) {
        listener();
      }
    },
  };
  return link;
}

/** A connected link to a fake Sceneuron, push to talk on it, and a count of its changes. */
async function connected(t) {
  const server = await startSceneuron(t);
  const link = startLink(t);
  link.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => link.state === "connected", "the connection");
  const changes = { count: 0 };
  const talk = new PushToTalk(link, () => (changes.count += 1));
  return { server, link, talk, changes };
}

test("push to talk is let go only once no key holds it", async (t) => {
  const { server, talk, changes } = await connected(t);
  await talk.hold("key-1", "deck");
  await talk.hold("key-2", "deck");
  await talk.release("key-1");
  assert.equal(talk.holding, true);
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD, HELD]);
  await talk.release("key-2");
  await talk.release("key-2");
  assert.equal(talk.holding, false);
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD, HELD, LET_GO]);
  assert.equal(changes.count, 4, "each hold and each let-go of a held key");
});

test("a quick press and release go out in the order they were made", async (t) => {
  const { server, talk } = await connected(t);
  await Promise.all([talk.hold("key-1", "deck"), talk.release("key-1"), talk.hold("key-1", "deck")]);
  await talk.release("key-1");
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD, LET_GO, HELD, LET_GO]);
  assert.equal(server.talking, false);
});

test("a hold that cannot go out lets the key go and rejects", async (t) => {
  const link = startLink(t);
  const talk = new PushToTalk(link, () => undefined);
  await assert.rejects(talk.hold("key-1", "deck"), NotConnectedError);
  assert.equal(talk.holding, false);
  await talk.release("key-1");
});

test("unplugging a device lets go of its keys only", async (t) => {
  const { server, talk } = await connected(t);
  await talk.hold("key-1", "deck");
  await talk.hold("key-2", "other deck");
  await talk.releaseDevice("deck");
  assert.equal(talk.holding, true, "the other device's key still holds");
  await talk.releaseDevice("other deck");
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD, HELD, LET_GO]);
});

test("a let-go that could not go out is sent once the link connects again", async (t) => {
  const { server, link, talk } = await connected(t);
  await talk.hold("key-1", "deck");
  link.configure(server.port, null);
  await assert.rejects(talk.release("key-1"), NotConnectedError);
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD]);
  link.configure(server.port, CONTROL_TOKEN);
  await waitFor(() => server.posted("/api/voice/talk").length === 2, "the owed let-go");
  assert.deepEqual(server.posted("/api/voice/talk"), [HELD, LET_GO]);
  assert.equal(server.talking, false);
});

test("a hold that fails after Sceneuron may have taken it is let go, and its release sends nothing more", async () => {
  const link = stubLink(async (body) => {
    if (body.down) {
      throw NOTHING_ANSWERED();
    }
  });
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await assert.rejects(talk.hold("key-1", "deck"), { status: 0 });
  assert.equal(talk.holding, false);
  await waitFor(() => link.posted.length === 2, "the let-go");
  await talk.release("key-1");
  await pause(50);
  assert.deepEqual(link.posted, [HELD, LET_GO]);
});

test("a key whose hold fails after another key let go lets go of the hold that one made", async () => {
  const second = Promise.withResolvers();
  const answers = [Promise.resolve(), second.promise];
  const link = stubLink(async () => answers.shift());
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await talk.hold("key-1", "deck");
  const holding = talk.hold("key-2", "deck");
  await talk.release("key-1");
  assert.deepEqual(link.posted, [HELD, HELD], "key 2 still holds, so no let-go yet");
  second.reject(NOTHING_ANSWERED());
  await assert.rejects(holding, { status: 0 });
  await waitFor(() => link.posted.length === 3, "the let-go");
  assert.deepEqual(link.posted, [HELD, HELD, LET_GO]);
  assert.equal(talk.holding, false);
});

test("a let-go owed after a hold that could not go out is sent once the link connects again", async () => {
  const link = stubLink();
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await talk.hold("key-1", "deck");
  const second = talk.hold("key-2", "deck");
  link.state = "offline";
  await talk.release("key-1");
  await assert.rejects(second, NotConnectedError);
  assert.deepEqual(link.posted, [HELD], "the let-go of key 1's hold could not go out");
  link.become("connecting");
  link.become("connected");
  await waitFor(() => link.posted.length === 2, "the owed let-go");
  assert.deepEqual(link.posted, [HELD, LET_GO]);
});

test("a hold that Sceneuron refused, or that never went out, needs no let-go", async () => {
  const link = stubLink(async () => {
    throw new ApiError(403, "not allowed");
  });
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await assert.rejects(talk.hold("key-1", "deck"), { status: 403 });
  link.become("offline");
  await assert.rejects(talk.hold("key-1", "deck"), NotConnectedError);
  link.become("connected");
  await pause(80);
  assert.deepEqual(link.posted, [HELD]);
});

test("a let-go that fails while the link stays connected is tried again until it goes out", async () => {
  let failures = 1;
  const link = stubLink(async (body) => {
    if (!body.down && failures > 0) {
      failures -= 1;
      throw new ApiError(500, "server error");
    }
  });
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await talk.hold("key-1", "deck");
  await assert.rejects(talk.release("key-1"), { status: 500 });
  await waitFor(() => link.posted.length === 3, "the let-go tried again");
  link.become("connected");
  await pause(80);
  assert.deepEqual(link.posted, [HELD, LET_GO, LET_GO], "nothing more once it went out");
});

test("an owed let-go is tried again after each wait, then left to Sceneuron's own minute", async () => {
  const link = stubLink(async (body) => {
    if (!body.down) {
      throw new ApiError(500, "server error");
    }
  });
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await talk.hold("key-1", "deck");
  await assert.rejects(talk.release("key-1"), { status: 500 });
  await waitFor(() => link.posted.length === 5, "three more tries");
  await pause(120);
  assert.deepEqual(link.posted, [HELD, LET_GO, LET_GO, LET_GO, LET_GO]);
  link.become("offline");
  link.become("connected");
  await waitFor(() => link.posted.length === 6, "the let-go sent again on connecting");
});

test("a hold that goes out while a let-go waits to be tried again cancels the try", async () => {
  let failures = 1;
  const link = stubLink(async (body) => {
    if (!body.down && failures > 0) {
      failures -= 1;
      throw new ApiError(500, "server error");
    }
  });
  const talk = new PushToTalk(link, () => undefined, { refreshMs: 60_000, retryMs: [60] });
  await talk.hold("key-1", "deck");
  await assert.rejects(talk.release("key-1"), { status: 500 });
  await talk.hold("key-1", "deck");
  await pause(100);
  assert.deepEqual(link.posted, [HELD, LET_GO, HELD], "no let-go while the key is held again");
  await talk.release("key-1");
  assert.deepEqual(link.posted, [HELD, LET_GO, HELD, LET_GO]);
});

test("a held key sends the hold again before Sceneuron lets go by itself, and stops once let go", async () => {
  const link = stubLink();
  const talk = new PushToTalk(link, () => undefined, TIMES);
  await talk.hold("key-1", "deck");
  await waitFor(() => link.posted.length >= 3, "the hold sent again twice");
  assert.ok(link.posted.every((body) => body.down), "only holds while the key is held");
  await talk.release("key-1");
  const sent = link.posted.length;
  await pause(120);
  assert.equal(link.posted.length, sent, "nothing after the let-go");
  assert.deepEqual(link.posted.at(-1), LET_GO);
});
