// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/signals-context.js: when the camera raises a card as the context decides, how that reads in the
// card editor's notes, for cards of other decks and of the same deck, and the Cards tab's live line on where hand
// signals go right now.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const C = require("../editor/signals-context.js");

const card = (id, signal) => ({ id, name: id, signal });

/** A meeting deck with a fist and a thumbs up, a poker deck of finger counts, and a classroom deck of thumbs. */
function project(listening = ["meeting", "poker", "classroom"]) {
  const decks = [
    {
      id: "meeting",
      name: "Meeting signals",
      cards: [card("wrap-up", { sign: "fist" }), card("agree", { sign: "thumbs_up" })],
    },
    { id: "poker", name: "Planning poker", cards: [card("p0", { fingers: 0 }), card("p3", { fingers: 3 })] },
    { id: "classroom", name: "Classroom", cards: [card("hand", "hand_raised"), card("got-it", { sign: "thumbs_up" })] },
  ];
  return { decks: decks.map((deck) => ({ ...deck, camera: listening.includes(deck.id) })) };
}

const always = { when: "always", decks: [] };

describe("when the camera raises a card", () => {
  test("finger counts wait for a vote, and signs that count wait for none, only while such decks compete", () => {
    const gate = C.gates(project(), "classroom", "hand", "hand_raised");
    assert.deepEqual(gate("poker", { fingers: 3 }), { when: "vote", decks: ["Planning poker"] });
    assert.deepEqual(gate("meeting", { sign: "fist" }), { when: "no-vote", decks: ["Planning poker"] });
    assert.deepEqual(gate("meeting", { sign: "thumbs_up" }), always, "a thumbs up gives no count");
    assert.deepEqual(gate("classroom", "hand_raised"), always, "a raised hand is no sign");
    const alone = C.gates(project(["poker", "classroom"]), "poker", "p3", { fingers: 3 });
    assert.deepEqual(alone("poker", { fingers: 3 }), always, "no listening deck reads a sign that counts");
  });

  test("the card being edited counts as listening, with the signal chosen for it", () => {
    const quiet = project(["poker"]);
    const gate = C.gates(quiet, "meeting", "agree", { sign: "peace" });
    assert.deepEqual(gate("meeting", { sign: "peace" }), { when: "no-vote", decks: ["Planning poker"] });
    assert.deepEqual(gate("poker", { fingers: 0 }), { when: "vote", decks: ["Planning poker"] });
    const own = C.gates(quiet, "poker", "p3", { fingers: 3 });
    assert.deepEqual(own("poker", { fingers: 3 }), always, "the meeting deck's hand signals are off");
    const counted = C.gates(project(["meeting"]), "poker", "p3", { fingers: 3 });
    assert.deepEqual(counted("poker", { fingers: 3 }), { when: "vote", decks: ["this deck"] });
  });

  test("cards split by their gate, in their order", () => {
    const gate = C.gates(project(), "classroom", "got-it", { sign: "thumbs_up" });
    const cards = project().decks[0].cards.concat(card("question", { sign: "pointing" }));
    const groups = C.byGate(cards, (one) => gate("meeting", one.signal));
    assert.deepEqual(groups.map(([found, group]) => [found.when, group.map((one) => one.id)]), [
      ["no-vote", ["wrap-up", "question"]],
      ["always", ["agree"]],
    ]);
  });
});

