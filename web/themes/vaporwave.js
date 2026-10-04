// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "Vaporwave": white italics with pink and cyan neon edges on a purple-to-blue gradient,
   with a glowing border from pink to cyan, a pink sunset glow, and a receding grid; text whose own color has a hue
   is pale neon yellow, a subtitle bar's language is a tag with a pink neon edge, and frames are neon gradient
   borders. Loaded before render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="vaporwave"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = '"Avenir Next", Futura, "Helvetica Neue", Verdana, sans-serif';
  const C = {
    text: "#ffffff",
    alt: "#fffb96",
    pink: "#ff71ce",
    purple: "#b967ff",
    cyan: "#01cdfe",
    panelTop: "#3b0f63",
    panelMid: "#250a45",
    panelBottom: "#0d2b55",
    horizon: "#ff71ce4d",
    grid: "#01cdfe59",
    glowPink: "#ff71ce8c",
    glowCyan: "#01cdfe59",
    edgePink: "#ff71cecc",
    edgeCyan: "#01cdfecc",
  };
  const NEON = `linear-gradient(90deg,${C.pink},${C.purple},${C.cyan})`;
  const GRID = (angle) => `linear-gradient(${angle},${C.grid} ${u(3)},transparent ${u(3)}) 0 0 / ${u(48)} ${u(48)}`;

  // The panel's colors under text, as layers from the bottom up.
  const PANELS = [
    ["panelTop"],
    ["panelMid"],
    ["panelBottom"],
    ["panelMid", "horizon"],
    ["panelBottom", "horizon"],
    ["panelBottom", "grid"],
    ["panelBottom", "horizon", "grid"],
  ];

  const css = `
${R} .sn-el{--sn-accent:${C.pink}}
${R} .sn-body{font-family:${FONT};font-style:italic;letter-spacing:.03em;color:${C.text};
border:${u(4)} solid transparent;border-radius:${u(16)};
background:radial-gradient(ellipse at 50% 110%,${C.horizon},transparent 65%) padding-box,
linear-gradient(170deg,${C.panelTop},${C.panelMid} 55%,${C.panelBottom}) padding-box,${NEON} border-box;
box-shadow:0 0 ${u(18)} ${C.glowPink},0 0 ${u(40)} ${C.glowCyan};
text-shadow:${u(-2)} 0 ${C.edgeCyan},${u(2)} 0 ${C.edgePink},0 0 ${u(14)} ${C.glowPink}}
${R} .sn-kind-text .sn-body{padding:0 calc(.4em + ${u(10)})}
${R} .sn-alt-text .sn-body{color:${C.alt}}
${R} .sn-label{background:${C.panelMid};box-shadow:0 0 0 ${u(2)} ${C.pink},0 0 ${u(10)} ${C.glowPink};
text-shadow:none}
${R} .sn-body::after{content:"";position:absolute;z-index:-1;pointer-events:none;left:-50%;right:-50%;bottom:0;
height:200%;background:${GRID("0deg")},${GRID("90deg")};
transform:perspective(calc(var(--sn-h) * 3)) rotateX(70deg);transform-origin:50% 100%;
-webkit-mask-image:linear-gradient(transparent 30%,#000);mask-image:linear-gradient(transparent 30%,#000)}
${R} .sn-kind-frame .sn-body{background:transparent;border:${u(6)} solid;border-image:${NEON} 1;border-radius:0}
${R} .sn-kind-frame .sn-body::after{display:none}
`;

  const theme = {
    id: "vaporwave",
    name: "Vaporwave",
    description: "White italics with pink and cyan neon on purple gradients, over a receding grid.",
    fonts: [FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["text", PANELS],
      ["alt", PANELS],
    ],
    titles: false,
    swatch: { text: "text", panel: "panelMid" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
