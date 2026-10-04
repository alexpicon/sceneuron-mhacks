// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/snapshot.js: preview composition from the draft and live layers, snapshot ordering,
// status labels, and event times.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { liveElement, project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");
const S = require("../editor/snapshot.js");

describe("composePreview", () => {
  test("draft base values with live held and rule fields on top", () => {
    const draft = project();
    draft.elements[0].props.text = "Draft text";
    draft.elements[0].props.x = 5;
    const effect = { effect: "flash", nonce: 3 };
    const live = {
      canvas: { width: 1920, height: 1080 },
      elements: [
        liveElement("brb", "text", { visible: true, text: "Held", x: 99 }, {
          overridden: ["text"],
          automated: ["visible"],
        }),
        liveElement("cam-frame", "frame", { border_width: 8 }, { effect }),
        liveElement("deleted", "box", {}),
      ],
    };
    draft.elements.push({ id: "text-1", name: "New", kind: "text", props: { ...R.DEFAULT_PROPS } });
    const view = S.composePreview(draft, live);
    assert.deepEqual(
      view.elements.map((element) => element.id),
      ["brb", "cam-frame", "text-1"],
    );
    const brb = view.elements[0];
    assert.equal(brb.props.text, "Held");
    assert.equal(brb.props.visible, true);
    assert.equal(brb.props.x, 5, "fields without a live layer come from the draft");
    assert.deepEqual(brb.overridden, ["text"]);
    assert.equal(view.elements[1].effect, effect);
    assert.equal(view.elements[2].effect, null);
    assert.equal(draft.elements[0].props.text, "Draft text", "the draft is not modified");
  });

  test("draft placeholders show live values, or a stand-in before the AI reports any", () => {
    const draft = project();
    draft.conditions = [{ id: "showing-item", name: "Showing something", detail: "the object's name" }];
    draft.elements[0].props.text = "Now showing: {{showing-item}} ({{scene}}){{typo}}";
    const empty = S.composePreview(draft, null);
    assert.equal(empty.elements[0].props.text, "Now showing: [Showing something] ([Scene description])");
    const live = {
      canvas: { width: 1920, height: 1080 },
      scene: { caption: "A person holds a mug." },
      conditions: [{ id: "showing-item", detail: "Coffee mug" }],
      elements: [liveElement("brb", "text", {}), liveElement("cam-frame", "frame", {})],
    };
    const filled = S.composePreview(draft, live).elements[0].props.text;
    assert.equal(filled, "Now showing: Coffee mug (A person holds a mug.)");
    assert.equal(draft.elements[0].props.text, "Now showing: {{showing-item}} ({{scene}}){{typo}}");
  });

  test("text that a rule or hold sets arrives filled in and is kept as is", () => {
    const draft = project();
    draft.elements[0].props.text = "{{scene}}";
    const live = {
      canvas: { width: 1920, height: 1080 },
      elements: [liveElement("brb", "text", { text: "Held {{literally}}" }, { overridden: ["text"] })],
    };
    assert.equal(S.composePreview(draft, live).elements[0].props.text, "Held {{literally}}");
  });

  test("a raised card shows its draft props over rule fields and under held ones", () => {
    const draft = project();
    draft.elements[0].props = { ...draft.elements[0].props, text: "{{scene}}", color: "#ffffff", x: 7 };
    const five = { id: "p5", name: "5", props: { text: "Five {{scene}}", background: "#000000ff" } };
    draft.decks = [{ id: "poker", name: "Poker", element: "brb", cards: [five] }];
    const effective = { visible: true, text: "Saved card", background: "#111111ff", x: 99, color: "#ff0000" };
    const live = {
      canvas: { width: 1920, height: 1080 },
      scene: { caption: "A desk." },
      elements: [liveElement("brb", "text", effective, { automated: ["background", "x"], overridden: ["color"] })],
      decks: [{ id: "poker", element: "brb", raised: "p5", raised_by: "manual", hide_at_ms: null }],
    };
    const props = S.composePreview(draft, live).elements[0].props;
    assert.equal(props.visible, true, "a raised card shows its slot");
    assert.equal(props.text, "Five A desk.", "the draft card's text, with placeholders filled in");
    assert.equal(props.background, "#000000ff", "the card wins over a rule");
    assert.equal(props.x, 99, "rule fields the card leaves alone stay");
    assert.equal(props.color, "#ff0000", "a hold wins over the card");
    live.decks[0].raised = null;
    const lowered = S.composePreview(draft, live).elements[0].props;
    assert.equal(lowered.visible, false, "a lowered deck leaves its slot as the draft has it");
    assert.equal(lowered.text, "A desk.");
    live.decks[0].raised = "p8";
    assert.equal(S.composePreview(draft, live).elements[0].props.visible, false, "a card the draft dropped");
    delete draft.decks;
    live.decks[0].raised = "p5";
    assert.equal(S.composePreview(draft, live).elements[0].props.visible, false, "a deck the draft dropped");
  });

  test("a covered card shows the overlay's cover in the draft's place, never the card", () => {
    const draft = project();
    draft.elements[0].props = { ...draft.elements[0].props, x: 7, color: "#ffffff" };
    const five = { id: "p5", name: "5", props: { text: "5", background: "#f8fafcff" } };
    draft.decks = [{ id: "poker", name: "Poker", element: "brb", cards: [five] }];
    const cover = { visible: true, text: "✓ Voted", background: "#1d4ed8ff", font_size: 60, x: 99 };
    const live = {
      canvas: { width: 1920, height: 1080 },
      elements: [liveElement("brb", "text", cover)],
      decks: [{ id: "poker", element: "brb", raised: "p5", raised_by: "manual", hide_at_ms: null, covered: true }],
    };
    const props = S.composePreview(draft, live).elements[0].props;
    assert.deepEqual(
      [props.visible, props.text, props.background, props.font_size],
      [true, "✓ Voted", "#1d4ed8ff", 60],
      "the cover",
    );
    assert.equal(props.x, 7, "the draft's position, which the streamer may be moving");
    live.decks[0].covered = false;
    assert.equal(S.composePreview(draft, live).elements[0].props.text, "5", "uncovered, the card shows");
  });

  test("text from a rule stays as the server filled it in unless a raised card replaces it", () => {
    const draft = project();
    draft.decks = [{ id: "poker", element: "brb", cards: [{ id: "p5", name: "5", props: { color: "#000000" } }] }];
    const live = {
      canvas: { width: 1920, height: 1080 },
      elements: [liveElement("brb", "text", { text: "Rule {{literally}}" }, { automated: ["text"] })],
      decks: [{ id: "poker", raised: "p5" }],
    };
    assert.equal(S.composePreview(draft, live).elements[0].props.text, "Rule {{literally}}");
  });

  test("falls back to the live snapshot before the project loads", () => {
    const live = {
      canvas: { width: 640, height: 360 },
      theme: "hacker",
      decks: [{ id: "poker", name: "Poker", element: "a", raised: null }],
      elements: [liveElement("a", "box", {})],
    };
    const expected = { canvas: live.canvas, theme: "hacker", decks: live.decks, elements: live.elements };
    const clock = { now_ms: undefined, on_air: undefined };
    assert.deepEqual(S.composePreview(null, live), { ...expected, subtitles: undefined, ...clock });
    assert.deepEqual(S.composePreview(null, null), { canvas: null, elements: [] });
    assert.equal(S.composePreview(project(), null).elements.length, 2);
  });

  test("draws the draft's theme and names its decks' card slots, whatever is live", () => {
    const draft = project();
    draft.theme = "windows-95";
    draft.decks = [{ id: "poker", name: "Planning poker", element: "brb", cards: [{ id: "p5", name: "5", props: {} }] }];
    const live = { canvas: draft.canvas, theme: "none", decks: [], elements: [] };
    const view = S.composePreview(draft, live);
    assert.equal(view.theme, "windows-95");
    assert.deepEqual(view.decks, [{ id: "poker", name: "Planning poker", element: "brb" }]);
    assert.equal(S.composePreview(project(), live).theme, undefined, "a draft without a theme draws none");
  });

  test("marks text that shows live subtitles from the draft, or from the overlay where live text replaces it", () => {
    const draft = project();
    draft.elements[0].props.text = "{{subtitles:1}}";
    const subtitles = { lines: [], live: "", translations: {}, status: { lines: 3, targets: ["es"] } };
    const held = liveElement("brb", "text", { text: "Held" }, { overridden: ["text"] });
    const rule = { ...liveElement("cam-frame", "frame", {}, { automated: ["text"] }), subtitles: true };
    const live = { canvas: draft.canvas, subtitles, elements: [liveElement("brb", "text", {}), rule] };
    const marks = (view) => view.elements.map((element) => element.subtitles);
    assert.deepEqual(marks(S.composePreview(draft, live)), [true, true]);
    assert.equal(S.composePreview(draft, live).subtitles, subtitles, "the lines a language shows");
    assert.deepEqual(marks(S.composePreview(draft, { ...live, elements: [held, rule] })), [false, true]);
    assert.deepEqual(marks(S.composePreview(project(), null)), [false, false]);
    assert.equal(S.composePreview(project(), null).subtitles, null);
  });
});

