// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The Stream Deck live test's deep link steps, after streamdeck-steps.mjs: a fresh, unpaired plugin gets the deep
// link the editor's "Pair the Stream Deck on this computer" opens, as Stream Deck delivers it, with a code the editor
// token asked for; it pairs through the same exchange and hello proof as a typed code, every key and the dial go
// live, and Sceneuron counts the pairing the editor watches for. The same link again, a wrong code, and bad ports
// then pair nothing and leave every key live, and neither the codes nor the token reach a key, the inspector, the
// log, or an address.

import { createRequire } from "node:module";

import { raisedCard } from "./card-steps.mjs";
import { waitFor } from "./cdp.mjs";
import { ACTIONS, KEYS } from "./streamdeck-steps.mjs";
import { readRequests, tokenBeforeProof } from "./streamdeck-log.mjs";

const require = createRequire(import.meta.url);
const { pairingLink } = require("../web/editor/stream-deck-text.js");
const js = JSON.stringify;
const INSPECTOR = { action: "ai.chaski.sceneuron.card", context: KEYS.card, device: "deck" };

/** Runs the deep link steps with the harness `t` that streamdeck-live.mjs gives runStreamDeckSteps. */
export async function runStreamDeckLinkSteps(t) {
  const { step, same, api, port, fake, start } = t;
  const drawing = (action, context, check) => (action === "dial" ? fake.strip : fake.image)(context, check);
  const pairRequests = (deck) => readRequests(deck.output).filter((entry) => entry.path === "/api/control/pair");
  const codes = [];
  let deck;
  let token;

  await step("an unpaired plugin pairs from the editor's deep link alone, and every key goes live", async () => {
    deck = await start();
    for (const [action, context, settings, controller] of ACTIONS) {
      deck.send(fake.appear(action, context, settings, controller));
    }
    await deck.next(fake.image(KEYS.card, (svg) => />Not paired</.test(svg)), "the unpaired card key");
    const { code, pairings } = await api("POST", "/api/control/pairing");
    codes.push(code);
    const from = deck.messages.length;
    deck.send(fake.deepLink(pairingLink(port, code)));
    for (const [action, context] of ACTIONS) {
      await deck.next(drawing(action, context, (svg) => !fake.dim(svg)), `the live ${action} control`, from);
    }
    token = deck.globals.token;
    same([deck.globals.port, typeof token], [port, "string"], "the global settings");
    const [pair] = pairRequests(deck);
    same([pair?.status, pair?.origin, pair?.token], [200, false, false], "the pairing request");
    const broken = tokenBeforeProof(readRequests(deck.output));
    if (broken) throw new Error(broken);
    same((await api("GET", "/api/control/pairing")).pairings, pairings + 1, "codes traded, as the editor sees");
    return `${ACTIONS.length} controls live`;
  });
  if (!token) throw new Error("the plugin did not pair from the link, so no other link step can run");

  await step("the same link again, a wrong code, and bad ports pair nothing and keep every key live", async () => {
    deck.send({ event: "propertyInspectorDidAppear", ...INSPECTOR });
    const from = deck.messages.length;
    const asked = pairRequests(deck).length;
    const { code } = await api("POST", "/api/control/pairing");
    const wrong = code === "000000" ? "000001" : "000000";
    codes.push(code, wrong);
    const answered = [
      [pairingLink(port, codes[0]), /wrong, used, or expired/],
      [pairingLink(port, wrong), /wrong, used, or expired/],
      [pairingLink(port === 1 ? 2 : 1, code), /Nothing answered on port/],
    ];
    for (const [link, message] of answered) {
      const answer = ({ event, payload }) => event === "sendToPropertyInspector" && payload.type === "paired";
      const reply = await deck.after(fake.deepLink(link), answer, "a pairing answer");
      if (reply.payload.ok || !message.test(reply.payload.message)) throw new Error(JSON.stringify(reply.payload));
    }
    const base = "streamdeck://plugins/message/ai.chaski.sceneuron/pair";
    const ignored = [`${base}?port=0&code=${code}`, `${base}?port=65536&code=${code}`, `${base}?port=${port}`];
    for (const link of ignored) {
      deck.send(fake.deepLink(link));
    }
    const said = () => deck.logs().join("\n").split("ignored a deep link that is not a pairing link").length - 1;
    await waitFor("the malformed links ignored", async () => said() === ignored.length);
    const statuses = pairRequests(deck).slice(asked).map((entry) => entry.status);
    same(statuses, [403, 403, 0], "the pairing requests the links made");
    same([deck.globals.port, deck.globals.token], [port, token], "the global settings");
    const since = deck.messages.slice(from);
    same(since.filter(({ event }) => event === "setGlobalSettings").length, 0, "settings saved");
    const dark = ACTIONS.filter(([action, context]) => since.some(drawing(action, context, fake.dim)));
    same(dark.map(([action]) => action), [], "controls drawn dark");
    const raised = async () => raisedCard(await api("GET", "/api/state"), ACTIONS[0][2].deck);
    const before = await raised();
    deck.send(fake.keyDown("card", KEYS.card, ACTIONS[0][2]));
    await waitFor("the card key working with the kept pairing", async () => (await raised()) !== before);
    return `${answered.length} links refused, ${ignored.length} ignored`;
  });

  await step("the codes and the token reach no key, inspector message, log, or address", async () => {
    const secrets = [token, ...codes];
    const sent = deck.messages.filter(({ event }) => event !== "setGlobalSettings").map((message) => js(message));
    const addresses = readRequests(deck.output).map((entry) => `${entry.path}${entry.query}`);
    for (const [where, texts] of [["a message", sent], ["an address", addresses], ["the log", deck.logs()]]) {
      if (texts.some((text) => secrets.some((secret) => text.includes(secret)))) throw new Error(`${where} held one`);
    }
  });
}
