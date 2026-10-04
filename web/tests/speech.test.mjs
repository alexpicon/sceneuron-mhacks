// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises speech.js, the overlay's spoken cards: each cue plays once after its audio is ready, a cue already
// spoken when the page connected or one whose audio came too late stays silent, a lowered or covered card stops,
// a restarted server's cues play again while a reconnect to the same server loses none, and the audio address
// carries the page's token.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const S = require("../speech.js");

const AUDIO = "0123456789abcdef".repeat(4);

function snapshot(cue, raised = "agree", nowMs = 1000) {
  return { now_ms: nowMs, speech: cue, decks: [{ id: "meeting", raised }] };
}

function cue(nonce, audio = AUDIO, atMs = 900) {
  return { text: "Agree", nonce, deck: "meeting", card: "agree", at_ms: atMs, audio };
}

describe("nextCue", () => {
  test("a cue plays once, after its audio is ready", () => {
    let state = S.nextCue({ last: null }, snapshot(null)).state;
    assert.deepEqual(state, { last: 0 });
    let next = S.nextCue(state, snapshot(cue(5, null)));
    assert.equal(next.play, null, "no audio yet");
    next = S.nextCue(next.state, snapshot(cue(5)));
    assert.equal(next.play.nonce, 5);
    state = next.state;
    assert.equal(S.nextCue(state, snapshot(cue(5))).play, null, "never twice");
    assert.equal(S.nextCue(state, snapshot(cue(6))).play.nonce, 6);
  });

  test("a cue that was ready when the page connected was spoken before", () => {
    const first = S.nextCue({ last: null }, snapshot(cue(7)));
    assert.deepEqual(first, { state: { last: 7 }, play: null });
    const waiting = S.nextCue({ last: null }, snapshot(cue(8, null)));
    assert.equal(S.nextCue(waiting.state, snapshot(cue(8))).play.nonce, 8, "one still waiting plays when ready");
  });

  test("a cue older than the last one means the server restarted", () => {
    let state = S.nextCue({ last: null }, snapshot(cue(40))).state;
    const restarted = S.nextCue(state, snapshot(cue(1, null)));
    assert.deepEqual(restarted, { state: { last: 0 }, play: null }, "the count starts afresh");
    state = restarted.state;
    assert.equal(S.nextCue(state, snapshot(cue(1))).play.nonce, 1, "and its first cue plays when ready");
  });

  test("audio that comes too late, or a malformed cue, stays silent", () => {
    const late = S.nextCue({ last: 0 }, snapshot(cue(3, AUDIO, 0), "agree", S.MAX_CUE_AGE_MS + 1));
    assert.deepEqual(late, { state: { last: 3 }, play: null });
    for (const odd of [cue(4, "../ai"), cue(4, AUDIO.toUpperCase()), { ...cue(4), nonce: "4" }, "cue"]) {
      assert.equal(S.nextCue({ last: 0 }, snapshot(odd)).play, null, JSON.stringify(odd));
    }
  });
});

describe("the player", () => {
  /** One fake audio element that records what it was asked to do. */
  function fakeAudio(log) {
    const listeners = {};
    const audio = {
      src: "",
      play: () => {
        log.push(`play ${audio.src}`);
        return Promise.resolve();
      },
      pause: () => log.push("pause"),
      addEventListener: (type, listener) => {
        listeners[type] = listener;
      },
      end: () => listeners.ended(),
    };
    return audio;
  }

  test("plays new cues from the server with the page token on one element", () => {
    const log = [];
    let made = 0;
    const audio = fakeAudio(log);
    const consider = S.createPlayer("overlay token", () => {
      made += 1;
      return audio;
    });
    consider(snapshot(null));
    assert.equal(made, 0, "no element before the first cue");
    consider(snapshot(cue(1)));
    consider(snapshot(cue(1)));
    const url = `/api/speech/${AUDIO}?token=overlay%20token`;
    assert.deepEqual(log, ["pause", `play ${url}`]);
    consider(snapshot(cue(2, AUDIO.replace("0", "f"))));
    assert.deepEqual(log.slice(2), ["pause", `play ${url.replace("0", "f")}`], "a newer cue replaces it");
    assert.equal(made, 1);
  });

  test("lowering the card stops it, and a finished cue is not stopped again", () => {
    const log = [];
    const audio = fakeAudio(log);
    const consider = S.createPlayer("t", () => audio);
    consider(snapshot(null));
    consider(snapshot(cue(1)));
    consider(snapshot(cue(1), null));
    assert.deepEqual(log.slice(2), ["pause"], "lowering the card stops it");
    consider(snapshot(cue(2)));
    audio.end();
    consider(snapshot(cue(2), null));
    assert.deepEqual(log.slice(3), ["pause", `play /api/speech/${AUDIO}?token=t`], "nothing to stop after the end");
  });

  test("a restarted server's cues play, and a cue that became ready during a reconnect is not lost", () => {
    const log = [];
    const audio = fakeAudio(log);
    const consider = S.createPlayer("t", () => audio);
    const plays = () => log.filter((line) => line.startsWith("play")).length;
    consider(snapshot(null));
    for (let nonce = 1; nonce <= 3; nonce += 1) {
      consider(snapshot(cue(nonce)));
    }
    assert.equal(plays(), 3);
    consider(snapshot(cue(4)));
    assert.equal(plays(), 4, "the first snapshot after a reconnect to the same server plays a cue it missed");
    consider(snapshot(cue(1, null)));
    consider(snapshot(cue(1)));
    assert.equal(plays(), 5, "cue 1 of a restarted server plays");
  });

  test("the address escapes the id and the token", () => {
    assert.equal(S.speechUrl("ab", "t&k"), "/api/speech/ab?token=t%26k");
  });

  test("a card that is still up keeps speaking", () => {
    const cueView = cue(1);
    assert.equal(S.shouldStop(cueView, snapshot(cueView)), false);
    assert.equal(S.shouldStop(cueView, snapshot(cueView, "disagree")), true);
    assert.equal(S.shouldStop(cueView, { decks: [] }), true);
    const covered = { decks: [{ id: "meeting", raised: "agree", covered: true }] };
    assert.equal(S.shouldStop(cueView, covered), true, "a covered card goes quiet");
  });
});
