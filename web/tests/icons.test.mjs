// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises icons.js, the line icons the overlay's themes and the Stream Deck plugin share: every drawing is plain
// paths, circles, and rectangles inside the 24 by 24 box, every icon a card or element names exists, and how a
// built-in card, a custom card, and a starter element choose their icon and lose the emoji it replaces.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const I = require("../icons.js");

describe("drawings", () => {
  test("every icon is paths, circles, and rectangles with numbers inside the box", () => {
    assert.ok(Object.keys(I.ICONS).length >= 30);
    for (const [name, shapes] of Object.entries(I.ICONS)) {
      assert.ok(shapes.length > 0, name);
      for (const [tag, attributes] of shapes) {
        assert.ok(["path", "circle", "rect"].includes(tag), `${name}: ${tag}`);
        if (tag === "path") {
          assert.match(attributes.d, /^M[\d.\-\s,MmLlHhVvAaCcSsZz]+$/, name);
          continue;
        }
        for (const value of Object.values(attributes)) {
          assert.ok(Number.isFinite(value) && value >= 0 && value <= 24, `${name}: ${tag} ${value}`);
        }
      }
    }
  });

  test("markup draws the shapes without stroke attributes, which each user adds", () => {
    assert.equal(I.iconMarkup("check"), '<path d="M5 12.5l4.5 4.5L19 7.5"/>');
    assert.equal(I.iconMarkup("person"), '<circle cx="12" cy="8" r="4"/><path d="M4.5 20.5a7.5 7.5 0 0 1 15 0"/>');
    assert.equal(I.iconMarkup("lower").startsWith('<rect x="6" y="3" width="12" height="9.5" rx="2"/>'), true);
    assert.equal(I.iconMarkup("no-such-icon"), "");
    assert.equal(I.iconMarkup("toString"), "", "only the icons' own names");
    assert.equal(I.STROKE_WIDTH, 1.75);
  });

  test("every icon a card, an element, or the fallback names is drawn", () => {
    const named = [I.FALLBACK, ...Object.values(I.ELEMENTS)];
    for (const cards of Object.values(I.CARDS)) {
      named.push(...Object.values(cards).filter(Boolean).map(([icon]) => icon));
    }
    for (const name of named) {
      assert.ok(name in I.ICONS, name);
    }
  });
});

describe("leadingEmoji", () => {
  test("takes a whole first emoji, joined or flagged, and the words after it", () => {
    assert.deepEqual(I.leadingEmoji("⏱️ Wrap it up"), { emoji: "⏱️", rest: "Wrap it up" });
    assert.deepEqual(I.leadingEmoji("  👩🏽‍💻 Coding"), { emoji: "👩🏽‍💻", rest: "Coding" });
    assert.deepEqual(I.leadingEmoji("🇯🇵 Japan"), { emoji: "🇯🇵", rest: "Japan" });
    assert.deepEqual(I.leadingEmoji("☕"), { emoji: "☕", rest: "" });
    for (const text of ["Wrap it up", "✓ Voted", "13", "", "A 👍"]) {
      assert.equal(I.leadingEmoji(text), null, text);
    }
  });
});

describe("cardIcon", () => {
  test("a built-in card shows its own icon and short label, without the emoji in its words", () => {
    assert.deepEqual(I.cardIcon("meeting", "wrap-up", "⏱️ Wrap it up"), {
      icon: "timer",
      emoji: null,
      text: "Wrap it up",
      label: "Wrap",
    });
    assert.deepEqual(I.cardIcon("status", "muted", "I'm on mute"), {
      icon: "mic-off",
      emoji: null,
      text: "I'm on mute",
      label: "Mute",
    });
    assert.equal(I.cardIcon("poker", "break", "☕").text, "", "the icon is the whole face");
  });

  test("a built-in value card is its words, without an icon", () => {
    const values = [["poker", "p13", "13"], ["poker", "unsure", "?"], ["classroom", "answer-a", "A"]];
    for (const [deck, card, text] of values) {
      assert.deepEqual(I.cardIcon(deck, card, text), { icon: null, emoji: null, text, label: text }, card);
    }
  });

  test("a custom card shows its leading emoji, its words alone when short, or the fallback icon", () => {
    assert.deepEqual(I.cardIcon("party", "cake", "🎂 Cake time"), {
      icon: null,
      emoji: "🎂",
      text: "Cake time",
      label: "Cake time",
    });
    assert.deepEqual(I.cardIcon("party", "n", "42"), { icon: null, emoji: null, text: "42", label: "42" });
    assert.deepEqual(I.cardIcon("meeting", "pizza", "Pizza is here"), {
      icon: I.FALLBACK,
      emoji: null,
      text: "Pizza is here",
      label: "Pizza is here",
    });
    assert.equal(I.cardIcon("constructor", "x", "Shipped it").icon, I.FALLBACK, "only the packs' own decks");
  });
});

describe("elementIcon", () => {
  test("a starter element shows its icon, and any other text its leading emoji or nothing", () => {
    assert.deepEqual(I.elementIcon("name-label", "Alex Picon"), { icon: "person", emoji: null, text: "Alex Picon" });
    assert.deepEqual(I.elementIcon("now-showing", "📦 Now showing: mug"), {
      icon: "box",
      emoji: null,
      text: "Now showing: mug",
    });
    assert.deepEqual(I.elementIcon("banner", "🎬 Rolling"), { icon: null, emoji: "🎬", text: "Rolling" });
    assert.deepEqual(I.elementIcon("banner", "Rolling"), { icon: null, emoji: null, text: "Rolling" });
  });
});
