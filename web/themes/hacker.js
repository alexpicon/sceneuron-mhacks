// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "Hacker": neon green monospace with a soft glow on black glass, a shell prompt before
   the text and a blinking block cursor after it, and faint scanlines; text whose own color has a hue is cyan, a
   subtitle bar's language is a tag outlined in its color, and frames are green brackets around the camera. Loaded
   before render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="hacker"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = 'Menlo, Monaco, "SF Mono", Consolas, "Courier New", monospace';
  const C = {
    text: "#3dff8a",
    alt: "#4de8ff",
    altGlow: "#4de8ff99",
    prompt: "#33dd72",
    glass: "#0c2a17eb",
    glassDeep: "#020a05eb",
    edge: "#3dff8a99",
    glow: "#3dff8a59",
    innerGlow: "#3dff8a1f",
    textGlow: "#3dff8a99",
    scanline: "#00000047",
  };
  // Bracket corners for frames: a horizontal and a vertical bar in each corner.
  const BAR = `linear-gradient(${C.text},${C.text})`;
  const BRACKETS = ["left top", "right top", "left bottom", "right bottom"]
    .flatMap((corner) => [`${BAR} ${corner} / ${u(44)} ${u(5)}`, `${BAR} ${corner} / ${u(5)} ${u(44)}`])
    .join(",");

  // The glass's colors under text, as layers from the bottom up.
  const GLASS = [["glass"], ["glassDeep"], ["glass", "innerGlow"], ["glassDeep", "scanline"]];

  const css = `
${R} .sn-el{--sn-accent:${C.text}}
${R} .sn-body{font-family:${FONT};color:${C.text};border:${u(2)} solid ${C.edge};border-radius:${u(6)};
background:linear-gradient(165deg,${C.glass},${C.glassDeep} 60%);
box-shadow:0 0 ${u(16)} ${C.glow},inset 0 0 ${u(22)} ${C.innerGlow};text-shadow:0 0 ${u(8)} ${C.textGlow}}
${R} .sn-kind-text .sn-body{padding:0 calc(.4em + ${u(6)})}
${R} .sn-alt-text .sn-body{color:${C.alt};text-shadow:0 0 ${u(8)} ${C.altGlow}}
${R} .sn-body::after{content:"";position:absolute;inset:0;z-index:-1;pointer-events:none;
background:repeating-linear-gradient(180deg,transparent 0 ${u(2)},${C.scanline} ${u(2)} ${u(4)})}
${R} .sn-kind-text .sn-text::before{content:"$\\a0";color:${C.prompt}}
${R} .sn-kind-text .sn-text::after{content:"\\2588";margin-left:.12em;font-weight:400;
animation:sn-hacker-cursor 1.1s step-end infinite}
@keyframes sn-hacker-cursor{50%{opacity:0}}
${R} .sn-label{border-radius:${u(3)};background:${C.glassDeep};box-shadow:inset 0 0 0 ${u(1.5)} currentColor}
${R} .sn-glossed .sn-text::before,${R} .sn-glossed .sn-text::after{content:none}
${R} .sn-glossed .sn-line:first-child::before{content:"$\\a0";color:${C.prompt}}
${R} .sn-glossed .sn-text>:last-child::after{content:"\\2588";margin-left:.12em;font-weight:400;
animation:sn-hacker-cursor 1.1s step-end infinite}
${R} .sn-kind-frame .sn-body{background:transparent;border:${u(2)} solid ${C.edge};border-radius:${u(4)};
box-shadow:0 0 ${u(14)} ${C.glow},inset 0 0 ${u(14)} ${C.glow}}
${R} .sn-kind-frame .sn-body::after{inset:${u(8)};background:${BRACKETS};background-repeat:no-repeat;
filter:drop-shadow(0 0 ${u(6)} ${C.text})}
`;

  const theme = {
    id: "hacker",
    name: "Hacker",
    description: "Neon green monospace on black glass, with a prompt, a blinking cursor, and faint scanlines.",
    fonts: [FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["text", GLASS],
      ["prompt", GLASS],
      ["alt", GLASS],
    ],
    titles: false,
    swatch: { text: "text", panel: "glassDeep" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
