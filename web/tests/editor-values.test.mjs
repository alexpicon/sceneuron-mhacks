// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/values.js: color picker and alpha conversion, and number, name, and text validation.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const V = require("../editor/values.js");

describe("color inputs", () => {
  test("splitColor separates the picker value and alpha", () => {
    assert.deepEqual(V.splitColor("#9b5cffcc"), { rgb: "#9b5cff", alpha: 0.8 });
    assert.deepEqual(V.splitColor("#FFF"), { rgb: "#ffffff", alpha: 1 });
    assert.deepEqual(V.splitColor("#0000"), { rgb: "#000000", alpha: 0 });
    assert.deepEqual(V.splitColor("bogus"), { rgb: "#ffffff", alpha: 1 });
  });

  test("joinColor writes #rrggbb when opaque and #rrggbbaa otherwise", () => {
    assert.equal(V.joinColor("#9B5CFF", 1), "#9b5cff");
    assert.equal(V.joinColor("#9b5cff", 0.8), "#9b5cffcc");
    assert.equal(V.joinColor("#000000", 0), "#00000000");
    assert.equal(V.joinColor("#123456", 2), "#123456");
    assert.equal(V.joinColor("#123456", Number.NaN), "#12345600");
    assert.equal(V.joinColor("red", 0.5), "#ffffff80");
  });

  test("sameColor compares channels, not spelling", () => {
    assert.ok(V.sameColor("#9b5cff", "#9b5cffff"));
    assert.ok(V.sameColor("#fff", "#FFFFFF"));
    assert.ok(V.sameColor("#0000", "#00000000"));
    assert.ok(!V.sameColor("#9b5cff", "#9b5cfffe"));
    assert.ok(!V.sameColor("#9b5cff", "#9b5cfe"));
    assert.ok(!V.sameColor("#9b5cff", "nope"));
  });

  test("split and join round-trip every alpha byte", () => {
    for (let byte = 0; byte <= 255; byte += 1) {
      const color = `#0a0b0c${byte.toString(16).padStart(2, "0")}`;
      const parts = V.splitColor(color);
      const joined = V.joinColor(parts.rgb, parts.alpha);
      assert.equal(joined, byte === 255 ? "#0a0b0c" : color);
    }
  });
});

describe("text and number inputs", () => {
  test("parseNumber enforces range and integers", () => {
    assert.equal(V.parseNumber("12.5", 0, 100, false), 12.5);
    assert.equal(V.parseNumber(" 7 ", 0, 100, true), 7);
    assert.equal(V.parseNumber("-10000", -10000, 10000, false), -10000);
    for (const text of ["", "  ", "abc", "1e9", "NaN", "Infinity", "101", "-1", null, undefined]) {
      assert.equal(V.parseNumber(text, 0, 100, false), null, String(text));
    }
    assert.equal(V.parseNumber("7.5", 0, 100, true), null);
  });

  test("names are 1-80 printable characters", () => {
    assert.ok(V.isValidName("Be right back"));
    assert.ok(V.isValidName("x".repeat(80)));
    assert.ok(V.isValidName("\u{1F600}".repeat(80)), "counts characters, not UTF-16 units");
    assert.ok(!V.isValidName(""));
    assert.ok(!V.isValidName("x".repeat(81)));
    assert.ok(!V.isValidName("tab\there"));
    assert.ok(!V.isValidName("bell\u0007"));
    assert.ok(!V.isValidName("c1\u0085"));
  });

  test("text allows newlines but no other control characters", () => {
    assert.ok(V.isValidText(""));
    assert.ok(V.isValidText("line one\nline two"));
    assert.ok(V.isValidText("x".repeat(1000)));
    assert.ok(!V.isValidText("x".repeat(1001)));
    assert.ok(!V.isValidText("tab\t"));
    assert.ok(!V.isValidText("cr\r"));
  });
});