describe("how a vote decides", () => {
  const vote = (name) => ({ when: "vote", decks: [name] });
  const none = (...names) => ({ when: "no-vote", decks: names });

  test("nothing is decided when neither card waits", () => {
    assert.equal(C.decides(always, always, "Got it"), null);
  });

  test("a sign and a count of one vote go one way each", () => {
    assert.equal(C.decides(none("Planning poker"), vote("Planning poker"), "3"),
      "a vote in Planning poker decides: 3 is raised while one is open, and this card while none is");
    assert.equal(C.decides(vote("this deck"), none("this deck"), "one of them"),
      "a vote in this deck decides: this card is raised while one is open, and one of them while none is");
  });

  test("a card that never waits is raised either way, beside one that waits", () => {
    assert.equal(C.decides(always, vote("Planning poker"), "one of them"), "a vote in Planning poker decides: this " +
      "card and one of them are raised while one is open, and only this card is raised while none is");
    assert.equal(C.decides(none("Planning poker"), always, "Got it"), "a vote in Planning poker decides: only Got " +
      "it is raised while one is open, and both are raised while none is");
    assert.equal(C.decides(none("Planning poker"), none("Planning poker"), "one of them"), "a vote in Planning " +
      "poker decides: none of these cards is raised while one is open, and this card and one of them are raised " +
      "while none is");
  });

  test("cards of one deck split by whether their order alone decides which is raised", () => {
    const gates = { a: always, b: none("Planning poker"), c: always, d: vote("this deck") };
    const cards = Object.keys(gates).map((id) => card(id, null));
    const split = (mine, ahead) => C.byOrder(cards, mine, (one) => gates[one.id], (one) => ahead.includes(one.id))
      .map(([group, but]) => [group.map((one) => one.id), but("X")]);
    assert.deepEqual(split(always, ["a", "b"]), [
      [["a", "c", "d"], ""],
      [["b"], ", but only this card while a vote is open in Planning poker"],
    ]);
    assert.deepEqual(split(none("Planning poker"), ["a", "b"]), [
      [["a", "b"], ""],
      [["c"], ", but only X while a vote is open in Planning poker"],
      [["d"], ", while both can be: this card is raised only while no vote is open in Planning poker, and X only " +
        "while a vote is open in this deck"],
    ]);
  });

  test("votes in several decks are named one by one", () => {
    assert.equal(C.decides(none("Planning poker", "Estimates"), vote("Planning poker"), "3"), "the context decides: " +
      "this card is raised only while no vote is open in Planning poker or Estimates, and 3 only while a vote is " +
      "open in Planning poker");
    assert.equal(C.decides(always, none("A", "B", "C"), "Stop"), "the context decides: this card is raised either " +
      "way, and Stop only while no vote is open in A, B, or C");
  });
});

describe("the live line on where hand signals go", () => {
  const clock = (ms) => `at ${ms}`;
  const live = (to, votes = {}) => ({
    decks: [
      { id: "meeting", name: "Meeting signals", vote: null },
      { id: "poker", name: "Planning poker", vote: votes.poker || null },
      { id: "estimates", name: "Estimates", vote: votes.estimates || null },
    ],
    hand_signals: { sign_decks: ["meeting"], count_decks: ["poker", "estimates"], to },
  });

  test("is empty while no decks compete", () => {
    assert.equal(C.routeLine({ decks: [], hand_signals: null }, clock), "");
    assert.equal(C.routeLine(null, clock), "");
  });

  test("names the decks that read signs while no vote is open", () => {
    assert.equal(C.routeLine(live(["meeting"]), clock),
      "Hand signals go to Meeting signals: no vote is open in Planning poker or Estimates.");
  });

  test("names the deck with a vote, and why it is open", () => {
    const room = live(["poker"], { poker: { by: "room", until_ms: null } });
    assert.equal(C.routeLine(room, clock), "Finger counts go to Planning poker: its shared room vote is open until " +
      "the reveal.");
    const press = live(["poker"], { poker: { by: "press", until_ms: 181000 } });
    assert.equal(C.routeLine(press, clock), "Finger counts go to Planning poker: raising its card opened a vote " +
      "until at 181000, or until the card is lowered.");
    const both = live(["poker", "estimates"], { poker: { by: "room" }, estimates: { by: "press", until_ms: 5 } });
    assert.equal(C.routeLine(both, clock), "Finger counts go to Planning poker and Estimates: a vote is open in each.");
  });

  test("shows each time as first converted, so a live region is not announced again for the same vote", () => {
    let offset = 0;
    const clock = C.steadyClock((ms) => `at ${ms + offset}`);
    assert.equal(clock(181000), "at 181000");
    offset = 1;
    assert.equal(clock(181000), "at 181000", "a later snapshot of the same vote");
    assert.equal(clock(300000), "at 300001", "another vote");
  });

  test("lists with a word before the last item", () => {
    assert.equal(C.listed(["A"]), "A");
    assert.equal(C.listed(["A", "B"], "or"), "A or B");
    assert.equal(C.listed(["A", "B", "C"]), "A, B, and C");
  });
});
