// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the built plugin end to end, started the way Stream Deck starts it, against a fake
// Stream Deck and a fake Sceneuron: pairing from the inspector, a token revoked while connected
// that darkens keys with "Pair again", a revoke that ends the stream and turns the keys and dial
// from live to "Pair again" without "Offline" on the way, pairing again that goes from "Pair
// again" straight to live, a paired start against a hung server that says "Offline" rather than
// "Not paired", keys that keep their live faces for 1.5 s while the link cannot reconnect, that
// the token reaches neither the inspector nor the log, and the license notices in the bundle. Run
// `npm run build` first; `npm test` does.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import { CONTROL_TOKEN } from "./fake-sceneuron.mjs";
import { PLUGIN_DIR, alert, appear, bundleExists, dim, image, keyDown, lit, strip } from "./fake-stream-deck.mjs";
import { startPlugin, waitFor } from "./harness.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";

function status(state) {
  return (message) => message.event === "sendToPropertyInspector" && message.payload.state === state;
}

test("a key paired in the inspector is live until its token is revoked and never leaks it", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, { code: "246810" }, () => ({}));
  const settings = { deck: "poker", card: "p5" };
  const unpaired = image("key-1", (svg) => />Not paired</.test(svg));
  await deck.after(appear("card", "key-1", settings), unpaired, "the unpaired key");

  const shown = { action: "ai.chaski.sceneuron.card", context: "key-1" };
  deck.send({ event: "propertyInspectorDidAppear", device: "deck", ...shown });
  const pair = { type: "pair", port: String(sceneuron.port), code: "246 810" };
  const answer = (message) => message.event === "sendToPropertyInspector" && message.payload.type === "paired";
  const paired = await deck.after({ event: "sendToPlugin", ...shown, payload: pair }, answer, "a pairing answer");
  assert.equal(paired.payload.ok, true, paired.payload.message);
  assert.deepEqual(deck.globals, { port: sceneuron.port, token: CONTROL_TOKEN });

  await deck.next(image("key-1", (svg) => />5<\/text>/.test(svg) && !dim(svg)), "the live card");
  const named = (message) => status("connected")(message) && message.payload.decks.length > 0;
  const connected = await deck.next(named, "a connected status with the decks");
  assert.deepEqual(
    connected.payload.decks.map((entry) => entry.id),
    ["poker", "meeting"],
  );
  await deck.after(keyDown("card", "key-1", settings), image("key-1", lit), "the lit frame");
  assert.equal(sceneuron.view.decks[0].raised, "p5");

  // The editor's Disconnect Stream Decks rotates the token; the open stream stays open.
  sceneuron.rotateToken("rotated-token-0123456789abcdef");
  await deck.after(keyDown("card", "key-1", settings), alert("key-1"), "the alert sign");
  await deck.next(image("key-1", (svg) => />Pair again</.test(svg) && dim(svg)), "the refused key");
  await deck.next(status("refused"), "the refused status");

  for (const message of deck.messages.filter((m) => m.event !== "setGlobalSettings")) {
    assert.ok(!JSON.stringify(message).includes(CONTROL_TOKEN), `${message.event} carried the token`);
  }
  const logs = deck.logs();
  const refusal = `the server on port ${sceneuron.port} refused this plugin's token`;
  assert.ok(logs.some((text) => text.includes(refusal)), "the refusal is in the log");
  assert.ok(logs.every((text) => !text.includes(CONTROL_TOKEN)), "a log holds the token");
});

test("a revoke that ends the stream turns the keys and dial to Pair again without Offline", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, (fake) => ({ port: fake.port, token: CONTROL_TOKEN }));
  const live = image("key-1", (svg) => />5<\/text>/.test(svg) && !dim(svg));
  await deck.after(appear("card", "key-1", { deck: "poker", card: "p5" }), live, "the live card");
  const dial = appear("dial", "dial-1", { deck: "poker" }, "Encoder");
  await deck.after(dial, strip("dial-1", (svg) => !dim(svg)), "the live dial");

  // Sceneuron ends a revoked token's streams, and the plugin proves the server again at once.
  const from = deck.messages.length;
  sceneuron.rotateToken("rotated-token-0123456789abcdef");
  sceneuron.endStreams();
  const again = (svg) => />Pair again</.test(svg) && dim(svg);
  await deck.next(image("key-1", again), "the refused key", from);
  await deck.next(strip("dial-1", again), "the refused dial", from);
  const offline = (svg) => />Offline</.test(svg);
  const flashed = deck.messages.slice(from).filter((m) => image("key-1", offline)(m) || strip("dial-1", offline)(m));
  assert.deepEqual(flashed, [], "drawings that said Offline after the revoke");
});

