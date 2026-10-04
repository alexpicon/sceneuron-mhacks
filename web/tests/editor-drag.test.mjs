// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/drag.js: a press in the preview moves its element only after the pointer travels
// 3 CSS pixels, so a click never nudges an element or marks the project unsaved, and a started drag
// follows the pointer in canvas units.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { project } from "./editor-fixtures.mjs";

const require = createRequire(import.meta.url);
const D = require("../editor/drag.js");
const P = require("../editor/project.js");

/** A press at (100, 200) on an element at canvas (1260, 660). */
const press = (moving = false) => ({ startX: 100, startY: 200, x: 1260, y: 660, moving });

describe("dragTarget", () => {
  test("a press that has not traveled 3 CSS pixels does not move the element", () => {
    assert.equal(D.DRAG_THRESHOLD_PX, 3);
    for (const [x, y] of [[100, 200], [101, 201], [102, 202], [97.1, 200], [100, 202.9]]) {
      assert.equal(D.dragTarget(press(), x, y, 0.5), null, `${x}, ${y}`);
    }
  });

  test("traveling 3 CSS pixels or more starts the drag, in canvas units", () => {
    assert.deepEqual(D.dragTarget(press(), 103, 200, 0.5), { x: 1266, y: 660 });
    assert.deepEqual(D.dragTarget(press(), 98, 197.5, 0.25), { x: 1252, y: 650 });
    assert.deepEqual(D.dragTarget(press(), 150, 230, 0.5), { x: 1360, y: 720 });
  });

  test("a started drag follows the pointer back inside the threshold", () => {
    assert.deepEqual(D.dragTarget(press(true), 101, 200, 0.5), { x: 1262, y: 660 });
    assert.deepEqual(D.dragTarget(press(true), 100, 200, 0.5), { x: 1260, y: 660 });
  });

  test("a click with pointer jitter leaves the project unchanged", () => {
    const saved = project();
    const draft = structuredClone(saved);
    const frame = draft.elements.find((element) => element.id === "cam-frame");
    const drag = { startX: 100, startY: 200, x: frame.props.x, y: frame.props.y, moving: false };
    for (const [x, y] of [[100, 200], [101, 200], [102, 201], [101, 202], [100, 200]]) {
      const target = D.dragTarget(drag, x, y, 0.3);
      assert.equal(target, null, `${x}, ${y}`);
    }
    assert.equal(P.isDirty(saved, draft), false);
    const target = D.dragTarget(drag, 103, 204, 0.5);
    assert.equal(P.moveElement(draft, "cam-frame", target.x, target.y), true);
    assert.deepEqual([frame.props.x, frame.props.y], [saved.elements[1].props.x + 6, saved.elements[1].props.y + 8]);
    assert.equal(P.isDirty(saved, draft), true);
  });
});
