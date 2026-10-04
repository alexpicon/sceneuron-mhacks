// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the key and touch strip drawings in the Graphite look: escaping of card text, the graphite key and the
// amber one while lit, the shade while dim, which leaves the caption clear, an amber caption that asks for pairing,
// the icon at half the key above its label, readable at 72 pixels, the layout regions, and the base64 data URL
// Stream Deck takes.

import assert from "node:assert/strict";
import { test } from "node:test";

import { ICONS } from "../src/lib/icons.ts";
import { escapeXml, keySvg, stripSvg, svgDataUrl } from "../src/lib/svg.ts";

const face = { title: "5", lit: false, dim: false };

/** The text drawn in each <text> element, unescaped. */
function texts(svg) {
  return [...svg.matchAll(/<text [^>]*>([^<]*)<\/text>/g)].map(([, body]) =>
    body.replace(/&#(\d+);/g, (_, code) => String.fromCharCode(Number(code))),
  );
}

/** Each text element's baseline, font size, color, and text. */
function lines(svg) {
  const found = svg.matchAll(/<text x="[\d.]+" y="([\d.]+)"[^>]* font-size="([\d.]+)"[^>]* fill="([^"]+)">([^<]*)</g);
  return [...found].map(([, y, size, fill, body]) => ({ y: Number(y), size: Number(size), fill, body }));
}

test("text that looks like markup is escaped", () => {
  assert.equal(escapeXml(`<b>&"'`), "&#60;b&#62;&#38;&#34;&#39;");
  const svg = keySvg({ ...face, title: '</text><script>alert("x")</script>' });
  assert.ok(!svg.includes("<script>"));
  assert.equal(texts(svg).join(""), '</text><script>alert("x")</script>');
});

test("control characters never reach the drawing", () => {
  const svg = keySvg({ ...face, title: "a\u0000b\u0007c\nd" });
  assert.equal(texts(svg).join(" "), "abc d");
});

test("a key is a 144x144 graphite key with an edge and warm white words, and amber with dark ink while lit", () => {
  const svg = keySvg(face);
  assert.match(svg, /^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" width="144" height="144" viewBox="0 0 144 144">/);
  assert.match(svg, /<rect width="144" height="144" fill="#1c1c1e"\/>/);
  assert.match(svg, /<rect x="1" y="1" width="142" height="142" rx="15" fill="#2a2a2c" stroke="#3a3a3c"/);
  assert.match(svg, /<text [^>]*fill="#f2f0ed">5<\/text>/);
  assert.ok(svg.endsWith("</svg>"));
  const lit = keySvg({ ...face, lit: true });
  assert.match(lit, /<rect width="144" height="144" rx="16" fill="#d6a76c"\/>/);
  assert.match(lit, /<text [^>]*fill="#1c1c1e">5<\/text>/);
  assert.ok(!lit.includes("#2a2a2c"));
});

test("only a dim key has the shade, and its caption stays clear of it, amber when it asks for pairing", () => {
  const shade = '<rect width="144" height="144" rx="16" fill="#1c1c1e" fill-opacity="0.6"/>';
  assert.ok(!keySvg({ ...face, caption: "Offline" }).includes(shade));
  const offline = keySvg({ ...face, dim: true, caption: "Offline" });
  assert.ok(offline.indexOf(shade) > 0 && offline.indexOf(shade) < offline.indexOf(">Offline<"));
  assert.equal(lines(offline).at(-1).fill, "#b9b5ae");
  const refused = keySvg({ ...face, dim: true, caption: "Pair again", alert: true });
  assert.equal(lines(refused).at(-1).fill, "#d6a76c");
});

test("the tag sits above the title and the caption below it", () => {
  for (const shown of [{ title: "Be right back" }, { title: "Lower", icon: ICONS.lower }]) {
    const svg = keySvg({ ...face, ...shown, tag: "Held", caption: "Shown" });
    const found = lines(svg);
    assert.equal(found[0].body, "Held");
    assert.equal(found.at(-1).body, "Shown");
    const ys = found.map((line) => line.y);
    assert.deepEqual([...ys].sort((a, b) => a - b), ys);
    assert.ok(ys.at(-1) <= 136, "the caption stays inside the key");
  }
});

test("an icon takes half the key above a label that reads at 72 pixels", () => {
  const svg = keySvg({ ...face, title: "Off track", icon: ICONS.lower });
  const [, x, y, scale] = /<g transform="translate\(([\d.]+) ([\d.]+)\) scale\(([\d.]+)\)" fill="none"/.exec(svg);
  assert.deepEqual([Number(x), Number(scale) * 24], [36, 72], "centered, half the key wide");
  assert.match(svg, /stroke="#f2f0ed" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">/);
  const [label] = lines(svg);
  assert.equal(label.body, "Off track");
  assert.ok(label.size / 2 >= 10 && label.y > Number(y) + 72 && label.y < 144, JSON.stringify(label));
  assert.match(keySvg({ ...face, title: "Off track", icon: ICONS.lower, lit: true }), /stroke="#1c1c1e" stroke-width/);
  const long = lines(keySvg({ ...face, title: "Let's take this offline", icon: ICONS.lower }));
  assert.ok(long.length === 2 && long.every((line) => line.size >= 17), "a long label takes two lines");
});

test("a strip is 200x100 with the header, the position, and the card", () => {
  const svg = stripSvg({ ...face, header: "Planning poker", position: "4/10" });
  assert.match(svg, /width="200" height="100" viewBox="0 0 200 100"/);
  assert.deepEqual(texts(svg), ["Planning poker", "4/10", "5"]);
  assert.match(svg, /text-anchor="end"[^>]*>4\/10</);
  assert.match(stripSvg({ ...face, header: "Poker", position: "", lit: true }), /fill="#d6a76c"/);
});

test("data URLs carry the SVG as base64 UTF-8, emoji included", () => {
  const svg = keySvg({ ...face, title: "☕ Back in 5" });
  const url = svgDataUrl(svg);
  assert.ok(url.startsWith("data:image/svg+xml;base64,"));
  assert.equal(Buffer.from(url.slice(26), "base64").toString("utf8"), svg);
});
