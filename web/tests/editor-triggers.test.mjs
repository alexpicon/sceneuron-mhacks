// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/triggers.js: encoding a rule's `when` for the trigger select and back, the select's
// choices, plain-word labels, and finding the rules a condition starts.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const T = require("../editor/triggers.js");

const conditions = [
  { id: "waving", name: "Waving" },
  { id: "arms-up", name: "Arms up" },
];

describe("encoding and decoding", () => {
  test("presence triggers stay strings and condition triggers become state:id", () => {
    assert.equal(T.encodeTrigger("away"), "away");
    assert.equal(T.encodeTrigger("present"), "present");
    assert.equal(T.encodeTrigger({ condition: "waving", state: "on" }), "on:waving");
    assert.equal(T.encodeTrigger({ condition: "arms-up", state: "off" }), "off:arms-up");
  });

  test("decoding inverts encoding", () => {
    for (const when of ["away", "present", { condition: "waving", state: "on" }, { condition: "a:b", state: "off" }]) {
      assert.deepEqual(T.decodeTrigger(T.encodeTrigger(when)), when);
    }
  });

  test("malformed triggers encode as nothing and unknown values do not decode", () => {
    for (const when of [null, "soon", { condition: "x", state: "maybe" }, { condition: "", state: "on" }, {}]) {
      assert.equal(T.encodeTrigger(when), "");
    }
    for (const value of ["", "on:", "maybe:x", "unknown"]) {
      assert.throws(() => T.decodeTrigger(value), /Unknown trigger/);
    }
  });

  test("triggerCondition names the watched condition", () => {
    assert.equal(T.triggerCondition({ condition: "waving", state: "on" }), "waving");
    assert.equal(T.triggerCondition("away"), null);
  });
});

describe("choices and labels", () => {
  test("the select lists presence, then every condition turning on and off", () => {
    const groups = T.triggerChoices(conditions, "away");
    assert.deepEqual(groups, [
      { label: "Camera presence", choices: [["away", "When I leave"], ["present", "When I come back"]] },
      {
        label: "AI conditions",
        choices: [
          ["on:waving", "When Waving turns on"],
          ["off:waving", "When Waving turns off"],
          ["on:arms-up", "When Arms up turns on"],
          ["off:arms-up", "When Arms up turns off"],
        ],
      },
    ]);
  });

  test("without conditions only presence is offered", () => {
    assert.equal(T.triggerChoices([], "present").length, 1);
    assert.equal(T.triggerChoices(undefined, "present").length, 1);
  });

  test("a trigger on a missing condition stays selectable and says so", () => {
    const groups = T.triggerChoices(conditions, { condition: "gone", state: "on" });
    assert.deepEqual(groups.at(-1), {
      label: "Missing",
      choices: [["on:gone", "When a missing condition (gone) turns on"]],
    });
  });

  test("labels read as plain words", () => {
    assert.equal(T.triggerLabel("away", conditions), "When I leave");
    assert.equal(T.triggerLabel("present", conditions), "When I come back");
    assert.equal(T.triggerLabel({ condition: "arms-up", state: "on" }, conditions), "When Arms up turns on");
    assert.equal(T.triggerLabel({ condition: "waving", state: "off" }, conditions), "When Waving turns off");
    assert.equal(T.triggerLabel(42, conditions), "When something unknown happens");
  });
});

test("rulesTriggeredBy finds the rules a condition starts", () => {
  const project = {
    rules: [
      { id: "a", when: { condition: "waving", state: "on" } },
      { id: "b", when: "away" },
      { id: "c", when: { condition: "waving", state: "off" } },
      { id: "d", when: { condition: "arms-up", state: "on" } },
    ],
  };
  assert.deepEqual(T.rulesTriggeredBy(project, "waving").map((rule) => rule.id), ["a", "c"]);
  assert.deepEqual(T.rulesTriggeredBy(project, "nothing"), []);
  assert.deepEqual(T.rulesTriggeredBy(null, "waving"), []);
});
