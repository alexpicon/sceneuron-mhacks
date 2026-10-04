// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises what editor/snapshot.js gives the preview for the Graphite theme: the on-air pill and timer marked as
// the server marks them, the live clock and program the time on air counts from, and the cards the live decks
// raised, with when they went up, their cover, and the room's summary, so the preview draws tiles as the overlay
// does. The rest of composePreview is in editor-snapshot.test.mjs.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";

import { liveElement, project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");
const S = require("../editor/snapshot.js");

test("marks the on-air pill and timer, and brings the live clock, program, and raised cards", () => {
  const draft = project();
  draft.elements[0].props.text = "{{on-air}}";
  const text = (id, words) => ({ id, name: id, kind: "text", props: { ...R.DEFAULT_PROPS, text: words } });
  draft.elements.push(text("timer", "{{on-air-time}}"), text("slot", ""));
  draft.decks = [{ id: "poker", name: "Poker", element: "slot", cards: [{ id: "p5", name: "5", props: {} }] }];
  const on_air = { scene: "Scene 01", streaming: true, recording: false, since_ms: 1000 };
  const raised = { raised: "p5", raised_at_ms: 4000, covered: true, summary: "Voted: 3 of 5" };
  const live = {
    now_ms: 5000,
    on_air,
    canvas: { width: 1920, height: 1080 },
    elements: [liveElement("brb", "text", {})],
    decks: [{ id: "poker", name: "Poker", element: "slot", raised_by: "manual", ...raised }],
  };
  const view = S.composePreview(draft, live);
  assert.deepEqual([view.now_ms, view.on_air], [5000, on_air]);
  const flags = view.elements.map((element) => [element.id, element.on_air, element.clock]);
  assert.deepEqual(flags.slice(0, 3), [["brb", true, false], ["cam-frame", false, false], ["timer", false, true]]);
  assert.equal(view.elements[0].props.text, "Scene 01 · LIVE");
  assert.equal(view.elements[2].props.text, "{{on-air-time}}", "the renderer fills in the time");
  assert.deepEqual(view.decks, [{ id: "poker", name: "Poker", element: "slot", ...raised }]);
  live.decks[0].raised = "p8";
  assert.deepEqual(S.composePreview(draft, live).decks, [{ id: "poker", name: "Poker", element: "slot" }],
    "a card the draft dropped is not raised");
});
