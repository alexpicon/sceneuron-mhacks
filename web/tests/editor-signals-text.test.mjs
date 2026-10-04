// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/signals-text.js: the choices of a card's Hand signal list and their values, and what a card's signal
// shares with other cards: refused in its own deck, warned about in other decks whose hand signals are on, and noted
// when one pose shows both, saying where a vote decides which card is raised. editor-signals-poses.test.mjs covers the
// poses themselves, and editor-signals-context.test.mjs how a vote decides.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { POSES } from "./editor-signals-fixtures.mjs";

const require = createRequire(import.meta.url);
const S = require("../editor/signals-text.js");
const { HELD, SIGN_AND_COUNT, THUMBS_AND_COUNT, THUMBS_HELD, TWO_SIGNS } = POSES;

const card = (id, name, signal) => (signal === undefined ? { id, name } : { id, name, signal });

/** Decks shaped like the meeting, poker, and classroom packs; `listening` names the decks whose hand signals are on. */
function project(listening = ["meeting", "poker", "classroom"]) {
  const decks = [
    {
      id: "meeting",
      name: "Meeting signals",
      cards: [
        card("wrap-up", "Wrap it up", { sign: "fist" }),
        card("question", "I have a question", { sign: "pointing" }),
        card("agree", "Agree", { sign: "thumbs_up" }),
        card("cant-hear", "Can't hear you"),
        card("next-topic", "Next topic", { sign: "peace" }),
      ],
    },
    {
      id: "poker",
      name: "Planning poker",
      cards: [0, 1, 2, 3, 5, 8].map((count) => card(`p${count}`, String(count), { fingers: count })),
    },
    {
      id: "classroom",
      name: "Classroom",
      cards: [card("hand", "Hand raised", "hand_raised"), card("got-it", "Got it", { sign: "thumbs_up" })],
    },
  ];
  return { decks: decks.map((deck) => ({ ...deck, camera: listening.includes(deck.id) })) };
}

describe("the Hand signal list", () => {
  test("offers none, a raised hand, 0 to 10 fingers, and the six signs, with plain labels", () => {
    const choices = S.signalChoices(undefined);
    assert.deepEqual(choices.slice(0, 5), [
      ["", "None"],
      ["hand_raised", "Raised hand"],
      ["fingers:0", "0 fingers"],
      ["fingers:1", "1 finger"],
      ["fingers:2", "2 fingers"],
    ]);
    assert.deepEqual(choices[12], ["fingers:10", "10 fingers"]);
    assert.deepEqual(choices.slice(13), [
      ["sign:thumbs_up", "Thumbs up"],
      ["sign:thumbs_down", "Thumbs down"],
      ["sign:open_palm", "Open palm"],
      ["sign:peace", "Peace sign"],
      ["sign:pointing", "Pointing"],
      ["sign:fist", "Fist"],
    ]);
    assert.deepEqual(S.signalChoices({ fingers: 3 }), choices, "an offered signal adds nothing");
  });

  test("keeps a sign it does not know, so the card shows and saves it as it is", () => {
    const choices = S.signalChoices({ sign: "rock_on" });
    assert.equal(choices.length, 20);
    assert.deepEqual(choices.at(-1), ["sign:rock_on", "Rock on"]);
    assert.deepEqual(S.signalFrom("sign:rock_on"), { sign: "rock_on" });
  });

  test("values stand for the signals the server stores", () => {
    assert.equal(S.signalValue(undefined), "");
    assert.equal(S.signalValue(null), "");
    assert.equal(S.signalValue("hand_raised"), "hand_raised");
    assert.equal(S.signalValue({ fingers: 0 }), "fingers:0");
    assert.equal(S.signalValue({ sign: "open_palm" }), "sign:open_palm");
    assert.equal(S.signalFrom(""), null);
    assert.equal(S.signalFrom("hand_raised"), "hand_raised");
    assert.deepEqual(S.signalFrom("fingers:10"), { fingers: 10 });
    assert.deepEqual(S.signalFrom("sign:peace"), { sign: "peace" });
    for (const [value] of S.signalChoices(undefined)) {
      assert.equal(S.signalValue(S.signalFrom(value)), value);
    }
    for (const bad of ["fingers:11", "fingers:-1", "fingers:2.5", "sign:Peace", "sign:", "raised", "3"]) {
      assert.equal(S.signalFrom(bad), undefined, bad);
    }
  });
});

