// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The Stream Deck live test's steps. Each drives the built plugin through the fake Stream Deck,
// then checks the result three ways: in Sceneuron through the editor token's API, in what the
// plugin drew on its keys and dial, and in the requests the plugin made.

import { PAGES } from "../streamdeck/src/profiles/layout.ts";
import { raisedCard, withPack } from "./card-steps.mjs";
import { waitFor } from "./cdp.mjs";
import { isProof, linkRequests, linkStates, readRequests, tokenBeforeProof } from "./streamdeck-log.mjs";
import { REACTION, runStreamDeckReactionSteps } from "./streamdeck-reaction-steps.mjs";

const DECK = "poker";
const SLOT = "poker-card";
const ELEMENT = "brb";
const CARD = { deck: DECK, card: "p5" };
const LOWER = { deck: DECK };
const DIAL = { deck: DECK };
export const KEYS = {
  card: "key-card",
  lower: "key-lower",
  element: "key-element",
  automation: "key-auto",
  dial: "dial",
  talk: "key-talk",
  reveal: "key-reveal",
  "next-round": "key-next-round",
  reaction: "key-reaction",
};
export const ACTIONS = [
  ["card", KEYS.card, CARD, "Keypad"],
  ["lower", KEYS.lower, LOWER, "Keypad"],
  ["element", KEYS.element, { element: ELEMENT }, "Keypad"],
  ["automation", KEYS.automation, {}, "Keypad"],
  ["dial", KEYS.dial, DIAL, "Encoder"],
  ["talk", KEYS.talk, {}, "Keypad"],
  ["reveal", KEYS.reveal, {}, "Keypad"],
  ["next-round", KEYS["next-round"], {}, "Keypad"],
  ["reaction", KEYS.reaction, REACTION, "Keypad"],
];
const INSPECTOR = { action: "ai.chaski.sceneuron.card", context: KEYS.card };

/** Matches a status the plugin sent its property inspector for connection state `state`. */
function status(state) {
  return ({ event, payload }) =>
    event === "sendToPropertyInspector" && payload.type === "status" && payload.state === state;
}

/** How a request went out and was answered, for comparing lists of requests. */
const answered = (request) => [request.method, request.token, request.status];

/**
 * Runs every step; `t` holds the harness: step, same, api, the server's port, the fake, start, and
 * the address of a stand-in SpacetimeDB for the room.
 */