test("plannedIds marks what a plan adds, changes, acts on, or shows cards in", () => {
  const deck = { id: "votes", name: "Votes", element: "slot", cards: [] };
  const plan = {
    operations: [
      { op: "add_element", element: { id: "new" } },
      { op: "update_element", id: "changed" },
      { op: "add_rule", rule: { actions: [{ element: "acted" }] } },
      { op: "update_rule", rule: { actions: [] } },
      { op: "add_deck", deck },
      { op: "update_deck", deck: { ...deck, element: "other-slot" } },
      { op: "remove_deck", id: "votes" },
      { op: "remove_element", id: "gone" },
    ],
  };
  assert.deepEqual([...S.plannedIds(plan)], ["new", "changed", "acted", "slot", "other-slot"]);
  assert.deepEqual([...S.plannedIds(null)], []);
  assert.deepEqual([...S.plannedIds({})], []);
});

describe("snapshots and labels", () => {
  test("acceptSnapshot keeps the newest revision", () => {
    const at = (revision) => ({ revision, elements: [] });
    assert.ok(S.acceptSnapshot(null, at(1)));
    assert.ok(S.acceptSnapshot(at(4), at(4)), "telemetry keeps the revision");
    assert.ok(S.acceptSnapshot(at(4), at(5)));
    assert.ok(!S.acceptSnapshot(at(5), at(4)));
    assert.ok(!S.acceptSnapshot(at(1), { revision: 2 }));
    assert.ok(!S.acceptSnapshot(at(1), null));
    assert.ok(!S.isSnapshot({ elements: [], revision: "3" }));
  });

  test("presenceLabel shows the state and last score", () => {
    assert.equal(S.presenceLabel({ state: "present", last_score: 0.934 }), "Present, score 0.93");
    assert.equal(S.presenceLabel({ state: "away", last_score: null }), "Away");
    assert.equal(S.presenceLabel({ state: "weird" }), "Unknown");
    assert.equal(S.presenceLabel(null), "Unknown");
  });

  test("visionLabel covers every analysis state", () => {
    assert.equal(S.visionLabel({ state: "idle" }), "Camera idle");
    assert.equal(S.visionLabel({ state: "starting" }), "Camera starting");
    assert.equal(S.visionLabel({ state: "running", latency_ms: 12.6 }), "Camera running, 13 ms");
    assert.equal(S.visionLabel({ state: "running" }), "Camera running");
    assert.equal(S.visionLabel({ state: "failed", message: "model missing" }), "Camera failed: model missing");
    assert.equal(S.visionLabel({ state: "failed", message: "x".repeat(100) }), `Camera failed: ${"x".repeat(77)}...`);
    assert.equal(S.visionLabel({ state: "failed" }), "Camera failed");
    assert.equal(S.visionLabel(null), "Camera idle");
  });

  test("event times convert the server clock to wall time", () => {
    assert.equal(S.eventWallTime(1000, 4000, 1_000_000), 997_000);
    assert.equal(S.eventWallTime(5000, 4000, 1_000_000), 1_000_000, "never in the future");
    assert.equal(S.formatClock(new Date(2026, 9, 1, 9, 5, 7).getTime()), "09:05:07");
  });
});
