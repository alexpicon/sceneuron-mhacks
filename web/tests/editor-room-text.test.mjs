// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/room-text.js: the Room section's status line, each person's line before and after the
// reveal, this Sceneuron's own hidden vote, the revealed spread, and the checks of a typed name and code.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const T = require("../editor/room-text.js");

const open = {
  joined: true,
  code: "K7M2QX",
  round: 2,
  revealed: false,
  deck: "poker",
  participants: [
    { id: "a1", name: "Ana", voted: true, me: true },
    { id: "b2", name: "Ben", voted: false, me: false },
    { id: "c3", name: "Cleo", voted: true, me: false },
  ],
  results: [],
  my_vote: { card: "p5", label: "5" },
  summary: "Voted: 2 of 3",
};

const revealed = {
  ...open,
  revealed: true,
  my_vote: null,
  results: [
    { id: "a1", name: "Ana", card: "p5", label: "5", me: true },
    { id: "c3", name: "Cleo", card: "p8", label: "8", me: false },
  ],
  summary: "5, 8",
};

test("the status names the room and round, then who voted or that the votes are revealed", () => {
  assert.equal(T.roomStatus(null), "Not in a room.");
  assert.equal(T.roomStatus({ joined: false }), "Not in a room.");
  assert.equal(T.roomStatus(open), "Room K7M2QX, round 2: 2 of 3 voted.");
  assert.equal(T.roomStatus(revealed), "Room K7M2QX, round 2: the votes are revealed.");
});

test("each person shows whether they voted, then what", () => {
  assert.deepEqual(open.participants.map((person) => T.personLine(person, open)), [
    "Ana (you): voted",
    "Ben: not yet",
    "Cleo: voted",
  ]);
  assert.deepEqual(revealed.participants.map((person) => T.personLine(person, revealed)), [
    "Ana (you): 5",
    "Ben: no vote",
    "Cleo: 8",
  ]);
});

test("two people with one name each see their own card, by their id", () => {
  const alexes = {
    ...revealed,
    participants: [
      { id: "x1", name: "Alex", voted: false, me: false },
      { id: "x2", name: "Alex", voted: true, me: false },
    ],
    results: [{ id: "x2", name: "Alex", card: "p13", label: "13", me: false }],
  };
  assert.deepEqual(alexes.participants.map((person) => T.personLine(person, alexes)), [
    "Alex: no vote",
    "Alex: 13",
  ]);
});

test("only this Sceneuron's editor names its hidden vote", () => {
  assert.equal(T.myVoteText(open, "Planning poker"), "Your hidden vote: 5. Raise another card to change it.");
  assert.equal(
    T.myVoteText({ ...open, my_vote: null }, "Planning poker"),
    "Raise a card of Planning poker to vote. Your overlay shows it covered until the reveal.",
  );
  assert.equal(T.myVoteText(revealed, "Planning poker"), "");
  assert.equal(T.myVoteText({ joined: false }, "Planning poker"), "");
});

test("the spread reads like the {{room}} placeholder", () => {
  assert.equal(T.spreadText(revealed), "Revealed: 5, 8.");
  assert.equal(T.spreadText({ ...revealed, results: [], summary: "No votes" }), "Nobody voted.");
  assert.equal(T.spreadText(open), "");
});

test("names and codes are checked as the room checks them", () => {
  assert.equal(T.isDisplayName("  Ana "), true);
  assert.equal(T.isDisplayName("   "), false);
  assert.equal(T.isDisplayName("é".repeat(40)), true);
  assert.equal(T.isDisplayName("é".repeat(41)), false);
  assert.equal(T.isDisplayName("A\u0007"), false);
  assert.equal(T.normalizeCode(" k7m2qx "), "K7M2QX");
  assert.equal(T.isRoomCode(" k7m2qx "), true);
  assert.equal(T.isRoomCode("chaski"), true);
  for (const code of ["", "ABC", "ABCDEFGHJKMNP", "K7-2QX"]) {
    assert.equal(T.isRoomCode(code), false, code);
  }
});
