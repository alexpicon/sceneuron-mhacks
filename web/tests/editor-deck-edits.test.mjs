// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/deck-edits.js: adding packs to the draft and refusing collisions and limits like the
// server, deleting decks with or without their card slot, and deck and card edits, a card's hand signal
// among them, that keep every other field.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");
const C = require("../editor/constants.js");
const D = require("../editor/deck-edits.js");

/** A one-deck pack shaped like the server's, with a hidden text slot. */
function pack(id = "poker", name = "Planning poker") {
  const slot = { id: `${id}-card`, name: "Poker card", kind: "text", props: { ...R.DEFAULT_PROPS, visible: false } };
  const cards = [
    { id: "p3", name: "3", props: { text: "3" }, signal: { fingers: 3 } },
    { id: "unsure", name: "?", props: { text: "?", background: "#f8fafcff" } },
  ];
  return {
    id,
    name,
    description: "Vote on estimates.",
    elements: [slot],
    decks: [{ id, name, element: slot.id, cards, auto_hide_ms: 0, camera: false }],
  };
}

function withPoker() {
  const draft = project();
  D.addPack(draft, pack());
  return draft;
}

describe("addPack", () => {
  test("adds copies of the pack's elements and decks", () => {
    const draft = project();
    const offered = pack();
    const added = D.addPack(draft, offered);
    assert.deepEqual(draft.elements.map((element) => element.id), ["brb", "cam-frame", "poker-card"]);
    assert.deepEqual(draft.decks, offered.decks);
    assert.equal(added[0], draft.decks[0], "returns the decks as the draft holds them");
    draft.decks[0].cards[0].props.text = "Three";
    assert.equal(offered.decks[0].cards[0].props.text, "3", "the offered pack is not changed by later edits");
    D.addPack(draft, pack("meeting", "Meeting signals"));
    assert.deepEqual(draft.decks.map((deck) => deck.id), ["poker", "meeting"]);
  });

  test("refuses a pack the project already has, and an element id another element uses", () => {
    const draft = withPoker();
    assert.throws(() => D.addPack(draft, pack()), { message: "Planning poker is already in this project." });
    const taken = project();
    taken.elements.push({ id: "poker-card", name: "My card", kind: "box", props: {} });
    const message = "Planning poker needs the element id poker-card, which My card already uses. Delete My card " +
      "first to add this pack.";
    assert.throws(() => D.addPack(taken, pack()), { message });
    assert.equal(taken.decks, undefined, "a refused pack changes nothing");
    assert.equal(taken.elements.length, 3);
  });

  test("adds a pack without a deck, such as the subtitles bar, once", () => {
    const draft = project();
    const bars = {
      id: "subtitles",
      name: "Subtitles",
      elements: [
        { id: "subtitles-spoken", name: "Subtitles", kind: "text", props: { text: "{{subtitles}}" } },
        { id: "subtitles-translated", name: "Translated subtitles", kind: "text", props: { text: "{{subtitles:1}}" } },
      ],
      decks: [],
    };
    assert.deepEqual(D.addPack(draft, bars), []);
    assert.deepEqual(draft.elements.map((element) => element.id).slice(-2), ["subtitles-spoken", "subtitles-translated"]);
    assert.throws(() => D.addPack(draft, bars), { message: "Subtitles is already in this project." });
  });

  test("adds the live description bar from the packs once, and finds it when it is there", () => {
    const draft = project();
    const bar = { id: "live-description", name: "Live description", kind: "text", props: { text: "{{seen}}" } };
    const packs = [pack(), { id: "live-description", name: "Live description", elements: [bar], decks: [] }];
    const first = D.addLiveDescription(draft, packs);
    assert.equal(first.added, true);
    assert.deepEqual(first.element, bar);
    assert.notEqual(first.element, bar, "the draft holds a copy");
    assert.equal(draft.elements.at(-1), first.element);
    const count = draft.elements.length;
    const again = D.addLiveDescription(draft, packs);
    assert.deepEqual([again.added, again.element, draft.elements.length], [false, first.element, count]);
    for (const offered of [[pack()], null]) {
      const message = "This Sceneuron offers no live description.";
      assert.throws(() => D.addLiveDescription(project(), offered), { message });
    }
  });

  test("refuses packs over the element and deck limits", () => {
    const full = project();
    while (full.elements.length < C.MAX_ELEMENTS) {
      full.elements.push({ id: `box-${full.elements.length}`, name: "Box", kind: "box", props: {} });
    }
    assert.throws(() => D.addPack(full, pack()), /at most 64 elements/);
    const decks = project();
    decks.decks = Array.from({ length: C.MAX_DECKS }, (_, n) => ({ id: `deck-${n}`, cards: [] }));
    assert.throws(() => D.addPack(decks, pack()), /Planning poker does not fit: a project can have at most 16 decks/);
  });
});

