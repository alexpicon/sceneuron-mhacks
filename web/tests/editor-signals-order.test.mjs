// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/signals-text.js's notes on two cards of one deck whose signals one pose shows: the deck raises the
// card that comes first, unless a vote keeps the camera from matching it, as signals-context.js decides; then the note
// says which card is raised instead.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { POSES } from "./editor-signals-fixtures.mjs";

const require = createRequire(import.meta.url);
const S = require("../editor/signals-text.js");
const { SIGN_AND_COUNT, THUMBS_AND_COUNT, TWO_SIGNS } = POSES;

const card = (id, name, signal) => ({ id, name, signal });

/** A meeting deck that also reads 5 fingers, beside a poker deck, both listening, and the decks in `more`. */
function project(...more) {
  const meeting = [
    card("agree", "Agree", { sign: "thumbs_up" }),
    card("wrap-up", "Wrap it up", { sign: "fist" }),
    card("next-topic", "Next topic", { sign: "peace" }),
    card("offline", "Offline", { fingers: 5 }),
  ];
  const poker = [0, 2, 3, 5].map((count) => card(`p${count}`, String(count), { fingers: count }));
  const decks = [{ id: "meeting", name: "Meeting signals", cards: meeting }, ...more];
  decks.splice(1, 0, { id: "poker", name: "Planning poker", cards: poker });
  return { decks: decks.map((deck) => ({ ...deck, camera: true })) };
}

/** The notes on card `cardId` of deck `deckId` about cards of its own deck. */
function ownNotes(draft, deckId, cardId) {
  const deck = draft.decks.find((found) => found.id === deckId);
  const signal = deck.cards.find((found) => found.id === cardId).signal;
  const others = draft.decks.filter((found) => found !== deck).map((found) => `${found.name}: `);
  const own = (note) => !others.some((prefix) => note.startsWith(prefix));
  return S.signalConflicts(draft, deckId, cardId, signal).notes.filter(own);
}

describe("two cards of one deck that one pose shows", () => {
  const poker = "a vote is open in Planning poker";

  test("a count raised either way wins while a vote keeps the sign that comes first from being matched", () => {
    assert.deepEqual(ownNotes(project(), "meeting", "next-topic"), [
      `Agree and Wrap it up have a thumbs up and a fist. ${TWO_SIGNS}, so this deck raises one of them then, not ` +
        "Next topic, since they come first.",
      `Offline has 5 fingers up. ${SIGN_AND_COUNT}, so this deck raises only Next topic then, since it comes first, ` +
        `but only Offline while ${poker}.`,
    ]);
    assert.deepEqual(ownNotes(project(), "meeting", "offline"), [
      `Agree has a thumbs up. ${THUMBS_AND_COUNT}, so this deck raises only Agree then, since it comes first.`,
      `Wrap it up and Next topic have a fist and a peace sign. ${SIGN_AND_COUNT}, so this deck raises one of them ` +
        `then, not Offline, since they come first, but only this card while ${poker}.`,
    ]);
  });

  test("a sign raised either way splits from the signs that wait for no vote", () => {
    const draft = project();
    draft.decks[0].cards.push(card("disagree", "Disagree", { sign: "thumbs_down" }));
    assert.deepEqual(ownNotes(draft, "meeting", "disagree"), [
      `Agree has a thumbs up. ${TWO_SIGNS}, so this deck raises only Agree then, since it comes first.`,
      `Wrap it up and Next topic have a fist and a peace sign. ${TWO_SIGNS}, so this deck raises one of them then, ` +
        `not Disagree, since they come first, but only this card while ${poker}.`,
      `Offline has 5 fingers up. ${THUMBS_AND_COUNT}, so this deck raises only Offline then, since it comes first.`,
    ]);
  });

  test("a count that waits for its own deck's vote, beside a card raised either way", () => {
    const draft = project();
    draft.decks[1].cards.push(card("break", "Coffee", { sign: "thumbs_up" }));
    assert.deepEqual(ownNotes(draft, "poker", "p3"), [
      `Coffee has a thumbs up. ${THUMBS_AND_COUNT}, so this deck raises only 3 then, since it comes first, but only ` +
        "Coffee while no vote is open in this deck.",
    ]);
  });

  test("a count and a sign that each wait for a different vote", () => {
    const estimates = {
      id: "estimates",
      name: "Estimates",
      cards: [card("three", "Three", { fingers: 3 }), card("pause", "Pause", { sign: "peace" })],
    };
    const draft = project(estimates);
    draft.decks[0].cards.pop();
    assert.deepEqual(ownNotes(draft, "estimates", "three"), [
      "Pause has a peace sign. A peace sign with the thumb out also counts as 3 fingers, so this deck raises only " +
        "Three then, since it comes first, while both can be: this card is raised only while a vote is open in this " +
        "deck, and Pause only while no vote is open in Planning poker.",
    ]);
  });
});
