// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/cards-text.js: durations and auto-hide choices, hand signal names, which card a deck
// shows, pack sizes, and the Stream Deck pairing code and countdown.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const T = require("../editor/cards-text.js");

describe("how long cards stay up", () => {
  test("durations read in the largest whole unit", () => {
    assert.equal(T.durationLabel(1000), "1 second");
    assert.equal(T.durationLabel(1500), "1.5 seconds");
    assert.equal(T.durationLabel(15000), "15 seconds");
    assert.equal(T.durationLabel(60000), "1 minute");
    assert.equal(T.durationLabel(90000), "90 seconds");
    assert.equal(T.durationLabel(1800000), "30 minutes");
    assert.equal(T.durationLabel(3600000), "1 hour");
  });

  test("the choices start with until lowered and keep a delay that is not offered", () => {
    const offered = T.autoHideChoices(15000);
    assert.deepEqual(offered[0], [0, "Until it is lowered"]);
    assert.deepEqual(offered.find(([ms]) => ms === 15000), [15000, "15 seconds"]);
    assert.equal(offered.length, 11);
    const custom = T.autoHideChoices(45000).map(([ms]) => ms);
    assert.equal(custom.length, 12);
    assert.equal(custom.indexOf(45000), custom.indexOf(30000) + 1, "in order");
    assert.equal(T.autoHideChoices(undefined).length, 11);
  });
});

test("signals read as what the camera must see", () => {
  assert.equal(T.signalLabel("hand_raised"), "a raised hand");
  assert.equal(T.signalLabel({ fingers: 0 }), "0 fingers up");
  assert.equal(T.signalLabel({ fingers: 1 }), "1 finger up");
  assert.equal(T.signalLabel({ fingers: 8 }), "8 fingers up");
  assert.equal(T.signalLabel({ sign: "thumbs_up" }), "thumbs up");
  assert.equal(T.signalLabel({ sign: "peace" }), "a peace sign");
  assert.equal(T.signalLabel({ sign: "rock_on" }), "rock on", "a sign this editor does not know yet");
  assert.equal(T.signalLabel(undefined), null);
  assert.equal(T.signalLabel(null), null);
});

test("the status names the deck, which card it shows, and who raised it", () => {
  const deck = { name: "Planning poker", cards: [{ id: "p5", name: "5" }] };
  const status = (view) => T.raisedStatus(deck, view);
  assert.equal(status(undefined), "Planning poker is not saved yet. Save to raise these cards live.");
  assert.equal(status({ raised: null }), "Planning poker: no card is up.");
  assert.equal(status({ raised: "p5", raised_by: "manual" }), "Planning poker shows 5.");
  assert.equal(status({ raised: "p5", raised_by: "camera" }), "Planning poker shows 5, raised by the camera.");
  assert.equal(
    status({ raised: "p5", raised_by: "manual", covered: true }),
    "Planning poker shows 5, covered as a hidden vote.",
  );
  assert.equal(status({ raised: "p5", raised_by: "voice" }), "Planning poker shows 5, raised by voice.");
  assert.equal(status({ raised: "p8", raised_by: "manual" }), "Planning poker shows p8.", "a card not in the draft");
});

test("a pack's size counts the cards of all its decks", () => {
  const cards = (n) => ({ cards: Array.from({ length: n }, (_, id) => ({ id })) });
  assert.equal(T.packSize({ decks: [cards(10)] }), "10 cards");
  assert.equal(T.packSize({ decks: [cards(1)] }), "1 card");
  assert.equal(T.packSize({ decks: [], elements: [{}, {}] }), "2 elements", "a pack without cards counts its elements");
  assert.equal(T.packSize({ decks: [cards(2), cards(3)] }), "5 cards");
});

test("the pairing code reads in two groups and the countdown never goes below zero", () => {
  assert.equal(T.groupCode("092156"), "092 156");
  assert.equal(T.groupCode(123), "123", "anything else is shown as it is");
  assert.equal(T.countdown(120000), "2:00");
  assert.equal(T.countdown(118001), "1:59");
  assert.equal(T.countdown(9000), "0:09");
  assert.equal(T.countdown(1), "0:01");
  assert.equal(T.countdown(0), "0:00");
  assert.equal(T.countdown(-5000), "0:00");
});
