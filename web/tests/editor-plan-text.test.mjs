// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/plan-text.js: every kind of edit plan operation in plain words, with names looked up in
// the project the plan was made for and the project it produces.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const T = require("../editor/plan-text.js");

const before = {
  elements: [
    { id: "name-label", name: "Name label", kind: "text", props: { text: "Sceneuron" } },
    { id: "hype", name: "Hype", kind: "text", props: { text: "LET'S GO" } },
  ],
  conditions: [{ id: "pet", name: "Pet on camera", prompt: "A pet is visible.", detail: "", enabled: true }],
  rules: [{ id: "old", name: "Old rule", when: "away", actions: [] }],
};
const after = {
  elements: before.elements,
  conditions: before.conditions.concat({ id: "arms-up", name: "Arms up", prompt: "Both arms go up." }),
  rules: [],
};
const sentence = (op) => {
  const line = T.describeOperation(op, before, after);
  return `${line.verb} ${line.text}`;
};

describe("describeOperation", () => {
  test("conditions", () => {
    const waving = { id: "waving", name: "Waving", prompt: "The person waves at the camera." };
    assert.equal(
      sentence({ op: "add_condition", condition: waving }),
      "Add condition Waving: The person waves at the camera",
    );
    const tailored = { ...before.conditions[0], prompt: "A cat is on the desk.", detail: "the animal" };
    assert.equal(
      sentence({ op: "update_condition", condition: tailored }),
      "Change condition Pet on camera: what to watch for, what to report (now “A cat is on the desk.”)",
    );
    assert.equal(
      sentence({ op: "remove_condition", id: "pet" }),
      "Remove condition Pet on camera and the rules it starts",
    );
  });

  test("elements", () => {
    const element = { id: "w", name: "Wave hello", kind: "text", props: { text: "Hi chat!", visible: false } };
    assert.equal(
      sentence({ op: "add_element", element }),
      "Add text element Wave hello: “Hi chat!”, hidden until a rule shows it",
    );
    const box = { id: "b", name: "Glow", kind: "box", props: { visible: true } };
    assert.equal(sentence({ op: "add_element", element: box }), "Add box element Glow");
    assert.equal(
      sentence({ op: "update_element", id: "name-label", props: { text: "Live", background: "#ff5c8aee" } }),
      "Change Name label: text, color (text “Live”)",
    );
    assert.equal(
      sentence({ op: "update_element", id: "hype", name: "Big hype", props: { x: 1, y: 2 } }),
      "Change Hype: rename to Big hype, position",
    );
    assert.equal(
      sentence({ op: "remove_element", id: "hype" }),
      "Remove element Hype and the rule actions that use it",
    );
  });

  test("rules read as trigger and actions, with 'it' for the same element", () => {
    const rule = {
      id: "hype-on",
      name: "Hype on arms up",
      when: { condition: "arms-up", state: "on" },
      actions: [
        { type: "set_props", element: "hype", props: { visible: true } },
        { type: "effect", element: "hype", effect: "shake" },
      ],
    };
    assert.equal(
      sentence({ op: "add_rule", rule }),
      "Add rule Hype on arms up: when Arms up turns on, show Hype and shake it",
    );
    const leave = { ...rule, name: "Away", when: "away", actions: [] };
    assert.equal(sentence({ op: "update_rule", rule: leave }), "Change rule Away: when I leave, do nothing");
    const flood = { ...leave, name: "Flood", actions: [{ type: "flood", flood: "tongue" }, rule.actions[0]] };
    assert.equal(
      sentence({ op: "add_rule", rule: flood }),
      "Add rule Flood: when I leave, flood the screen with emojis and show Hype",
    );
    assert.equal(sentence({ op: "remove_rule", id: "old" }), "Remove rule Old rule");
  });

  test("decks, their card slots, and removing a slot", () => {
    const card = (id, name = id) => ({ id, name, props: { text: name } });
    const votes = {
      id: "votes",
      name: "Votes",
      element: "slot",
      auto_hide_ms: 0,
      cards: [card("yes", "Yes"), card("no", "No"), card("maybe", "Maybe")],
    };
    const poker = {
      id: "poker",
      name: "Planning poker",
      element: "poker-card",
      auto_hide_ms: 0,
      cards: ["0", "1", "2", "3", "5", "8", "13", "21", "?", "☕"].map((value) => card(`p${value}`, value)),
    };
    const slot = { id: "slot", name: "Vote slot", kind: "text", props: { text: "", visible: false } };
    const pokerSlot = { id: "poker-card", name: "Poker card", kind: "text", props: { text: "", visible: false } };
    const withDecks = { ...before, elements: before.elements.concat(slot), decks: [votes] };
    const planned = { ...withDecks, elements: withDecks.elements.concat(pokerSlot), decks: [votes, poker] };
    const say = (op) => {
      const line = T.describeOperation(op, withDecks, planned);
      return `${line.verb} ${line.text}`;
    };
    assert.equal(
      say({ op: "add_element", element: pokerSlot }),
      "Add text element Poker card to show the Planning poker cards",
    );
    assert.equal(
      say({ op: "add_deck", deck: poker }),
      "Add deck Planning poker on Poker card: 10 cards (0, 1, 2, 3, 5, 8, 13, and 3 more), each shown until it is " +
        "lowered",
    );
    const signals = { ...votes, id: "signals", name: "Signals", cards: [card("wrap", "Wrap it up")] };
    assert.equal(
      say({ op: "add_deck", deck: { ...signals, auto_hide_ms: 15000, camera: true } }),
      "Add deck Signals on Vote slot: 1 card (Wrap it up), each shown for 15 seconds at a time, and hand signals " +
        "can raise them",
    );
    const replaced = {
      ...votes,
      name: "Quick votes",
      cards: [{ ...card("yes", "Yes"), props: { text: "Aye" } }, card("no", "No"), card("abstain", "Abstain")],
      auto_hide_ms: 120000,
      camera: true,
    };
    assert.equal(
      say({ op: "update_deck", deck: replaced }),
      "Change deck Votes: rename to Quick votes; add card Abstain; change card Yes; remove card Maybe; " +
        "show each card for 2 minutes at a time; turn hand signals on",
    );
    assert.equal(
      say({ op: "update_deck", deck: { ...votes, auto_hide_ms: 1500 } }),
      "Change deck Votes: show each card for 1.5 seconds at a time",
    );
    assert.equal(say({ op: "update_deck", deck: votes }), "Change deck Votes: nothing");
    assert.equal(
      say({ op: "remove_deck", id: "votes" }),
      "Remove deck Votes and its cards, keeping the element Vote slot",
    );
    const withoutSlot = { ...withDecks, elements: before.elements, decks: [] };
    const removed = T.describeOperation({ op: "remove_element", id: "slot" }, withDecks, withoutSlot);
    assert.equal(
      `${removed.verb} ${removed.text}`,
      "Remove element Vote slot, the rule actions that use it, and the Votes deck it shows",
    );
    const unknown = T.describeOperation({ op: "remove_element", id: "slot" }, withDecks, undefined);
    assert.equal(unknown.text, removed.text, "without the result, the deck goes with its element");
    const kinds = ["add_deck", "update_deck", "remove_deck"].map(
      (op) => T.describeOperation({ op, id: "votes", deck: votes }, withDecks, planned).kind,
    );
    assert.deepEqual(kinds, ["add", "change", "remove"]);
  });

  test("a plan that moves a deck to another element keeps the deck and drops the old slot", () => {
    const card = (id, name) => ({ id, name, props: { text: name } });
    const votes = { id: "votes", name: "Votes", element: "slot", auto_hide_ms: 0, cards: [card("yes", "Yes")] };
    const slot = { id: "slot", name: "Vote slot", kind: "text", props: { text: "", visible: false } };
    const bigSlot = { id: "big-slot", name: "Big slot", kind: "text", props: { text: "", visible: false } };
    const current = { ...before, elements: before.elements.concat(slot), decks: [votes] };
    const moved = { ...votes, element: "big-slot" };
    const plan = {
      operations: [
        { op: "add_element", element: bigSlot },
        { op: "remove_deck", id: "votes" },
        { op: "add_deck", deck: moved },
        { op: "remove_element", id: "slot" },
      ],
      project: { ...before, elements: before.elements.concat(bigSlot), decks: [moved] },
    };
    assert.deepEqual(T.describePlan(plan, current).map((line) => `${line.verb} ${line.text}`), [
      "Add text element Big slot to show the Votes cards",
      "Remove deck Votes and its cards",
      "Add deck Votes on Big slot: 1 card (Yes), each shown until it is lowered",
      "Remove element Vote slot and the rule actions that use it",
    ]);
  });

  test("a card whose properties are listed in another order is unchanged", () => {
    const saved = { id: "yes", name: "Yes", props: { background: "#16a34af2", text: "Yes" }, signal: "hand_raised" };
    const votes = { id: "votes", name: "Votes", element: "slot", auto_hide_ms: 0, camera: true, cards: [saved] };
    const project = { ...before, decks: [votes] };
    const planned = { ...saved, signal: "hand_raised", props: { text: "Yes", background: "#16a34af2" } };
    const update = (card) => {
      const line = T.describeOperation({ op: "update_deck", deck: { ...votes, cards: [card] } }, project, project);
      return `${line.verb} ${line.text}`;
    };
    assert.equal(update(planned), "Change deck Votes: nothing");
    const recolored = { ...planned, props: { text: "Yes", background: "#000000" } };
    assert.equal(update(recolored), "Change deck Votes: change card Yes");
  });

  test("kinds mark additions, changes, and removals", () => {
    const kinds = ["add_rule", "update_rule", "remove_rule"].map((op) =>
      T.describeOperation({ op, id: "old", rule: { ...before.rules[0], actions: [] } }, before, after).kind
    );
    assert.deepEqual(kinds, ["add", "change", "remove"]);
    assert.equal(T.describeOperation({ op: "teleport" }, before, after).kind, "change");
  });
});

describe("describeActions", () => {
  test("texts, other fields, and unknown elements", () => {
    const actions = [
      { type: "set_props", element: "name-label", props: { text: "Back soon" } },
      { type: "set_props", element: "name-label", props: { visible: false, color: "#000" } },
      { type: "set_props", element: "gone", props: { opacity: 0.5 } },
      { type: "effect", element: "hype", effect: "pop" },
    ];
    assert.equal(
      T.describeActions(actions, before),
      "set Name label's text to “Back soon”, hide it and change its text color, change gone's opacity, " +
        "and pop Hype",
    );
  });

  test("long texts are shortened", () => {
    const text = "x".repeat(80);
    const phrase = T.describeActions([{ type: "set_props", element: "hype", props: { text } }], before);
    assert.ok(phrase.endsWith("...”"), phrase);
  });
});

test("describePlan describes every operation against the plan's own result", () => {
  const plan = {
    operations: [{ op: "add_condition", condition: after.conditions[1] }, { op: "remove_rule", id: "old" }],
    project: after,
  };
  assert.deepEqual(T.describePlan(plan, before).map((line) => line.kind), ["add", "remove"]);
  assert.equal(T.joinWords(["a", "b", "c"]), "a, b, and c");
});
