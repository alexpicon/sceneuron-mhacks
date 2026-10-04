// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/project.js: draft element and rule edits, rule action construction, dirty tracking,
// and the body that saving sends.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");
const C = require("../editor/constants.js");
const P = require("../editor/project.js");

describe("draft element edits", () => {
  test("addElement creates centered elements with unique ids and names", () => {
    const draft = project();
    const text = P.addElement(draft, "text");
    assert.equal(text.id, "text-1");
    assert.equal(text.name, "Text 1");
    assert.equal(text.props.text, "New text");
    assert.equal(text.props.x, Math.round((1920 - text.props.width) / 2));
    assert.equal(text.props.y, Math.round((1080 - text.props.height) / 2));
    assert.deepEqual(Object.keys(text.props).sort(), Object.keys(R.DEFAULT_PROPS).sort());
    assert.equal(P.addElement(draft, "text").id, "text-2");
    const frame = P.addElement(draft, "frame");
    assert.equal(frame.id, "frame-1");
    assert.equal(frame.props.border_width, 8);
    assert.equal(P.addElement(draft, "box").props.background, "#9b5cffcc");
    assert.equal(draft.elements.length, 6);
  });

  test("addElement skips reserved ids and rejects bad kinds and the element limit", () => {
    const draft = project();
    assert.equal(P.addElement(draft, "box", ["box-1", "box-2"]).id, "box-3");
    assert.throws(() => P.addElement(draft, "circle"), /Unknown element kind/);
    while (draft.elements.length < C.MAX_ELEMENTS) {
      P.addElement(draft, "box");
    }
    assert.throws(() => P.addElement(draft, "box"), /at most 64 elements/);
  });

  test("deleteElement removes the element and actions that target it", () => {
    const draft = project();
    assert.equal(P.deleteElement(draft, "brb"), true);
    assert.deepEqual(
      draft.elements.map((element) => element.id),
      ["cam-frame"],
    );
    assert.deepEqual(draft.rules[0].actions, []);
    assert.deepEqual(draft.rules[1].actions, [{ type: "effect", element: "cam-frame", effect: "flash" }]);
    assert.equal(P.deleteElement(draft, "missing"), false);
  });

  test("deleteElement also removes the deck the element shows, like the server's plan edits", () => {
    const draft = project();
    const deck = (id, element) => ({ id, name: id, element, cards: [{ id: "c", name: "C", props: {} }] });
    draft.decks = [deck("votes", "brb"), deck("other", "elsewhere")];
    P.deleteElement(draft, "brb");
    assert.deepEqual(draft.decks.map((kept) => kept.id), ["other"]);
    const plain = project();
    P.deleteElement(plain, "brb");
    assert.equal(plain.decks, undefined, "a project without decks gets none");
  });

  test("canonical sorts object keys at every depth and keeps array order", () => {
    const value = { b: 1, a: { d: [2, { y: 1, x: 2 }], c: null } };
    assert.equal(JSON.stringify(P.canonical(value)), JSON.stringify({ a: { c: null, d: [2, { x: 2, y: 1 }] }, b: 1 }));
    assert.equal(P.canonical("text"), "text");
  });

  test("moveElement rounds, clamps, and reports changes", () => {
    const draft = project();
    assert.equal(P.moveElement(draft, "brb", 10.4, 20.6), true);
    assert.deepEqual([draft.elements[0].props.x, draft.elements[0].props.y], [10, 21]);
    assert.equal(P.moveElement(draft, "brb", 10.2, 20.9), false);
    P.moveElement(draft, "brb", 99999, -99999);
    assert.deepEqual([draft.elements[0].props.x, draft.elements[0].props.y], [10000, -10000]);
    assert.equal(P.moveElement(draft, "missing", 0, 0), false);
  });

  test("findElement tolerates a missing project", () => {
    assert.equal(P.findElement(null, "brb"), null);
    assert.equal(P.findElement(project(), "brb").name, "Be right back");
    assert.equal(P.findElement(project(), "nope"), null);
  });
});

