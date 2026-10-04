// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The part of a fake Sceneuron that changes the control view: card actions with wrap-around,
// element toggles, automation, push to talk, the shared room's reveal and next round, and reactions,
// with the same 400, 403, 404, and 409 cases as the shared contract.

import { BUILT_IN_REACTIONS } from "../src/lib/built-in-reactions.ts";

/** A control view with a poker deck, a meeting deck, and two elements. */
export function sampleView() {
  const card = (id, name, text, color = "#ffffff", background = "#1d4ed8") => ({ id, name, text, color, background });
  return {
    revision: 1,
    automation_enabled: true,
    decks: [
      {
        id: "poker",
        name: "Planning poker",
        raised: null,
        raised_by: null,
        hide_at_ms: null,
        covered: false,
        cards: [card("p1", "1", "1"), card("p2", "2", "2"), card("p3", "3", "3"), card("p5", "5", "5")],
      },
      {
        id: "meeting",
        name: "Meeting signals",
        raised: null,
        raised_by: null,
        hide_at_ms: null,
        covered: false,
        cards: [card("wrap-up", "Wrap it up", "⏱️ Wrap it up", "#111111", "#facc15"), card("agree", "Agree", "")],
      },
    ],
    elements: [
      { id: "brb", name: "Be right back", visible: false, held: false },
      { id: "lower-third", name: "Lower third", visible: true, held: false },
    ],
  };
}

const CARD_ACTIONS = new Set(["raise", "toggle", "lower", "next", "previous"]);
const ELEMENT_ACTIONS = new Set(["show", "hide", "toggle", "release"]);

/** A result for the server: an HTTP status and an error message when it failed. */
function fail(status, error) {
  return { status, error };
}

function onlyFields(body, allowed) {
  return typeof body === "object" && body !== null && Object.keys(body).every((key) => allowed.includes(key));
}

/** Applies `{deck, card?, action}` to `view` in place. */
export function applyCard(view, body) {
  const shaped = onlyFields(body, ["deck", "card", "action"]) && typeof body.deck === "string";
  if (!shaped || !CARD_ACTIONS.has(body.action)) {
    return fail(400, "expected {deck, card?, action}");
  }
  const deck = view.decks.find((entry) => entry.id === body.deck);
  if (!deck) {
    return fail(404, `no deck ${body.deck}`);
  }
  const needsCard = body.action === "raise" || body.action === "toggle";
  if (needsCard && typeof body.card !== "string") {
    return fail(400, `${body.action} needs a card`);
  }
  if (needsCard && !deck.cards.some((card) => card.id === body.card)) {
    return fail(404, `no card ${body.card} in ${deck.id}`);
  }
  const raise = (id) => {
    deck.raised = id;
    deck.raised_by = id === null ? null : "manual";
  };
  const index = deck.cards.findIndex((card) => card.id === deck.raised);
  const count = deck.cards.length;
  switch (body.action) {
    case "raise":
      raise(body.card);
      break;
    case "toggle":
      raise(deck.raised === body.card ? null : body.card);
      break;
    case "lower":
      raise(null);
      break;
    case "next":
      raise(deck.cards[index < 0 ? 0 : (index + 1) % count].id);
      break;
    case "previous":
      raise(deck.cards[index < 0 ? count - 1 : (index - 1 + count) % count].id);
      break;
  }
  return { status: 200 };
}

/** Applies `{element, action}` to `view` in place; a manual change holds the element. */
export function applyElement(view, body) {
  if (!onlyFields(body, ["element", "action"]) || !ELEMENT_ACTIONS.has(body.action)) {
    return fail(400, "expected {element, action}");
  }
  const element = view.elements.find((entry) => entry.id === body.element);
  if (!element) {
    return fail(404, `no element ${body.element}`);
  }
  if (body.action === "release") {
    element.held = false;
  } else {
    element.visible = body.action === "toggle" ? !element.visible : body.action === "show";
    element.held = true;
  }
  return { status: 200 };
}

/** Applies `{enabled}` to `view` in place. */
export function applyAutomation(view, body) {
  if (!onlyFields(body, ["enabled"]) || typeof body.enabled !== "boolean") {
    return fail(400, "expected {enabled}");
  }
  view.automation_enabled = body.enabled;
  return { status: 200 };
}

/**
 * Applies `{down}` to `server`, which holds push to talk as `talking`, and to the view's voice
 * part when the view has one.
 */
export function applyTalk(view, body, server) {
  if (!onlyFields(body, ["down"]) || typeof body.down !== "boolean") {
    return fail(400, "expected {down}");
  }
  server.talking = body.down;
  if (view.voice) {
    view.voice.talking = body.down;
  }
  return { status: 200 };
}

/**
 * Applies `{flood}`, a reaction preset, to `view`: it plays at once, with a new nonce from `server`,
 * for its preset's length.
 */
export function applyFlood(view, body, server) {
  const preset = BUILT_IN_REACTIONS.find((entry) => entry.id === body?.flood);
  if (!onlyFields(body, ["flood"]) || !preset) {
    return fail(400, "expected {flood} naming a reaction preset");
  }
  server.nonce = (server.nonce ?? 0) + 1;
  view.flood = { flood: preset.id, nonce: server.nonce, left_ms: preset.duration_ms };
  return { status: 200 };
}

/**
 * Applies a control token's `{action}` to the room `server` has joined, as `room`, and to the
 * view's room part when the view has one. A control token may only reveal and start the next
 * round, and its answer carries no state.
 */
export function applyRoom(view, body, server) {
  if (!onlyFields(body, ["action"]) || typeof body.action !== "string") {
    return fail(400, "expected {action}");
  }
  if (body.action !== "reveal" && body.action !== "next") {
    return fail(403, "the control token can only reveal the votes and start the next round");
  }
  if (!server.room) {
    return fail(409, "join a room first");
  }
  const revealed = body.action === "reveal";
  server.room = { revealed, round: server.room.round + (revealed ? 0 : 1) };
  if (view.room) {
    view.room.revealed = revealed;
    view.room.voted = revealed ? view.room.voted : 0;
  }
  return { status: 200, body: { ok: true } };
}
