// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const OUT = resolve(import.meta.dirname, "..", "..", "docs", "graphics");
const C = {
  bg: "#17151b",
  panel: "#242129",
  panelHigh: "#2c2733",
  stroke: "#4b4254",
  text: "#f2f0ed",
  muted: "#b9b2c2",
  faint: "#8f879b",
  amber: "#d6a76c",
  violet: "#b99bf0",
  cyan: "#78c5da",
  green: "#71d8a5",
  rose: "#ed9eaa",
};

const xml = (value) => String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const tone = (name) => C[name] ?? C.amber;
const text = (x, y, value, size = 17, fill = C.text, weight = 400, tracking = 0) =>
  '<text x="' + x + '" y="' + y + '" fill="' + fill + '" font-size="' + size
  + '" font-weight="' + weight + '" letter-spacing="' + tracking
  + '" font-family="Inter, ui-sans-serif, system-ui, -apple-system, sans-serif">'
  + xml(value) + "</text>";
const rect = (x, y, w, h, fill, radius = 20, stroke = "none", extra = "") =>
  '<rect x="' + x + '" y="' + y + '" width="' + w + '" height="' + h
  + '" rx="' + radius + '" fill="' + fill + '" stroke="' + stroke + '" ' + extra + "/>";
const line = (x1, y1, x2, y2, color = C.stroke, width = 2, extra = "") =>
  '<path d="M ' + x1 + " " + y1 + " L " + x2 + " " + y2
  + '" fill="none" stroke="' + color + '" stroke-width="' + width + '" ' + extra + "/>";

function card(x, y, w, h, item, index, compact = false) {
  const accent = tone(item.tone);
  const body = Array.isArray(item.body) ? item.body : [item.body];
  const parts = [
    rect(x, y, w, h, "url(#card)", 19, C.stroke, 'filter="url(#shadow)"'),
    rect(x + 18, y + 17, 45, 4, accent, 2),
    text(x + 20, y + (compact ? 52 : 58), String(index).padStart(2, "0"), 15, accent, 700, 2.5),
    text(x + 20, y + (compact ? 83 : 100), item.title, compact ? 20 : 23, C.text, 700),
  ];
  const first = y + (compact ? 109 : 139);
  body.forEach((value, i) => parts.push(text(x + 20, first + i * 25, value, compact ? 14 : 16, C.muted)));
  return parts.join("\n");
}

function header(spec, index) {
  const accent = tone(spec.tone);
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="640" viewBox="0 0 1200 640" role="img" aria-labelledby="title desc">',
    '<title id="title">' + xml(spec.title) + "</title>",
    '<desc id="desc">' + xml(spec.subtitle) + "</desc>",
    "<defs>",
    '<linearGradient id="background" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#201b28"/><stop offset=".58" stop-color="#17151b"/><stop offset="1" stop-color="#121216"/></linearGradient>',
    '<linearGradient id="card" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#2b2732"/><stop offset="1" stop-color="#201e25"/></linearGradient>',
    '<radialGradient id="aura"><stop stop-color="' + accent + '" stop-opacity=".16"/><stop offset="1" stop-color="' + accent + '" stop-opacity="0"/></radialGradient>',
    '<pattern id="dots" width="28" height="28" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="1" fill="#9f89c4" opacity=".13"/></pattern>',
    '<filter id="shadow" x="-20%" y="-30%" width="140%" height="170%"><feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#08070b" flood-opacity=".45"/></filter>',
    '<marker id="arrow" markerWidth="9" markerHeight="9" refX="7.5" refY="4.5" orient="auto"><path d="M1 1 L8 4.5 L1 8" fill="none" stroke="' + accent + '" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></marker>',
    "</defs>",
    rect(0, 0, 1200, 640, "url(#background)", 0),
    rect(0, 0, 1200, 640, "url(#dots)", 0),
    '<circle cx="1050" cy="30" r="430" fill="url(#aura)"/>',
    rect(54, 48, 7, 70, accent, 3),
    text(78, 61, (spec.eyebrow ?? "SCENEURON / SYSTEM").toUpperCase(), 13, accent, 700, 3),
    text(78, 108, spec.title, 38, C.text, 750),
    text(80, 143, spec.subtitle, 17, C.muted),
    line(54, 171, 1146, 171, C.stroke, 1),
    line(54, 594, 1146, 594, C.stroke, 1),
    text(55, 619, "SCENEURON", 12, C.faint, 700, 2.5),
    text(1097, 619, String(index).padStart(2, "0"), 12, accent, 700, 2),
  ].join("\n");
}

