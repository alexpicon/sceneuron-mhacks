// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/condition-edits.js: condition ids made from names, adding, renaming with the ids that
// follow names until saved, removing with the rules a condition starts, and one-line field checks.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const C = require("../editor/constants.js");
const E = require("../editor/condition-edits.js");

/** The fixture project with one condition, a rule on it, and texts that show its detail. */
function withCondition() {
  const draft = project();
  const waving = { id: "waving", name: "Waving", prompt: "The person waves.", detail: "" };
  draft.conditions = [{ ...waving, ...C.CONDITION_DEFAULTS }];
  draft.rules.push({
    id: "hello",
    name: "Say hi",
    enabled: true,
    when: { condition: "waving", state: "on" },
    actions: [{ type: "set_props", element: "brb", props: { visible: true, text: "Hi {{ waving }}!" } }],
  });
  draft.elements[0].props.text = "{{waving}} and {{scene}}";
  return draft;
}

describe("conditionId", () => {
  test("lowercases names into letters, digits, and single hyphens", () => {
    assert.equal(E.conditionId("Arms up!", []), "arms-up");
    assert.equal(E.conditionId("  Showing a Product  ", []), "showing-a-product");
    assert.equal(E.conditionId("Café crème", []), "cafe-creme");
    assert.equal(E.conditionId("Level 2 -- boss", []), "level-2-boss");
  });

  test("is unique, never a reserved placeholder, and never empty", () => {
    assert.equal(E.conditionId("Waving", ["waving"]), "waving-2");
    assert.equal(E.conditionId("Waving", ["waving", "waving-2"]), "waving-3");
    assert.equal(E.conditionId("Scene", []), "scene-2");
    assert.equal(E.conditionId("Seen", []), "seen-2");
    assert.equal(E.conditionId("Room", []), "room-2");
    assert.equal(E.conditionId("!!!", []), "condition");
    assert.equal(E.conditionId("", ["condition"]), "condition-2");
  });

  test("stays within the id length limit, suffix included", () => {
    const long = "a very long condition name that keeps going and going";
    const id = E.conditionId(long, []);
    assert.ok(id.length <= C.MAX_ID_CHARS, id);
    assert.ok(!id.endsWith("-"));
    const next = E.conditionId(long, [id]);
    assert.ok(next.length <= C.MAX_ID_CHARS && next.endsWith("-2"), next);
    assert.match(next, /^[a-z0-9]+(-[a-z0-9]+)*$/);
  });
});

describe("adding and renaming", () => {
  test("addCondition appends defaults with an empty prompt, up to the limit", () => {
    const draft = project();
    const condition = E.addCondition(draft);
    assert.deepEqual(condition, {
      id: "new-condition",
      name: "New condition",
      prompt: "",
      detail: "",
      enabled: true,
      min_confidence: 0.6,
      confirm: 1,
      release: 2,
    });
    assert.equal(E.addCondition(draft).id, "new-condition-2");
    while (draft.conditions.length < C.MAX_CONDITIONS) {
      E.addCondition(draft);
    }
    assert.throws(() => E.addCondition(draft), /at most 16 conditions/);
  });

  test("an unsaved condition's id follows its name, and rules and texts follow the id", () => {
    const draft = withCondition();
    const condition = draft.conditions[0];
    assert.equal(E.renameCondition(draft, condition, "Hand wave", []), "hand-wave");
    assert.equal(condition.name, "Hand wave");
    assert.deepEqual(draft.rules.at(-1).when, { condition: "hand-wave", state: "on" });
    assert.equal(draft.rules.at(-1).actions[0].props.text, "Hi {{hand-wave}}!");
    assert.equal(draft.elements[0].props.text, "{{hand-wave}} and {{scene}}");
  });

  test("a saved condition keeps its id when renamed", () => {
    const draft = withCondition();
    assert.equal(E.renameCondition(draft, draft.conditions[0], "Hand wave", ["waving"]), "waving");
    assert.equal(draft.conditions[0].name, "Hand wave");
    assert.equal(draft.elements[0].props.text, "{{waving}} and {{scene}}");
  });

  test("a rename never takes another condition's id", () => {
    const draft = withCondition();
    const other = E.addCondition(draft, "Other");
    assert.equal(E.renameCondition(draft, other, "Waving", []), "waving-2");
  });
});

describe("removing", () => {
  test("removeCondition drops the rules it starts, which could not stay valid", () => {
    const draft = withCondition();
    const removed = E.removeCondition(draft, "waving");
    assert.deepEqual(removed.map((rule) => rule.id), ["hello"]);
    assert.deepEqual(draft.conditions, []);
    assert.deepEqual(draft.rules.map((rule) => rule.id), ["away-show-brb", "present-hide-brb"]);
    assert.deepEqual(E.removeCondition(draft, "waving"), []);
  });

  test("placeholderUsers names the elements and rules whose text shows the detail", () => {
    const draft = withCondition();
    assert.deepEqual(E.placeholderUsers(draft, "waving"), ["Be right back", "Say hi"]);
    assert.deepEqual(E.placeholderUsers(draft, "scene"), ["Be right back"]);
    assert.deepEqual(E.placeholderUsers(draft, "nothing"), []);
  });
});

describe("field checks", () => {
  test("prompts need 1 to 300 characters on one line", () => {
    assert.ok(E.isValidPrompt("The person waves."));
    assert.ok(!E.isValidPrompt(""));
    assert.ok(!E.isValidPrompt("x".repeat(301)));
    assert.ok(E.isValidPrompt("x".repeat(300)));
    assert.ok(!E.isValidPrompt("two\nlines"));
  });

  test("details may be empty, up to 120 characters on one line", () => {
    assert.ok(E.isValidDetail(""));
    assert.ok(E.isValidDetail("the object's name"));
    assert.ok(!E.isValidDetail("y".repeat(121)));
    assert.ok(!E.isValidDetail("tab\there"));
  });

  test("oneLine joins pasted lines with single spaces", () => {
    assert.equal(E.oneLine("A cat\n  is on\r\nthe desk"), "A cat is on the desk");
    assert.equal(E.oneLine("unchanged"), "unchanged");
  });
});
