// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises pairing from the editor's deep link in the built plugin, started the way Stream Deck starts it, against
// a fake Stream Deck and a fake Sceneuron: the link the editor writes pairs every key at once through the same
// exchange and hello proof as a typed code; a link with a used, expired, or wrong code, a bad port, a port where
// nothing answers, or another path pairs nothing and keeps the current pairing; a link that comes while the plugin
// starts is not undone by the saved pairing; and neither the code nor the token reaches the log. Run
// `npm run build` first; `npm test` does.

import assert from "node:assert/strict";
import http from "node:http";
import { createRequire } from "node:module";
import { test } from "node:test";

import { CONTROL_TOKEN, startFakeSceneuron } from "./fake-sceneuron.mjs";
import {
  appear,
  bundleExists,
  deepLink,
  dim,
  image,
  keyDown,
  lit,
  startStreamDeck,
  strip,
} from "./fake-stream-deck.mjs";
import { startPlugin, waitFor } from "./harness.mjs";

const require = createRequire(import.meta.url);
const { pairingLink } = require("../../web/editor/stream-deck-text.js");
const skip = bundleExists() ? false : "the plugin is not built; run npm run build";

const KEYS = [
  ["card", "key-card", { deck: "poker", card: "p5" }],
  ["lower", "key-lower", { deck: "poker" }],
  ["element", "key-element", { element: "brb" }],
  ["automation", "key-auto", {}],
];
const DIAL = ["dial", "dial-1", { deck: "poker" }, "Encoder"];
const INSPECTOR = { action: "ai.chaski.sceneuron.card", context: "key-card", device: "deck" };
const NEW_TOKEN = "second-control-token-0123456789";

/** Puts every key and the dial on the fake Stream Deck. */
function showAll(deck) {
  for (const [action, context, settings] of KEYS) {
    deck.send(appear(action, context, settings));
  }
  deck.send(appear(...DIAL));
}

/** Waits until every key and the dial are drawn live after message `from`. */
async function everyLive(deck, from) {
  const live = (svg) => !dim(svg);
  for (const [action, context] of KEYS) {
    await deck.next(image(context, live), `the live ${action} key`, from);
  }
  await deck.next(strip(DIAL[1], live), "the live dial", from);
}

/** The pairing answer the plugin sends its open inspector after message `from`. */
function pairingAnswer(deck, from) {
  const answer = (message) => message.event === "sendToPropertyInspector" && message.payload.type === "paired";
  return deck.next(answer, "a pairing answer", from);
}

/** A port on 127.0.0.1 where nothing listens any more. */
async function closedPort() {
  const server = http.createServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}

test("the editor's link pairs every key at once, proving the server before the token is used", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, { code: "097531" }, () => ({}));
  showAll(deck);
  await deck.next(image("key-card", (svg) => />Not paired</.test(svg)), "the unpaired key");
  const from = deck.messages.length;
  deck.send(deepLink(pairingLink(sceneuron.port, "097531")));
  await everyLive(deck, from);
  assert.deepEqual(deck.globals, { port: sceneuron.port, token: CONTROL_TOKEN });
  const requests = sceneuron.requests.map((request) => [request.path, "x-sceneuron-token" in request.headers]);
  assert.deepEqual(requests.slice(0, 3), [
    ["/api/control/pair", false],
    ["/api/hello", false],
    ["/api/hello", false],
  ]);
  assert.ok(requests.slice(3).every(([path, token]) => token || path === "/api/hello"), JSON.stringify(requests));
  const logs = deck.logs().join("\n");
  assert.match(logs, /a pairing link: Paired with Sceneuron on port \d+\./);
  assert.ok(!logs.includes("097531") && !logs.includes(CONTROL_TOKEN), "the log holds the code or the token");
});

