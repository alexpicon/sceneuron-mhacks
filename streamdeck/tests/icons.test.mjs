// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the key faces' line icons: they are the overlay's own icons (web/icons.js), each action image in
// imgs/actions/ is the drawing its key shows, so a key looks alike in Stream Deck's action list and on the device,
// and every card of the built-in packs has its icon and short label, or is its words alone.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { test } from "node:test";

import { BUILT_IN_DECKS } from "../src/lib/built-in-cards.ts";
import { CARDS, ICONS, STROKE_WIDTH, iconMarkup } from "../src/lib/icons.ts";
import { PLUGIN_DIR } from "./fake-stream-deck.mjs";

const require = createRequire(import.meta.url);
const shared = require("../../web/icons.js");

/** The shapes inside an action image's icon group, and the group's stroke. */
function drawing(file) {
  const svg = readFileSync(path.join(PLUGIN_DIR, "imgs", "actions", file), "utf8");
  const [, attributes, shapes] = /<g ([^>]*)>(.*)<\/g>/s.exec(svg);
  const stroke = /stroke="(#[0-9a-f]{6})"/.exec(attributes)[1];
  return { shapes, stroke, width: /stroke-width="([\d.]+)"/.exec(attributes)[1] };
}

test("the keys draw the overlay's own icons", () => {
  assert.equal(STROKE_WIDTH, shared.STROKE_WIDTH);
  assert.equal(CARDS, shared.CARDS);
  assert.equal(ICONS.talk, shared.iconMarkup("mic"));
  assert.equal(iconMarkup("timer"), shared.iconMarkup("timer"));
});

test("each action image is the drawing of its key, warm white on graphite, or dark on amber while paused", () => {
  const images = {
    "talk/key.svg": ICONS.talk,
    "reveal/key.svg": ICONS.reveal,
    "next-round/key.svg": ICONS["next-round"],
    "lower/key.svg": ICONS.lower,
    "automation/key.svg": ICONS.pause,
    "reaction/key.svg": ICONS.reaction,
    "element/key.svg": ICONS.eye,
    "card/key.svg": iconMarkup("card"),
    "dial/key.svg": iconMarkup("dial"),
  };
  for (const [file, icon] of Object.entries(images)) {
    assert.deepEqual(drawing(file), { shapes: icon, stroke: "#f2f0ed", width: String(STROKE_WIDTH) }, file);
    const listed = drawing(file.replace("key.svg", "icon.svg"));
    assert.deepEqual([listed.shapes, listed.stroke], [icon, "#ffffff"], "the action list shows it in white");
  }
  assert.deepEqual(drawing("automation/key-paused.svg").shapes, ICONS.play);
  assert.equal(drawing("automation/key-paused.svg").stroke, "#1c1c1e");
});

test("every card of the built-in packs has its icon and a one- or two-word label, or is its words alone", () => {
  for (const deck of BUILT_IN_DECKS) {
    assert.deepEqual(Object.keys(CARDS[deck.id]).sort(), deck.cards.map((card) => card.id).sort(), deck.id);
    for (const card of deck.cards) {
      const entry = CARDS[deck.id][card.id];
      if (entry === null) {
        assert.ok([...card.text].length <= 3, `${deck.id}/${card.id} is a value: ${card.text}`);
        continue;
      }
      const [icon, label] = entry;
      assert.ok(iconMarkup(icon), `${deck.id}/${card.id} draws ${icon}`);
      assert.ok(label.split(" ").length <= 2 && label.length <= 11, `${deck.id}/${card.id}: ${label}`);
    }
  }
  assert.deepEqual(Object.keys(CARDS).sort(), BUILT_IN_DECKS.map((deck) => deck.id).sort());
});
