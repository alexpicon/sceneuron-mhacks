// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/panels/stream-deck.js: Pair the Stream Deck on this computer sends a fresh code to the Stream
// Deck app in a deep link, Connect a Stream Deck shows one to type, an OBS dock offers only the code to type, the
// section says Paired once the server counts the code traded, and an expired code, an answer about an older code,
// and a refused request each say so or change nothing.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import vm from "node:vm";

const require = createRequire(import.meta.url);
const cardsText = require("../editor/cards-text.js");
const streamDeckText = require("../editor/stream-deck-text.js");
const source = readFileSync(new URL("../editor/panels/stream-deck.js", import.meta.url), "utf8");

const LINK = "streamdeck://plugins/message/ai.chaski.sceneuron/pair?port=4466&code=092156&streamdeck=hidden";
const BROWSER =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) " +
  "Chrome/140.0.0.0 Safari/537.36";
const OBS_DOCK =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) " +
  "Chrome/127.0.6533.120 OBS/31.1.2 Safari/537.36";

/** Lets every promise the section is waiting on settle. */
const settle = () => new Promise((resolve) => setImmediate(resolve));

/**
 * Wires the section, in a browser whose user agent is `userAgent`, to stand-in elements, which hold only the state
 * the section sets, and to `api`; returns the elements, what the section told the user, its one-second timer, which
 * the test runs by hand, and `pass(ms)`, which moves the section's clock on.
 */
function setup(api, userAgent = BROWSER) {
  const elements = new Map();
  const seen = { clicks: [], focused: null, toasts: [], announced: [], reported: [], timers: new Map() };
  const byId = (id) => {
    if (!elements.has(id)) {
      const listeners = {};
      elements.set(id, {
        id,
        hidden: false,
        textContent: "",
        attributes: {},
        listeners,
        addEventListener: (type, listener) => (listeners[type] = listener),
        setAttribute(name, value) {
          this.attributes[name] = value;
        },
        removeAttribute(name) {
          delete this.attributes[name];
        },
        replaceChildren() {},
        click: () => seen.clicks.push(id),
        focus: () => (seen.focused = id),
      });
    }
    return elements.get(id);
  };
  let next = 0;
  const context = vm.createContext({
    SceneuronEditor: { cardsText, streamDeckText, dom: { h: () => ({}) } },
    SceneuronLink: { pagePort: () => "4466" },
    location: {},
    navigator: { userAgent },
    setInterval: (tick) => {
      next += 1;
      seen.timers.set(next, tick);
      return next;
    },
    clearInterval: (id) => seen.timers.delete(id),
  });
  context.window = context;
  let skew = 0;
  vm.runInContext("Date", context).now = () => Date.now() + skew;
  vm.runInContext(source, context, { filename: "editor/panels/stream-deck.js" });
  const editor = {
    api,
    byId,
    toast: (message, kind) => seen.toasts.push([message, kind]),
    announce: (message) => seen.announced.push(message),
    report: (error) => seen.reported.push(error.message),
  };
  context.SceneuronEditor.streamDeck(editor);
  const tick = async () => {
    for (const run of seen.timers.values()) {
      run();
    }
    await settle();
  };
  return { byId, seen, tick, pass: (ms) => (skew += ms) };
}

/** An API that issues code 092156 with `pairings` traded so far, then answers each count from `counts` in turn. */
function fakeApi(pairings, counts, expiresInS = 120) {
  const asked = [];
  return {
    asked,
    pairing: async () => {
      asked.push("POST pairing");
      return { code: "092156", expires_in_s: expiresInS, pairings };
    },
    pairings: async () => {
      asked.push("GET pairing");
      return { pairings: counts.length > 1 ? counts.shift() : counts[0] };
    },
    revokeControl: async () => asked.push("POST revoke"),
  };
}

test("Pair the Stream Deck on this computer sends a fresh code to the Stream Deck app in a deep link", async () => {
  const api = fakeApi(2, [2]);
  const { byId, seen } = setup(api);
  byId("pair-here").listeners.click();
  await settle();
  assert.deepEqual(api.asked, ["POST pairing", "GET pairing"]);
  assert.equal(byId("pairing-link").attributes.href, LINK);
  assert.deepEqual(seen.clicks, ["pairing-link"], "the link was opened once");
  assert.equal(byId("pairing-digits").textContent, "092 156", "the code shows too, to type if nothing happens");
  assert.equal(byId("pairing-expiry").textContent, "It works once, for 2:00.");
  const shown = ["pairing", "pairing-sent", "pairing-steps"].map((id) => byId(id).hidden);
  assert.deepEqual(shown, [false, false, false]);
  assert.equal(seen.focused, "pairing-sent");
  assert.deepEqual(seen.reported, []);
});

test("Connect a Stream Deck shows a code to type and opens no link", async () => {
  const { byId, seen } = setup(fakeApi(0, [0]));
  byId("pair-stream-deck").listeners.click();
  await settle();
  assert.deepEqual(seen.clicks, []);
  assert.equal(byId("pairing-sent").hidden, true);
  assert.equal(byId("pairing-digits").textContent, "092 156");
  assert.equal(seen.focused, "pairing-code");
});