test("used, expired, wrong, and malformed links pair nothing and keep the pairing", { skip }, async (t) => {
  const paired = (fake) => ({ port: fake.port, token: CONTROL_TOKEN });
  const { sceneuron, deck } = await startPlugin(t, {}, paired);
  showAll(deck);
  await everyLive(deck, 0);
  deck.send({ event: "propertyInspectorDidAppear", ...INSPECTOR });

  // A revoke and a fresh code: the link pairs again with the new token, and the keys keep their faces.
  sceneuron.rotateToken(NEW_TOKEN);
  sceneuron.issueCode("135790");
  const used = pairingLink(sceneuron.port, "135790");
  let from = deck.messages.length;
  deck.send(deepLink(used));
  assert.equal((await pairingAnswer(deck, from)).payload.ok, true);
  const connected = (message) => message.event === "sendToPropertyInspector" && message.payload.state === "connected";
  await deck.next(connected, "the connected status", from);
  const kept = { port: sceneuron.port, token: NEW_TOKEN };
  assert.deepEqual(deck.globals, kept);

  from = deck.messages.length;
  const asked = sceneuron.requests.filter((request) => request.path === "/api/control/pair").length;
  sceneuron.issueCode("246802");
  sceneuron.issueCode(null);
  const nowhere = await closedPort();
  const refused = [
    [used, /wrong, used, or expired/],
    [pairingLink(sceneuron.port, "246802"), /wrong, used, or expired/],
    [pairingLink(sceneuron.port, "000000"), /wrong, used, or expired/],
    [pairingLink(nowhere, "135790"), new RegExp(`Nothing answered on port ${nowhere}`)],
  ];
  for (const [link, message] of refused) {
    const start = deck.messages.length;
    deck.send(deepLink(link));
    const answer = await pairingAnswer(deck, start);
    assert.equal(answer.payload.ok, false, link);
    assert.match(answer.payload.message, message, link);
  }
  const base = `streamdeck://plugins/message/ai.chaski.sceneuron`;
  const malformed = [
    `${base}/pair?port=0&code=135790`,
    `${base}/pair?port=${sceneuron.port}&code=135%20790`,
    `${base}/pair?port=${sceneuron.port}&port=1&code=135790`,
    `${base}/unpair?port=${sceneuron.port}&code=135790`,
  ];
  for (const link of malformed) {
    deck.send(deepLink(link));
  }
  const ignored = () => deck.logs().join("\n").split("ignored a deep link that is not a pairing link").length - 1;
  await waitFor(() => ignored() === malformed.length, "every malformed link ignored");

  const pairs = sceneuron.requests.filter((request) => request.path === "/api/control/pair").length - asked;
  assert.equal(pairs, 3, "only the well-formed links to this port asked to pair");
  const since = deck.messages.slice(from);
  assert.deepEqual(since.filter((message) => message.event === "setGlobalSettings"), [], "settings saved");
  assert.deepEqual(since.filter(image("key-card", dim)), [], "dark drawings of the card key");
  assert.deepEqual(deck.globals, kept);
  await deck.after(keyDown(...KEYS[0]), image("key-card", lit), "the card raised with the kept pairing");
  assert.equal(sceneuron.view.decks[0].raised, "p5");
  const logs = deck.logs().join("\n");
  for (const secret of ["135790", "246802", "000000", CONTROL_TOKEN, NEW_TOKEN]) {
    assert.ok(!logs.includes(secret), `the log holds ${secret}`);
  }
});

test("a link that comes while the plugin starts is not undone by the saved pairing", { skip }, async (t) => {
  const sceneuron = await startFakeSceneuron({ token: NEW_TOKEN, code: "097531" });
  const saved = { port: sceneuron.port, token: CONTROL_TOKEN };
  const deck = await startStreamDeck(saved, { holdGlobals: true });
  t.after(async () => {
    await deck.stop();
    await sceneuron.close();
  });
  await deck.next((message) => message.event === "getGlobalSettings", "the request for the saved pairing");
  deck.send(deepLink(pairingLink(sceneuron.port, "097531")));
  await new Promise((resolve) => setTimeout(resolve, 200));
  assert.deepEqual(sceneuron.requests, [], "the link waited for the saved pairing");
  deck.releaseGlobals();
  await waitFor(() => deck.globals.token === NEW_TOKEN, "the new pairing saved");
  const from = deck.messages.length;
  showAll(deck);
  await everyLive(deck, from);
  assert.deepEqual(deck.globals, { port: sceneuron.port, token: NEW_TOKEN });
});