describe("what a card's signal shares with other cards", () => {
  const once = " once this deck's hand signals are on too.";
  const onceFirst = "once this deck's hand signals are on too, ";
  const poker = "a vote in Planning poker decides:";

  test("the same signal on another card of the deck is refused, naming both cards", () => {
    const refusal =
      "Agree already uses a thumbs up in this deck. Choose another hand signal for Can't hear you, or None.";
    const found = S.signalConflicts(project(), "meeting", "cant-hear", { sign: "thumbs_up" });
    assert.equal(found.refusal, refusal);
    assert.deepEqual(S.readSignal(project(), "meeting", "cant-hear", "sign:thumbs_up"), { problem: refusal });
    assert.equal(S.signalConflicts(project(), "meeting", "agree", { sign: "thumbs_up" }).refusal, null, "its own");
  });

  test("a free signal, none, and a value the list never offers are read", () => {
    assert.deepEqual(S.readSignal(project(), "meeting", "cant-hear", "sign:thumbs_down"), {
      value: { sign: "thumbs_down" },
    });
    assert.deepEqual(S.readSignal(project(), "meeting", "agree", ""), { value: null });
    assert.deepEqual(S.readSignal(project(), "meeting", "agree", "fingers:11"), {
      problem: "Choose a hand signal from the list.",
    });
  });

  test("the same signal in another deck whose hand signals are on is a warning naming that deck and card", () => {
    const warning = "Classroom: Got it also uses a thumbs up, and its deck's hand signals are on, so both cards " +
      "would be raised.";
    assert.deepEqual(S.signalConflicts(project(), "meeting", "agree", { sign: "thumbs_up" }).warnings, [warning]);
    const quiet = S.signalConflicts(project(["meeting", "poker"]), "meeting", "agree", { sign: "thumbs_up" });
    assert.deepEqual([quiet.refusal, quiet.warnings], [null, []], "Classroom's hand signals are off");
    const off = S.signalConflicts(project(["classroom"]), "meeting", "agree", { sign: "thumbs_up" });
    assert.deepEqual(off.warnings, [warning.replace(/\.$/, " once this deck's hand signals are on too.")]);
  });

  test("poses that show this card's signal and a listening deck's card are noted, saying where a vote decides", () => {
    const peace = S.signalConflicts(project(), "meeting", "next-topic", { sign: "peace" });
    assert.deepEqual(peace, {
      refusal: null,
      warnings: [],
      notes: [
        "Wrap it up, I have a question, and Agree have a fist, a pointing hand, and a thumbs up. " +
          `${TWO_SIGNS}, so this deck raises one of them then, not Next topic, since they come first.`,
        `Planning poker: 2 has 2 fingers up. A peace sign also counts as 2 fingers, so ${poker} 2 is raised while ` +
          "one is open, and this card while none is.",
        "Planning poker: 3 has 3 fingers up. A peace sign with the thumb out also counts as 3 fingers, so " +
          `${poker} 3 is raised while one is open, and this card while none is.`,
        `Planning poker: 5 and 8 have 5 and 8 fingers up. ${SIGN_AND_COUNT}, so ${poker} one of them is raised ` +
          "while one is open, and this card while none is.",
        `Classroom: Hand raised has a raised hand. ${HELD}, so ${poker} only Hand raised is raised while one is ` +
          "open, and both are raised while none is.",
        `Classroom: Got it has a thumbs up. ${TWO_SIGNS}, so ${poker} only Got it is raised while one is open, and ` +
          "both are raised while none is.",
      ],
    });
    const palm = S.signalConflicts(project(["poker"]), "meeting", "cant-hear", { sign: "open_palm" });
    assert.deepEqual(palm.notes, [
      "Wrap it up, I have a question, Agree, and Next topic have a fist, a pointing hand, a thumbs up, and a peace " +
        `sign. ${TWO_SIGNS}, so this deck raises only whichever of the two comes first.`,
      `Planning poker: 5 has 5 fingers up. An open palm also counts as 5 fingers, so ${onceFirst}${poker} 5 is ` +
        "raised while one is open, and this card while none is.",
      `Planning poker: 8 has 8 fingers up. ${SIGN_AND_COUNT}, so ${onceFirst}${poker} 8 is raised while one is ` +
        "open, and this card while none is.",
      "Hands often open while talking, so an open palm can raise this card by accident.",
    ]);
  });

  test("a sign on one hand and fingers on the other raise a card in each listening deck while a vote is open", () => {
    const draft = project(["poker"]);
    draft.decks[0].cards.push(card("disagree", "Disagree", { sign: "thumbs_down" }));
    // A thumbs down beside 3 fingers raises Disagree, and poker's 3 while a vote is open; with no other hand to add,
    // it never makes 8.
    const down = S.signalConflicts(draft, "meeting", "disagree", { sign: "thumbs_down" });
    assert.deepEqual(down.notes, [
      "Wrap it up, I have a question, and Next topic have a fist, a pointing hand, and a peace sign. " +
        `${TWO_SIGNS}, so this deck raises one of them then, not Disagree, since they come first, but only this ` +
        "card while a vote is open in Planning poker.",
      `Agree has a thumbs up. ${TWO_SIGNS}, so this deck raises only Agree then, since it comes first.`,
      `Planning poker: 0, 1, 2, 3, and 5 have 0, 1, 2, 3, and 5 fingers up. ${THUMBS_AND_COUNT}, so ${onceFirst}` +
        `${poker} this card and one of them are raised while one is open, and only this card is raised while none is.`,
    ]);
    const fist = S.signalConflicts(draft, "meeting", "wrap-up", { sign: "fist" });
    assert.deepEqual(fist.notes, [
      `I have a question and Next topic have a pointing hand and a peace sign. ${TWO_SIGNS}, so this deck raises ` +
        "only Wrap it up then, since it comes first.",
      `Agree and Disagree have a thumbs up and a thumbs down. ${TWO_SIGNS}, so this deck raises only Wrap it up ` +
        "then, since it comes first, but only one of them while a vote is open in Planning poker.",
      `Planning poker: 0 has 0 fingers up. A fist also counts as 0 fingers, so ${onceFirst}${poker} 0 is raised ` +
        "while one is open, and this card while none is.",
      `Planning poker: 1, 2, 3, and 5 have 1, 2, 3, and 5 fingers up. ${SIGN_AND_COUNT}, so ${onceFirst}${poker} ` +
        "one of them is raised while one is open, and this card while none is.",
    ]);
    draft.decks[0].camera = true;
    draft.decks[1].camera = false;
    const here = "a vote in this deck decides:";
    assert.deepEqual(S.signalConflicts(draft, "poker", "p3", { fingers: 3 }).notes, [
      `Meeting signals: Wrap it up and I have a question have a fist and a pointing hand. ${SIGN_AND_COUNT}, so ` +
        `${onceFirst}${here} this card is raised while one is open, and one of them while none is.`,
      `Meeting signals: Agree and Disagree have a thumbs up and a thumbs down. ${THUMBS_AND_COUNT}, so ${onceFirst}` +
        `${here} this card and one of them are raised while one is open, and only one of them is raised while none is.`,
      "Meeting signals: Next topic has a peace sign. A peace sign with the thumb out also counts as 3 fingers, so " +
        `${onceFirst}${here} this card is raised while one is open, and Next topic while none is.`,
    ]);
  });

  test("cards of one deck that the same pose shows share a note, since that deck raises one of them", () => {
    assert.deepEqual(S.signalConflicts(project(), "classroom", "hand", "hand_raised").notes, [
      `Got it has a thumbs up. ${THUMBS_HELD}, so this deck raises only Hand raised then, since it comes first.`,
      "Meeting signals: Wrap it up, I have a question, and Next topic have a fist, a pointing hand, and a peace " +
        `sign. ${HELD}, so ${poker} only this card is raised while one is open, and this card and one of them are ` +
        "raised while none is.",
      `Meeting signals: Agree has a thumbs up. ${THUMBS_HELD}, so both would be raised.`,
      `Planning poker: 0, 1, 2, 3, 5, and 8 have 0, 1, 2, 3, 5, and 8 fingers up. ${HELD}, so ${poker} this card ` +
        "and one of them are raised while one is open, and only this card is raised while none is.",
    ]);
  });

  test("a pose's cards in another deck split by what decides whether they are raised", () => {
    // A thumbs down never waits for a vote, while a fist, pointing, and a peace sign also count as fingers.
    const draft = project();
    const before = S.signalConflicts(draft, "classroom", "got-it", { sign: "thumbs_up" }).notes;
    const meeting = (notes) => notes.filter((note) => note.startsWith("Meeting signals:"));
    assert.deepEqual(meeting(before), [
      "Meeting signals: Wrap it up, I have a question, and Next topic have a fist, a pointing hand, and a peace " +
        `sign. ${TWO_SIGNS}, so ${poker} only this card is raised while one is open, and this card and one of them ` +
        "are raised while none is.",
    ]);
    draft.decks[0].cards.push(card("disagree", "Disagree", { sign: "thumbs_down" }));
    const after = S.signalConflicts(draft, "classroom", "got-it", { sign: "thumbs_up" }).notes;
    assert.deepEqual(meeting(after), [
      ...meeting(before),
      `Meeting signals: Disagree has a thumbs down. ${TWO_SIGNS}, so both would be raised.`,
    ]);
  });

  test("the same sign in two decks that read it waits for no vote in both, and the same count for a vote", () => {
    const draft = project();
    draft.decks[2].cards.push(card("stop", "Stop", { sign: "fist" }));
    assert.deepEqual(S.signalConflicts(draft, "classroom", "stop", { sign: "fist" }).warnings, [
      "Meeting signals: Wrap it up also uses a fist, and its deck's hand signals are on, so a vote in Planning poker " +
        "decides: neither is raised while one is open, and both are raised while none is.",
    ]);
    draft.decks[2].cards.push(card("five", "Five", { fingers: 5 }));
    assert.deepEqual(S.signalConflicts(draft, "classroom", "five", { fingers: 5 }).warnings, [
      "Planning poker: 5 also uses 5 fingers up, and its deck's hand signals are on, so the context decides: this " +
        "card is raised only while a vote is open in this deck, and 5 only while a vote is open in Planning poker.",
    ]);
  });

  test("a pose that shows two signals of one deck raises only the card that comes first", () => {
    const draft = project([]);
    draft.decks[2].cards.push(card("answer-a", "A", { fingers: 1 }), card("answer-b", "B", { fingers: 2 }));
    const later = S.signalConflicts(draft, "classroom", "answer-a", { fingers: 1 });
    assert.deepEqual(later.notes, [
      `Hand raised has a raised hand. ${HELD}, so this deck raises only Hand raised then, since it comes first.`,
      `Got it has a thumbs up. ${THUMBS_AND_COUNT}, so this deck raises only Got it then, since it comes first.`,
    ]);
    const first = S.signalConflicts(draft, "classroom", "hand", "hand_raised");
    assert.equal(
      first.notes[1],
      `A and B have 1 and 2 fingers up. ${HELD}, so this deck raises only Hand raised then, since it comes first.`,
    );
  });

  test("no signal, or a deck the project lacks, shares nothing", () => {
    const none = { refusal: null, warnings: [], notes: [] };
    assert.deepEqual(S.signalConflicts(project(), "meeting", "cant-hear", null), none);
    assert.deepEqual(S.signalConflicts(project(), "status", "brb", { sign: "fist" }), none);
    assert.deepEqual(S.signalConflicts(null, "meeting", "agree", { sign: "fist" }), none);
  });
});