export async function runStreamDeckSteps(t) {
  const { step, same, api, port, fake, start, roomServer } = t;
  const state = () => api("GET", "/api/state");
  const raised = async () => raisedCard(await state(), DECK);
  const live = (check) => (svg) => check(svg) && !fake.dim(svg);
  const key = (context, check) => fake.image(context, live(check));
  const strip = (at, up) => fake.strip(KEYS.dial, live((svg) => svg.includes(`>${at}<`) && fake.lit(svg) === up));
  const unlit = (svg) => !fake.lit(svg);
  const press = (action, settings = {}) => fake.keyDown(action, KEYS[action], settings);
  const turn = (ticks) => fake.dialEvent("dialRotate", KEYS.dial, DIAL, { pressed: false, ticks });
  let deck;
  let code;
  let token;

  await step("the poker pack is added through PUT /api/project", async () => {
    const pack = (await api("GET", "/api/packs")).find((offered) => offered.id === DECK);
    if (!pack) throw new Error("GET /api/packs offers no poker pack");
    await api("PUT", "/api/project", withPack(await api("GET", "/api/project"), pack));
    same(await raised(), null, "the poker deck's raised card");
    return `${pack.decks[0].cards.length} cards`;
  });

  await step("every deck and card the bundled 15-key layout names is in GET /api/packs", async () => {
    const packs = await api("GET", "/api/packs");
    const cardIds = (deck) => deck.cards.map((card) => card.id);
    const decks = new Map(packs.flatMap((pack) => pack.decks.map((deck) => [deck.id, cardIds(deck)])));
    const keys = PAGES.flatMap((page) => page.rows.flat())
      .filter((key) => key && (key.kind === "card" || key.kind === "lower"));
    const found = (key) => decks.has(key.deck) && (key.kind !== "card" || decks.get(key.deck).includes(key.card));
    const missing = keys.filter((key) => !found(key));
    if (missing.length) throw new Error(`not in the packs: ${JSON.stringify(missing)}`);
    return `${keys.length} keys name decks and cards of ${new Set(keys.map((key) => key.deck)).size} packs`;
  });

  await step("the editor token asks POST /api/control/pairing for a code", async () => {
    const reply = await api("POST", "/api/control/pairing");
    if (!/^\d{6}$/.test(reply.code)) throw new Error(`the code is ${JSON.stringify(reply.code)}`);
    same(reply.expires_in_s, 120, "the code's lifetime in seconds");
    code = reply.code;
  });

  await step("the plugin pairs with that code from its property inspector", async () => {
    deck = await start();
    for (const [action, context, settings, controller] of ACTIONS) {
      deck.send(fake.appear(action, context, settings, controller));
    }
    await deck.next(fake.image(KEYS.card, (svg) => />Not paired</.test(svg)), "the unpaired card key");
    deck.send({ event: "propertyInspectorDidAppear", device: "deck", ...INSPECTOR });
    const pair = { event: "sendToPlugin", ...INSPECTOR, payload: { type: "pair", port: String(port), code } };
    const answer = ({ event, payload }) => event === "sendToPropertyInspector" && payload.type === "paired";
    const reply = await deck.after(pair, answer, "a pairing answer");
    if (!reply.payload.ok) throw new Error(reply.payload.message);
    token = deck.globals.token;
    same([deck.globals.port, typeof token], [port, "string"], "the global settings");
    const request = readRequests(deck.output).find((entry) => entry.path === "/api/control/pair");
    same([request?.status, request?.origin, request?.token], [200, false, false], "the pairing request");
    return reply.payload.message;
  });
  if (!token) throw new Error("the plugin did not pair, so no other step can run");

  await step("the plugin proves the server through /api/hello before it sends the token", async () => {
    await deck.next(status("connected"), "the connected status");
    const stream = (entry) => entry.path === "/api/control/events" && entry.status !== null;
    await waitFor("the answered event stream", () => readRequests(deck.output).some(stream));
    const requests = readRequests(deck.output);
    const broken = tokenBeforeProof(requests);
    if (broken) throw new Error(broken);
    // Pairing checked the new token with a hello of its own; the link must prove the server again.
    const [proof, ...own] = linkRequests(requests);
    const first = own.find((entry) => entry.token);
    if (!proof || !isProof(proof) || !first || proof.answeredAfter >= first.seq) {
      throw new Error(`the link's own requests: ${JSON.stringify([proof, ...own])}`);
    }
    const hellos = requests.filter((entry) => entry.path === "/api/hello");
    if (hellos.length < 2 || !hellos.every(isProof)) throw new Error(`hellos: ${JSON.stringify(hellos)}`);
    if (new Set(hellos.map((entry) => entry.query)).size !== hellos.length) throw new Error("a nonce was used twice");
    return requests.map((entry) => `${entry.method} ${entry.path} ${entry.status ?? "unanswered"}`).join(", ");
  });

  await step("the plugin follows the control event stream, even for changes made in the editor", async () => {
    const read = (path) => readRequests(deck.output).filter((entry) => entry.path === path);
    same(read("/api/control/events").map(answered), [["GET", true, 200]], "the event streams");
    // The plugin also reads the view once beside the stream; wait for its answer before counting.
    await waitFor("the state read", () => read("/api/control/state").some((entry) => entry.status !== null));
    same(read("/api/control/state").map(answered), [["GET", true, 200]], "the state reads");
    await deck.next(key(KEYS.card, (svg) => />5<\/text>/.test(svg) && unlit(svg)), "the live card key");
    const asked = readRequests(deck.output).length;
    const from = deck.messages.length;
    await api("POST", "/api/card", { deck: DECK, card: "p13", action: "raise" });
    await deck.next(strip("7/10", true), "the dial showing 13 raised", from);
    await deck.next(key(KEYS.lower, fake.lit), "the lit lower key", from);
    await api("POST", "/api/card", { deck: DECK, action: "lower" });
    await deck.next(strip("7/10", false), "the dial showing 13 lowered", from);
    same(readRequests(deck.output).length, asked, "requests the plugin made for the editor's changes");
  });

  await step("a card key raises its card, and a second press lowers it", async () => {
    await deck.after(press("card", CARD), key(KEYS.card, fake.lit), "the framed card key");
    const shown = await state();
    const slot = shown.elements.find((element) => element.id === SLOT);
    const { raised_by } = shown.decks.find((entry) => entry.id === DECK);
    same([raisedCard(shown, DECK), raised_by, slot.props.visible, slot.props.text], ["p5", "manual", true, "5"], "p5");
    await deck.after(press("card", CARD), key(KEYS.card, unlit), "the plain card key");
    same(await raised(), null, "the raised card after the second press");
  });

  await step("the lower key lowers the deck", async () => {
    await deck.after(press("card", CARD), key(KEYS.lower, fake.lit), "the lit lower key");
    same(await raised(), "p5", "the raised card");
    await deck.after(press("lower", LOWER), key(KEYS.lower, unlit), "the plain lower key");
    same(await raised(), null, "the raised card after lowering");
  });

  await step("the dial steps the deck with next and previous from the card its strip shows", async () => {
    for (const [ticks, at, expected] of [[1, "6/10", "p8"], [2, "8/10", "p21"], [-3, "5/10", "p5"]]) {
      await deck.after(turn(ticks), strip(at, true), `the strip at ${at}`);
      same(await raised(), expected, `the raised card after a turn of ${ticks}`);
    }
    await deck.after(fake.dialEvent("dialDown", KEYS.dial, DIAL), strip("5/10", false), "the strip lowered");
    same(await raised(), null, "the raised card after a push");
    return "from p5 lowered: +1 raises p8, +2 reaches p21, -3 returns to p5, a push lowers it";
  });

  await step("an element key shows or hides its element and holds the change", async () => {
    const element = async () => (await state()).elements.find((entry) => entry.id === ELEMENT);
    const before = (await element()).props.visible;
    const caption = (visible) => (visible ? "Shown" : "Hidden");
    const held = (visible) => key(KEYS.element, (svg) => svg.includes(`>${caption(visible)}<`) && />Held</.test(svg));
    await deck.after(press("element", { element: ELEMENT }), held(!before), "the flipped element key");
    const flipped = await element();
    same([flipped.props.visible, flipped.overridden.includes("visible")], [!before, true], "the element");
    await deck.after(press("element", { element: ELEMENT }), held(before), "the element key back");
    same((await element()).props.visible, before, "the element's visibility after a second press");
    await api("POST", "/api/release", { element: ELEMENT });
    await deck.next(key(KEYS.element, (svg) => !/>Held</.test(svg)), "the released element key");
    return `${caption(!before)}, then ${caption(before)}, then released in the editor`;
  });

  await step("the automation key pauses and resumes automation", async () => {
    const keyState = (wanted) => ({ event, context, payload }) =>
      event === "setState" && context === KEYS.automation && payload.state === wanted;
    await deck.after(press("automation"), keyState(1), "the paused key state");
    same((await state()).automation_enabled, false, "automation after a press");
    await deck.next(key(KEYS.automation, (svg) => />Paused</.test(svg)), "the paused key");
    await deck.after(press("automation"), keyState(0), "the running key state");
    same((await state()).automation_enabled, true, "automation after a second press");
  });

  const talking = async () => (await state()).voice.talking;
  const requestsTo = (path) => readRequests(deck.output).filter((entry) => entry.path === path);
  const accepted = [
    ["POST", true, 200],
    ["POST", true, 200],
  ];

  await step("push to talk holds while its key is down, and Sceneuron lets go when it comes up", async () => {
    await deck.after(press("talk"), key(KEYS.talk, fake.lit), "the held talk key");
    await waitFor("Sceneuron talking", talking);
    await deck.after(fake.keyUp("talk", KEYS.talk, {}), key(KEYS.talk, unlit), "the let-go talk key");
    await waitFor("Sceneuron letting go", async () => !(await talking()));
    same(requestsTo("/api/voice/talk").map(answered), accepted, "the push to talk requests");
  });

  await step("a push to talk key that disappears while held lets go", async () => {
    await deck.after(press("talk"), key(KEYS.talk, fake.lit), "the held talk key");
    await waitFor("Sceneuron talking", talking);
    deck.send(fake.disappear("talk", KEYS.talk, {}));
    await waitFor("Sceneuron letting go", async () => !(await talking()));
    same(requestsTo("/api/voice/talk").slice(2).map(answered), accepted, "the push to talk requests");
    await deck.after(fake.appear("talk", KEYS.talk, {}), key(KEYS.talk, unlit), "the talk key back");
  });

  const room = () => api("GET", "/api/room");
  let saidNoRoom = 0;

  await step("with no room joined, Reveal votes changes nothing and the room keys say No room", async () => {
    same((await room()).joined, false, "whether a room is joined");
    await deck.after(press("reveal"), fake.alert(KEYS.reveal), "the alert sign");
    const noRoom = (svg) => fake.words(svg).endsWith(" No room") && fake.dim(svg);
    await deck.next(fake.image(KEYS.reveal, noRoom), "the reveal key saying No room");
    await deck.next(fake.image(KEYS["next-round"], noRoom), "the next round key saying No room");
    saidNoRoom = deck.messages.length;
    same(requestsTo("/api/room").map(answered), [["POST", true, 409]], "the room requests");
    same((await room()).joined, false, "whether a room is joined after the press");
  });

  await step("in a room on a stand-in SpacetimeDB, Reveal votes reveals and Next round starts a round", async () => {
    await api("POST", "/api/room", { action: "create", name: "Alex", deck: DECK, server: roomServer });
    const { code } = await room();
    // The press that found no room is forgotten after a few seconds, so in a room the key looks live.
    const plain = key(KEYS.reveal, (svg) => fake.words(svg) === "Reveal votes");
    await deck.next(plain, "the reveal key no longer saying No room", saidNoRoom);
    await api("POST", "/api/card", { deck: DECK, card: "p5", action: "raise" });
    const voted = (shown) => shown.participants.some((person) => person.me && person.voted);
    await waitFor("the hidden vote", async () => voted(await room()));
    const from = deck.messages.length;
    deck.send(press("reveal"));
    await waitFor("the reveal", async () => (await room()).revealed);
    same(deck.messages.slice(from).filter(fake.alert(KEYS.reveal)).length, 0, "alert signs on the reveal key");
    same((await room()).results.map((result) => result.card), ["p5"], "the revealed votes");
    deck.send(press("next-round"));
    await waitFor("the next round", async () => {
      const shown = await room();
      return shown.round === 2 && !shown.revealed;
    });
    await waitFor("the vote lowered", async () => (await raised()) === null);
    same(requestsTo("/api/room").slice(1).map(answered), accepted, "the room requests");
    await api("POST", "/api/room", { action: "leave" });
    return `room ${code}: a vote of 5 revealed, then round 2`;
  });

  const reactionKey = { context: KEYS.reaction, press: press("reaction", REACTION), answered };
  await runStreamDeckReactionSteps(t, { deck, fake, ...reactionKey });

  let asked = 0;
  await step("revoking with the editor token refuses the plugin at once, and its keys say Pair again", async () => {
    const from = deck.messages.length;
    asked = readRequests(deck.output).length;
    const started = Date.now();
    same(await api("POST", "/api/control/revoke"), { ok: true }, "the revoke reply");
    const refused = await deck.next(status("refused"), "the refused status", from);
    const took = Date.now() - started;
    // Sceneuron ends the revoked token's stream, and the plugin proves the server again at once.
    const states = linkStates(deck.messages.slice(from, deck.messages.indexOf(refused) + 1));
    if (states.includes("offline")) throw new Error(`went ${states.join(" -> ")} after the revoke`);
    const drawing = (action, context, check) => (action === "dial" ? fake.strip : fake.image)(context, check);
    const again = (svg) => />Pair again</.test(svg) && fake.dim(svg);
    for (const [action, context] of ACTIONS) {
      await deck.next(drawing(action, context, again), `the ${action} control saying Pair again`, from);
    }
    // The keys keep their live faces while the plugin proves the server, so none says Offline.
    const since = deck.messages.slice(from);
    const offline = (svg) => />Offline</.test(svg);
    const flashed = ACTIONS.filter(([action, context]) => since.some(drawing(action, context, offline)));
    if (flashed.length) throw new Error(`${flashed.map(([action]) => action).join(", ")} showed Offline`);
    return `${states.join(" -> ")} ${took} ms after the revoke, with no Offline on the way`;
  });

  await step("after the revoke a press shows the alert and the old token goes nowhere", async () => {
    const before = await raised();
    await deck.after(press("card", CARD), fake.alert(KEYS.card), "the alert sign");
    same(await raised(), before, "the raised card after the press");
    const since = readRequests(deck.output).slice(asked);
    const strays = since.filter((entry) => entry.token || entry.path !== "/api/hello");
    if (strays.length) throw new Error(`requests since the revoke: ${JSON.stringify(strays)}`);
    return `${since.length} hello request(s) since the revoke, none with the token`;
  });

  await step("the token reaches no key, inspector message, log, or address", async () => {
    const sent = deck.messages.filter(({ event }) => event !== "setGlobalSettings");
    const carried = sent.filter((message) => JSON.stringify(message).includes(token));
    if (carried.length) throw new Error(`${carried.map(({ event }) => event).join(", ")} carried the token`);
    const addresses = readRequests(deck.output).map((entry) => `${entry.path}${entry.query}`);
    if (addresses.some((address) => address.includes(token))) throw new Error("an address held the token");
    if (deck.logs().some((text) => text.includes(token))) throw new Error("the plugin's log held the token");
  });
}