function flow(spec) {
  const items = spec.items;
  const gap = items.length === 5 ? 22 : 28;
  const w = (1092 - (items.length - 1) * gap) / items.length;
  const y = 215;
  const h = 285;
  const parts = [];
  for (let i = 0; i < items.length - 1; i++) {
    const x = 54 + i * (w + gap);
    parts.push(line(x + w + 2, y + 146, x + w + gap - 8, y + 146, tone(spec.tone), 2.5, 'marker-end="url(#arrow)"'));
  }
  items.forEach((item, i) => parts.push(card(54 + i * (w + gap), y, w, h, item, i + 1)));
  return parts.join("\n");
}

function grid(spec) {
  const cols = spec.items.length <= 4 ? 2 : 3;
  const gap = 24;
  const w = (1092 - (cols - 1) * gap) / cols;
  const h = spec.items.length <= 4 ? 148 : 145;
  return spec.items.map((item, i) => {
    const x = 54 + (i % cols) * (w + gap);
    const y = 197 + Math.floor(i / cols) * 174;
    return card(x, y, w, h, item, i + 1, true);
  }).join("\n");
}

function lanes(spec) {
  const gap = 24;
  const w = (1092 - (spec.items.length - 1) * gap) / spec.items.length;
  return spec.items.map((item, i) =>
    card(54 + i * (w + gap), 205, w, 292, item, i + 1)
  ).join("\n");
}

function hub(spec) {
  const center = { x: 441, y: 291, w: 318, h: 166 };
  const positions = [
    { x: 54, y: 218, w: 288, h: 135 },
    { x: 858, y: 218, w: 288, h: 135 },
    { x: 54, y: 399, w: 288, h: 135 },
    { x: 858, y: 399, w: 288, h: 135 },
  ];
  const parts = [];
  positions.forEach((p, i) => {
    const startX = i % 2 === 0 ? p.x + p.w : p.x;
    const endX = i % 2 === 0 ? center.x : center.x + center.w;
    const sy = p.y + p.h / 2;
    const ey = center.y + center.h / 2;
    const middle = (startX + endX) / 2;
    parts.push('<path d="M ' + startX + " " + sy + " C " + middle + " " + sy + ", "
      + middle + " " + ey + ", " + endX + " " + ey
      + '" fill="none" stroke="' + tone(spec.tone) + '" stroke-opacity=".75" stroke-width="2.4" marker-end="url(#arrow)"/>');
  });
  spec.items.forEach((item, i) => {
    const p = positions[i];
    parts.push(card(p.x, p.y, p.w, p.h, item, i + 1, true));
  });
  parts.push(rect(center.x, center.y, center.w, center.h, C.panelHigh, 25, tone(spec.tone), 'filter="url(#shadow)" stroke-width="2"'));
  parts.push(text(center.x + 24, center.y + 46, "LIVE CORE", 13, tone(spec.tone), 700, 2.8));
  parts.push(text(center.x + 24, center.y + 92, spec.center.title, 27, C.text, 750));
  parts.push(text(center.x + 24, center.y + 125, spec.center.body, 16, C.muted));
  return parts.join("\n");
}

export function writeDiagrams(diagrams) {
  mkdirSync(OUT, { recursive: true });
  diagrams.forEach((spec, index) => {
    const main = { flow, grid, lanes, hub }[spec.kind](spec);
    const note = spec.note
      ? rect(54, 541, 1092, 39, C.panel, 12, C.stroke)
        + text(73, 566, spec.note, 15, tone(spec.tone), 600)
      : "";
    const notice = "<!-- Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. -->\n"
      + "<!-- Proprietary software; see LICENSE. -->\n\n";
    const svg = notice + header(spec, index + 1) + "\n" + main + "\n" + note + "\n</svg>\n";
    writeFileSync(join(OUT, spec.name + ".svg"), svg, "utf8");
  });
}
