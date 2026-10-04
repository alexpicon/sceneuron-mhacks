// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises what a Reaction key shows: its preset's emoji and short name for every preset, before
// pairing as after, lit and saying Playing only while live and its preset plays, darkened while
// Sceneuron is out of reach, and asking for a reaction or saying Not found otherwise; and its
// drawing, made at 144 pixels and shown at 72 on the 15-key Stream Deck, which keeps the emoji
// large and the name readable at both sizes, inside the key.

import assert from "node:assert/strict";
import { test } from "node:test";

import { BUILT_IN_REACTIONS } from "../src/lib/built-in-reactions.ts";
import { ICONS } from "../src/lib/icons.ts";
import { findReaction, reactionFace } from "../src/lib/reaction-face.ts";
import { keySvg } from "../src/lib/svg.ts";

const EMAIL = { reaction: "email" };

/** Each text element of a drawing: its baseline, font size, and text. */
function texts(svg) {
  return [...svg.matchAll(/<text x="[\d.]+" y="([\d.]+)"[^>]* font-size="([\d.]+)"[^>]*>([^<]*)<\/text>/g)].map(
    ([, y, size, body]) => ({ y: Number(y), size: Number(size), body }),
  );
}

test("every preset's key shows its first emoji and short name", () => {
  assert.deepEqual(
    BUILT_IN_REACTIONS.map((preset) => preset.id),
    ["tongue", "email", "mind-blown", "ship-it", "coffee", "absolutely-right", "more-tokens", "works-on-my-gpu", "meatproxy", "clanker"],
  );
  for (const preset of BUILT_IN_REACTIONS) {
    const face = reactionFace("live", { reaction: preset.id }, null);
    assert.deepEqual([face.emoji, face.title], [preset.emojis[0], preset.short]);
    assert.deepEqual([face.lit, face.dim, face.caption], [false, false, undefined], preset.id);
  }
  assert.equal(findReaction("email").banner, "This meeting could have been an email");
  assert.equal(findReaction(undefined), undefined);
});

test("a key lights up and says Playing only while live and its own preset plays", () => {
  const playing = reactionFace("live", EMAIL, "email");
  assert.deepEqual([playing.lit, playing.caption], [true, "Playing"]);
  assert.equal(reactionFace("live", EMAIL, "ship-it").lit, false);
  assert.equal(reactionFace("offline", EMAIL, "email").lit, false);
});

test("before pairing it is drawn in full, and darkened only while out of reach or refused", () => {
  const shown = (reach) => {
    const face = reactionFace(reach, EMAIL, null);
    return [face.title, face.emoji, face.caption, face.dim];
  };
  assert.deepEqual(shown("unpaired"), ["Email", "📧", "Not paired", false]);
  assert.deepEqual(shown("offline"), ["Email", "📧", "Offline", true]);
  assert.deepEqual(shown("refused"), ["Email", "📧", "Pair again", true]);
});

test("a key without a reaction asks for one, and one the plugin lacks says Not found", () => {
  const choose = reactionFace("live", {}, null);
  assert.deepEqual([choose.title, choose.icon, choose.emoji], ["Choose a reaction", ICONS.reaction, undefined]);
  const missing = reactionFace("live", { reaction: "confetti" }, null);
  assert.deepEqual([missing.title, missing.caption, missing.dim], ["confetti", "Not found", true]);
  assert.equal(reactionFace("unpaired", { reaction: "confetti" }, null).caption, "Not paired");
});

test("the drawing is made at 144 pixels and reads at 72: a large emoji, then the name", () => {
  for (const preset of BUILT_IN_REACTIONS) {
    for (const reach of ["live", "unpaired"]) {
      const svg = keySvg(reactionFace(reach, { reaction: preset.id }, null));
      assert.match(svg, /width="144" height="144" viewBox="0 0 144 144"/);
      const [emoji, name, ...rest] = texts(svg);
      assert.deepEqual([emoji.body, name.body], [preset.emojis[0], preset.short], `${preset.id} ${reach}`);
      assert.ok(emoji.size / 2 >= 26, `the emoji is at least 26 pixels on a 72-pixel key: ${emoji.size}`);
      assert.ok(name.size / 2 >= 10, `the name is at least 10 pixels at 72: ${name.size}`);
      assert.ok(name.size >= 20 && name.size <= 26, `the name at 144: ${name.size}`);
      assert.ok(emoji.y < name.y && rest.every((line) => line.y > name.y), "emoji, name, then caption");
      assert.ok([emoji, name, ...rest].every((line) => line.y > 0 && line.y < 144), "inside the key");
      assert.ok(!/…/.test(name.body), `${preset.short} fits without an ellipsis`);
    }
  }
});

test("lit and caption drawings keep the emoji and the name, and the key is amber only while playing", () => {
  const amber = 'rx="16" fill="#d6a76c"/>';
  const lit = keySvg(reactionFace("live", EMAIL, "email"));
  assert.ok(lit.includes(amber));
  assert.deepEqual(
    texts(lit).map((line) => line.body),
    ["📧", "Email", "Playing"],
  );
  assert.ok(texts(lit).at(-1).y <= 136, "the caption stays inside the key");
  assert.ok(!keySvg(reactionFace("live", EMAIL, null)).includes(amber));
});