test("the section says Paired once the server counts the code traded, and stops asking", async () => {
  const api = fakeApi(2, [2, 2, 3]);
  const { byId, seen, tick } = setup(api);
  byId("pair-here").listeners.click();
  await settle();
  await tick();
  assert.equal(byId("pairing-digits").textContent, "092 156", "the same count is no pairing");
  await tick();
  assert.equal(byId("pairing-digits").textContent, "Paired");
  assert.equal(byId("pairing-expiry").textContent, streamDeckText.WORDS.paired);
  assert.deepEqual([byId("pairing-sent").hidden, byId("pairing-steps").hidden], [true, true]);
  assert.deepEqual(seen.toasts, [[streamDeckText.WORDS.pairedToast, "info"]]);
  assert.equal(seen.timers.size, 0, "the countdown stopped");
  assert.equal(api.asked.filter((call) => call === "GET pairing").length, 3);
});

test("in an OBS dock, the section offers only a code to type and never follows a link", async () => {
  const { byId, seen } = setup(fakeApi(0, [0]), OBS_DOCK);
  assert.equal(byId("pair-here").listeners.click, undefined, "no button sends the code in a link");
  byId("pair-stream-deck").listeners.click();
  await settle();
  assert.deepEqual(seen.clicks, []);
  assert.equal(byId("pairing-link").attributes.href, undefined);
  assert.equal(byId("pairing-digits").textContent, "092 156");
  const shown = ["pairing", "pairing-sent", "pairing-steps"].map((id) => byId(id).hidden);
  assert.deepEqual(shown, [false, true, false], "the code and the steps to type it, with no note that it was sent");
  assert.equal(seen.focused, "pairing-code");
});

test("a typed code that pairs says Paired too", async () => {
  const { byId, seen } = setup(fakeApi(0, [1]));
  byId("pair-stream-deck").listeners.click();
  await settle();
  assert.equal(byId("pairing-digits").textContent, "Paired");
  assert.equal(seen.toasts.length, 1);
});

test("a code sent in a deep link that expires says what to check, and offers to send it no more", async () => {
  const { byId, seen, tick, pass } = setup(fakeApi(4, [4]));
  byId("pair-here").listeners.click();
  await settle();
  assert.equal(byId("pairing-sent").hidden, false);
  pass(119_000);
  await tick();
  assert.equal(byId("pairing-expiry").textContent, "It works once, for 0:01.");
  pass(1_000);
  await tick();
  assert.equal(byId("pairing-digits").textContent, "Expired");
  assert.equal(byId("pairing-expiry").textContent, streamDeckText.expiredNote(true));
  assert.equal(byId("pairing-sent").hidden, true, "no offer to send the expired code again");
  assert.equal(byId("pairing-link").attributes.href, undefined, "the link no longer holds the expired code");
  assert.deepEqual(seen.announced, ["The Stream Deck pairing code expired."]);
  assert.equal(seen.timers.size, 0);
});

test("a typed code that expires says to ask for a new one", async () => {
  const { byId, seen, tick, pass } = setup(fakeApi(0, [0]));
  byId("pair-stream-deck").listeners.click();
  await settle();
  pass(120_000);
  await tick();
  assert.equal(byId("pairing-digits").textContent, "Expired");
  assert.equal(byId("pairing-expiry").textContent, streamDeckText.expiredNote(false));
  assert.deepEqual([byId("pairing-sent").hidden, byId("pairing-steps").hidden], [true, false]);
  assert.deepEqual(seen.announced, ["The Stream Deck pairing code expired."]);
});

test("a code that has already expired when it comes is not sent to the Stream Deck app", async () => {
  const { byId, seen } = setup(fakeApi(4, [4], 0));
  byId("pair-here").listeners.click();
  await settle();
  assert.equal(byId("pairing-digits").textContent, "Expired");
  assert.deepEqual(seen.clicks, []);
  assert.equal(byId("pairing-link").attributes.href, undefined);
  assert.equal(byId("pairing-sent").hidden, true);
  assert.equal(seen.focused, "pairing-code");
  assert.equal(seen.timers.size, 0);
});

test("an answer that comes after the code was revoked shows no pairing", async () => {
  let answer;
  const api = fakeApi(2, [2]);
  api.pairings = () => new Promise((resolve) => (answer = resolve));
  const { byId, seen } = setup(api);
  byId("pair-here").listeners.click();
  await settle();
  byId("revoke-confirmed").listeners.click();
  await settle();
  answer({ pairings: 3 });
  await settle();
  assert.equal(byId("pairing").hidden, true);
  assert.notEqual(byId("pairing-digits").textContent, "Paired");
  assert.deepEqual(seen.toasts.map(([message]) => message), [
    "Disconnected every Stream Deck. Connect one again to use it.",
  ]);
});

test("a refused request for a code reports it and sends no link", async () => {
  const api = fakeApi(0, [0]);
  api.pairing = async () => {
    throw new Error("Sceneuron is not answering at this address.");
  };
  const { byId, seen } = setup(api);
  byId("pair-here").listeners.click();
  await settle();
  assert.deepEqual(seen.reported, ["Sceneuron is not answering at this address."]);
  assert.deepEqual(seen.clicks, []);
  assert.equal(byId("pairing-link").attributes.href, undefined);
});