test("pairing again after a revoke goes from Pair again straight to the live faces", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, (fake) => ({ port: fake.port, token: CONTROL_TOKEN }));
  const live = (svg) => />5<\/text>/.test(svg) && !dim(svg);
  await deck.after(appear("card", "key-1", { deck: "poker", card: "p5" }), image("key-1", live), "the live card");
  sceneuron.rotateToken("rotated-token-0123456789abcdef");
  sceneuron.endStreams();
  await deck.next(image("key-1", (svg) => />Pair again</.test(svg)), "the refused key");

  // The new token restarts the link, which shows neither "Offline" nor the bare card id meanwhile.
  sceneuron.issueCode("135790");
  const shown = { action: "ai.chaski.sceneuron.card", context: "key-1" };
  deck.send({ event: "propertyInspectorDidAppear", device: "deck", ...shown });
  const from = deck.messages.length;
  const pair = { type: "pair", port: String(sceneuron.port), code: "135790" };
  const answer = (message) => message.event === "sendToPropertyInspector" && message.payload.type === "paired";
  const reply = await deck.after({ event: "sendToPlugin", ...shown, payload: pair }, answer, "a pairing answer");
  assert.equal(reply.payload.ok, true, reply.payload.message);
  await deck.next(image("key-1", live), "the live card again", from);
  const others = deck.messages.slice(from).filter(image("key-1", (svg) => !live(svg)));
  assert.deepEqual(others, [], "drawings after pairing again that were not live");
});

test("a paired plugin that starts while Sceneuron hangs says Offline, never Not paired", { skip }, async (t) => {
  const paired = (fake) => ({ port: fake.port, token: CONTROL_TOKEN });
  const { sceneuron, deck } = await startPlugin(t, { hung: true }, paired);
  // A key that appears before the plugin has read its settings is rightly not paired yet.
  await waitFor(() => sceneuron.requests.length > 0, "the plugin's first hello");
  const offline = image("key-1", (svg) => />Offline</.test(svg));
  await deck.after(appear("card", "key-1", { deck: "poker", card: "p5" }), offline, "the offline card");
  const unpaired = deck.messages.filter(image("key-1", (svg) => />Not paired</.test(svg)));
  assert.deepEqual(unpaired, [], "drawings that said Not paired");
});

test("keys keep their live faces for 1.5 s while the link cannot reconnect, then say Offline", { skip }, async (t) => {
  const { sceneuron, deck } = await startPlugin(t, {}, (fake) => ({ port: fake.port, token: CONTROL_TOKEN }));
  const live = image("key-1", (svg) => />5<\/text>/.test(svg) && !dim(svg));
  await deck.after(appear("card", "key-1", { deck: "poker", card: "p5" }), live, "the live card");

  // The proof after the stream ends gets no reply, and the request gives up only after 5 s.
  const from = deck.messages.length;
  sceneuron.hang();
  sceneuron.endStreams();
  const ended = Date.now();
  await deck.next(image("key-1", (svg) => />Offline</.test(svg) && dim(svg)), "the offline key", from);
  const took = Date.now() - ended;
  assert.ok(took >= 1_400 && took <= 3_000, `the key said Offline ${took} ms after the stream ended`);
});

test("the bundle ships the license of every package it contains", { skip }, () => {
  const text = readFileSync(path.join(PLUGIN_DIR, "bin", "third-party-notices.txt"), "utf8");
  for (const name of ["@elgato/streamdeck", "@elgato/utils", "ws"]) {
    assert.match(text, new RegExp(`^${name} \\d+\\.\\d+\\.\\d+ \\(MIT\\)$`, "m"), name);
  }
  assert.match(text, /Permission is hereby granted/);
});
