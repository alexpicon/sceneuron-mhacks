// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "80s computer": glowing amber phosphor in a chunky monospace on a curved CRT screen,
   with scanlines, a vignette, and a faint glare, set in a beige monitor bezel; text whose own color has a hue is a
   paler amber, a subtitle bar's language is a capitalized tag outlined in phosphor, and frames are bezels around
   the camera with a phosphor glow. Loaded before render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="eighties"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = '"Andale Mono", Monaco, Menlo, "Courier New", monospace';
  const C = {
    text: "#ffb000",
    alt: "#ffe2a6",
    glow: "#ffb000b3",
    glowSoft: "#ff8c0066",
    bezel: "#c9bfa5",
    bezelLight: "#e2dac6",
    bezelShade: "#8c8268",
    screen: "#2e1d06",
    screenMid: "#1c1103",
    screenEdge: "#0a0601",
    glare: "#ffd9a01f",
    scanline: "#0000005c",
    vignette: "#000000d9",
    phosphor: "#ffb00066",
    drop: "#0000008c",
  };

  // The screen's colors under text, as layers from the bottom up.
  const SCREENS = [["screen"], ["screenMid"], ["screenEdge"], ["screen", "glare"], ["screenMid", "scanline"]];

  const css = `
${R} .sn-el{--sn-accent:${C.text}}
${R} .sn-body{font-family:${FONT};color:${C.text};letter-spacing:.02em;-webkit-text-stroke:.03em ${C.text};
border-radius:${u(22)};
background:linear-gradient(180deg,${C.bezelLight},${C.bezel} ${u(12)});
box-shadow:inset 0 ${u(-4)} 0 ${C.bezelShade},0 ${u(6)} ${u(14)} ${C.drop};
text-shadow:0 0 ${u(3)} ${C.glow},0 0 ${u(14)} ${C.glowSoft}}
${R} .sn-kind-text .sn-body{padding:${u(11)} calc(.4em + ${u(14)})}
${R} .sn-alt-text .sn-body{color:${C.alt};-webkit-text-stroke-color:${C.alt}}
${R} .sn-body::after{content:"";position:absolute;inset:${u(8)};z-index:-1;pointer-events:none;border-radius:4% / 16%;
background:repeating-linear-gradient(180deg,transparent 0 ${u(2)},${C.scanline} ${u(2)} ${u(3)}),
radial-gradient(ellipse at 30% 20%,${C.glare},transparent 45%),
radial-gradient(ellipse at 50% 50%,${C.screen},${C.screenMid} 65%,${C.screenEdge});
box-shadow:inset 0 0 ${u(28)} ${u(4)} ${C.vignette}}
${R} .sn-label{border-radius:${u(3)};background:${C.screenEdge};box-shadow:inset 0 0 0 ${u(2)} currentColor;
text-transform:uppercase}
${R} .sn-kind-frame .sn-body{background:transparent;border:${u(12)} solid ${C.bezel};
box-shadow:inset 0 0 ${u(24)} ${C.phosphor},inset 0 0 0 ${u(2)} ${C.bezelShade},0 ${u(6)} ${u(14)} ${C.drop}}
${R} .sn-kind-frame .sn-body::after{display:none}
`;

  const theme = {
    id: "eighties",
    name: "80s computer",
    description: "Glowing amber monospace on a curved CRT screen, with scanlines, in a beige monitor bezel.",
    fonts: [FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["text", SCREENS],
      ["alt", SCREENS],
    ],
    titles: false,
    swatch: { text: "text", panel: "screenMid" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
