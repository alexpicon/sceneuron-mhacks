// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/signals-text.js's poses that show two signals as the camera matches them: a sign's own finger count
// on one hand, a sign and the other hand's fingers or another sign on two hands, and a raised hand beside the rest.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { POSES } from "./editor-signals-fixtures.mjs";

const require = createRequire(import.meta.url);
const S = require("../editor/signals-text.js");
const { HELD, SIGN_AND_COUNT, THUMBS_AND_COUNT, THUMBS_HELD, TWO_SIGNS } = POSES;

describe("poses that show two signals", () => {
  const pose = (a, b) => {
    const forward = S.sharedPose(a, b);
    assert.equal(S.sharedPose(b, a), forward, "the same both ways");
    return forward;
  };
  const sign = (name) => ({ sign: name });
  const count = (fingers) => ({ fingers });
  const KNOWN = ["thumbs_up", "thumbs_down", "open_palm", "peace", "pointing", "fist"];

  test("a sign's hand also gives its finger count, which depends on the thumb", () => {
    assert.equal(pose(sign("fist"), count(0)), "A fist also counts as 0 fingers");
    assert.equal(pose(sign("pointing"), count(1)), "A pointing hand also counts as 1 finger");
    assert.equal(pose(sign("pointing"), count(2)), "A pointing hand with the thumb out also counts as 2 fingers");
    assert.equal(pose(sign("peace"), count(2)), "A peace sign also counts as 2 fingers");
    assert.equal(pose(sign("peace"), count(3)), "A peace sign with the thumb out also counts as 3 fingers");
    assert.equal(pose(sign("open_palm"), count(5)), "An open palm also counts as 5 fingers");
    assert.equal(pose(sign("open_palm"), count(4)), "An open palm with the thumb folded also counts as 4 fingers");
  });

  test("on two hands a sign's count and the other hand's 0 to 5 fingers add up, and nothing else makes a count", () => {
    // The counts each sign shows with one hand, and those it makes with a second hand, as the camera adds them up.
    const reach = {
      fist: [[0], [1, 2, 3, 4, 5]],
      pointing: [[1, 2], [3, 4, 5, 6, 7]],
      peace: [[2, 3], [4, 5, 6, 7, 8]],
      open_palm: [[4, 5], [6, 7, 8, 9, 10]],
    };
    for (const [name, [one, two]] of Object.entries(reach)) {
      for (let fingers = 0; fingers <= 10; fingers += 1) {
        const shown = pose(sign(name), count(fingers));
        const what = `${name} and ${fingers}`;
        if (one.includes(fingers)) {
          assert.match(shown, / also counts as /, what);
        } else {
          assert.equal(shown, two.includes(fingers) ? SIGN_AND_COUNT : null, what);
        }
      }
    }
  });

  test("a thumbs up or down adds no fingers, so the other hand's 0 to 5 show beside it", () => {
    for (const thumbs of ["thumbs_up", "thumbs_down"]) {
      for (let fingers = 0; fingers <= 10; fingers += 1) {
        assert.equal(pose(sign(thumbs), count(fingers)), fingers <= 5 ? THUMBS_AND_COUNT : null, `${fingers}`);
      }
    }
  });

  test("a thumbs up or down is a raised hand only with the arm up or the other hand held up", () => {
    for (const thumbs of ["thumbs_up", "thumbs_down"]) {
      assert.equal(pose("hand_raised", sign(thumbs)), THUMBS_HELD);
    }
  });

  test("any other hand held up is a raised hand too, whatever it shows", () => {
    for (const other of [count(0), count(3), count(10), sign("fist"), sign("pointing"), sign("peace")]) {
      assert.equal(pose("hand_raised", other), HELD, JSON.stringify(other));
    }
    assert.equal(pose("hand_raised", sign("open_palm")), HELD);
  });

  test("two different signs show on two hands", () => {
    for (const first of KNOWN) {
      for (const second of KNOWN.filter((name) => name !== first)) {
        assert.equal(pose(sign(first), sign(second)), TWO_SIGNS, `${first} and ${second}`);
      }
      assert.equal(pose(sign(first), sign(first)), null, `${first} twice is the same signal`);
    }
  });

  test("two counts, and a sign this editor does not know, never overlap", () => {
    assert.equal(pose(count(2), count(3)), null);
    assert.equal(pose("hand_raised", sign("rock_on")), null);
    assert.equal(pose(sign("rock_on"), count(2)), null);
    assert.equal(pose(sign("rock_on"), sign("peace")), null);
  });
});