describe("deleting decks", () => {
  test("a deck goes alone, or with its card slot and the rule actions that use it", () => {
    const draft = withPoker();
    const kept = structuredClone(draft);
    assert.equal(D.deleteDeck(draft, "poker", false).name, "Planning poker");
    assert.deepEqual(draft.decks, []);
    assert.deepEqual(draft.elements, kept.elements, "the card slot stays");
    kept.rules[0].actions.push({ type: "effect", element: "poker-card", effect: "pop" });
    D.deleteDeck(kept, "poker", true);
    assert.deepEqual(kept.elements.map((element) => element.id), ["brb", "cam-frame"]);
    assert.deepEqual(kept.rules[0].actions.map((action) => action.element), ["brb"]);
    assert.equal(D.deleteDeck(kept, "poker", true), null);
  });

  test("deckShownBy finds the deck an element shows", () => {
    const draft = withPoker();
    assert.equal(D.deckShownBy(draft, "poker-card").id, "poker");
    assert.equal(D.deckShownBy(draft, "brb"), null);
    assert.equal(D.deckShownBy(project(), "brb"), null);
  });
});

describe("deck and card edits", () => {
  test("auto-hide takes 0 or 1 second to 1 hour, in whole milliseconds", () => {
    const deck = withPoker().decks[0];
    D.setAutoHide(deck, 15000);
    assert.equal(deck.auto_hide_ms, 15000);
    D.setAutoHide(deck, 0);
    assert.equal(deck.auto_hide_ms, 0);
    for (const bad of [500, 3600001, 1500.5, Number.NaN]) {
      assert.throws(() => D.setAutoHide(deck, bad), /until it is lowered/);
    }
    assert.equal(deck.auto_hide_ms, 0);
  });

  test("card edits change only the name, text, and colors, and keep the hand signal", () => {
    const draft = withPoker();
    const [three, unsure] = draft.decks[0].cards;
    D.setCardField(three, "name", "Three");
    D.setCardField(three, "text", "3 points");
    D.setCardField(three, "color", "#ff0000");
    D.setCardField(unsure, "background", "#000000cc");
    assert.deepEqual(three, {
      id: "p3",
      name: "Three",
      props: { text: "3 points", color: "#ff0000" },
      signal: { fingers: 3 },
    });
    assert.deepEqual(unsure.props, { text: "?", background: "#000000cc" });
    assert.throws(() => D.setCardField(three, "visible", true), /cannot set a card's visible/);
    const bare = { id: "bare", name: "Bare" };
    D.setCardField(bare, "text", "Hi");
    assert.deepEqual(bare.props, { text: "Hi" });
  });

  test("trigger phrases and the speech line are set, and an empty value removes them", () => {
    const draft = withPoker();
    const [three] = draft.decks[0].cards;
    D.setCardField(three, "phrases", ["three points", "vote three"]);
    D.setCardField(three, "speech", "Three points");
    assert.deepEqual(three.phrases, ["three points", "vote three"]);
    assert.equal(three.speech, "Three points");
    assert.deepEqual(three.signal, { fingers: 3 }, "the hand signal stays");
    D.setCardField(three, "phrases", []);
    D.setCardField(three, "speech", "");
    assert.equal("phrases" in three, false);
    assert.equal("speech" in three, false);
  });

  test("a hand signal is set as the server stores it, and null removes it", () => {
    const draft = withPoker();
    const [three, unsure] = draft.decks[0].cards;
    D.setCardField(unsure, "signal", { sign: "open_palm" });
    D.setCardField(three, "signal", "hand_raised");
    assert.deepEqual([unsure.signal, three.signal], [{ sign: "open_palm" }, "hand_raised"]);
    D.setCardField(three, "signal", null);
    assert.equal("signal" in three, false, "no signal is no key, as the server saves it");
    assert.deepEqual(three.props, { text: "3" }, "the card's other fields stay");
  });

  test("a card shows its own value, else its slot's, else the default", () => {
    const draft = withPoker();
    const slot = draft.elements[2];
    slot.props.color = "#123456";
    const [three, unsure] = draft.decks[0].cards;
    assert.equal(D.cardValue(unsure, slot, "background"), "#f8fafcff");
    assert.equal(D.cardValue(three, slot, "color"), "#123456");
    assert.equal(D.cardValue(three, null, "background"), R.DEFAULT_PROPS.background);
  });

  test("findDeck, findCard, and isSavedCard tolerate missing parts", () => {
    const saved = withPoker();
    assert.equal(D.findDeck(saved, "poker").name, "Planning poker");
    assert.equal(D.findDeck(null, "poker"), null);
    assert.equal(D.findCard(saved.decks[0], "p3").name, "3");
    assert.equal(D.findCard(null, "p3"), null);
    assert.equal(D.isSavedCard(saved, "poker", "p3"), true);
    assert.equal(D.isSavedCard(saved, "poker", "p13"), false);
    assert.equal(D.isSavedCard(project(), "poker", "p3"), false);
    assert.equal(D.isSavedCard(null, "poker", "p3"), false);
  });
});