describe("rule edits", () => {
  test("addRule targets the first element and gets a unique id", () => {
    const draft = project();
    const rule = P.addRule(draft);
    assert.deepEqual(rule, {
      id: "rule-1",
      name: "New rule",
      enabled: true,
      when: "away",
      actions: [{ type: "set_props", element: "brb", props: { visible: true } }],
    });
    assert.equal(P.addRule(draft).id, "rule-2");
    assert.equal(P.deleteRule(draft, "rule-1"), true);
    assert.equal(P.addRule(draft).id, "rule-1");
    assert.equal(P.deleteRule(draft, "nope"), false);
  });

  test("addRule without elements has no actions, and the rule limit holds", () => {
    const draft = { ...project(), elements: [], rules: [] };
    assert.deepEqual(P.addRule(draft).actions, []);
    while (draft.rules.length < C.MAX_RULES) {
      P.addRule(draft);
    }
    assert.throws(() => P.addRule(draft), /at most 64 rules/);
  });

  test("makeAction builds every action type; a flood names no element", () => {
    assert.deepEqual(P.makeAction("set_props", "brb"), { type: "set_props", element: "brb", props: { visible: true } });
    assert.deepEqual(P.makeAction("effect", "brb"), { type: "effect", element: "brb", effect: "flash" });
    assert.deepEqual(P.makeAction("flood", "brb"), { type: "flood", flood: "tongue" });
    assert.throws(() => P.makeAction("delete", "brb"), /Unknown action type/);
  });

  test("deleteElement keeps flood actions, which cover the whole overlay", () => {
    const draft = project();
    draft.rules[0].actions.push(P.makeAction("flood"));
    P.deleteElement(draft, "brb");
    assert.deepEqual(draft.rules[0].actions, [{ type: "flood", flood: "tongue" }]);
  });

  test("addAction respects the per-rule limit", () => {
    const rule = { id: "r", name: "r", enabled: true, when: "away", actions: [] };
    while (rule.actions.length < C.MAX_ACTIONS) {
      P.addAction(rule, "brb");
    }
    assert.throws(() => P.addAction(rule, "brb"), /at most 16 actions/);
  });

  test("visibility choices map to the visible field", () => {
    const action = P.makeAction("set_props", "brb");
    assert.equal(P.actionVisibility(action), "show");
    P.setActionVisibility(action, "hide");
    assert.deepEqual(action.props, { visible: false });
    assert.equal(P.actionVisibility(action), "hide");
    P.setActionVisibility(action, "keep");
    assert.deepEqual(action.props, {});
    assert.equal(P.actionVisibility(action), "keep");
    const bare = { type: "set_props", element: "brb" };
    assert.equal(P.actionVisibility(bare), "keep");
    P.setActionVisibility(bare, "show");
    assert.deepEqual(bare.props, { visible: true });
  });

  test("text is set or removed and other fields are reported", () => {
    const action = P.makeAction("set_props", "brb");
    P.setActionText(action, "Back in 5");
    assert.deepEqual(action.props, { visible: true, text: "Back in 5" });
    P.setActionText(action, "");
    assert.equal(action.props.text, "");
    P.setActionText(action, null);
    assert.deepEqual(action.props, { visible: true });
    action.props.color = "#ff0000";
    action.props.x = 4;
    assert.deepEqual(P.otherActionFields(action), ["color", "x"]);
    assert.deepEqual(P.otherActionFields({ type: "effect", element: "brb", effect: "pop" }), []);
  });
});

describe("dirty tracking and saving", () => {
  test("isDirty compares content, not key order or automation mode", () => {
    const saved = project();
    const draft = structuredClone(saved);
    assert.equal(P.isDirty(saved, draft), false);
    draft.automation_enabled = false;
    assert.equal(P.isDirty(saved, draft), false);
    const action = draft.rules[0].actions[0];
    draft.rules[0].actions[0] = { props: action.props, element: action.element, type: action.type };
    assert.equal(P.isDirty(saved, draft), false);
    draft.elements[0].props.text = "Away";
    assert.equal(P.isDirty(saved, draft), true);
    assert.equal(P.isDirty(null, null), false);
  });

  test("projectForSave keeps the server's automation mode", () => {
    const draft = project();
    const payload = P.projectForSave(draft, { automation_enabled: false });
    assert.equal(payload.automation_enabled, false);
    assert.equal(draft.automation_enabled, true, "the draft itself is untouched");
    assert.notEqual(payload.elements, draft.elements);
    assert.equal(P.projectForSave(draft, null).automation_enabled, true);
  });
});
