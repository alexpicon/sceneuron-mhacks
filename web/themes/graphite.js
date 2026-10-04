// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "Graphite": graphite panels with a thin edge and warm amber accents, in the system's
   sans-serif. A card shows its icon in a rounded amber tile beside its words, the newest card is edged in amber
   with an amber dot, and a vote card shows a check in an amber circle with the room's count below its words; a text
   element that is not a card is a lower third with an amber bar and, for the starter's labels, an icon; the on-air
   pill shows an amber dot while OBS is live and a gray one when not, beside the time-on-air pill; text whose own
   color has a hue is amber, though translated subtitles are smaller and muted on a soft dark strip; and frames are
   amber. Cards slide in, slide out, and pulse once when raised: a poker card is a playing card that turns over when
   its deck's cover clears, a status card a pill with a colored dot, and a classroom card a speech bubble.
   render-icons.js and render-cards.js give elements their tiles, roles, and card looks. Loaded before
   render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="graphite"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = 'system-ui, -apple-system, "Helvetica Neue", Arial, sans-serif';
  const C = {
    canvas: "#1c1c1e",
    glass: "#1c1c1ef0",
    panel: "#2a2a2c",
    edge: "#3a3a3c",
    text: "#f2f0ed",
    amber: "#d6a76c",
    muted: "#b9b2a6",
    calm: "#8a867f",
    grey: "#8e8e93",
    red: "#c2564b",
    strip: "#1c1c1ee0",
    halo: "#d6a76c40",
    shadow: "#00000066",
  };

  // Card looks (render-cards.js): motion, the poker card and its flip, the status pill, and the classroom bubble.
  const motion = `
${R} .sn-card .sn-enter{animation:sn-graphite-in .2s ease-out both}
${R} .sn-card.sn-gone{opacity:0;translate:0 ${u(16)};
transition:opacity .15s ease-in,translate .15s ease-in,display .15s allow-discrete}
${R} .sn-gp-pulse .sn-body{animation:sn-graphite-pulse .3s ease-in-out .2s}
@keyframes sn-graphite-in{from{opacity:0;transform:translateY(${u(16)})}to{opacity:1;transform:none}}
@keyframes sn-graphite-fade{from{opacity:0}to{opacity:1}}
@keyframes sn-graphite-pulse{0%,100%{transform:scale(1)}50%{transform:scale(1.03)}}`;
  const poker = `
${R} .sn-gp-poker .sn-body{overflow:visible;padding:0;border:0;background:transparent;box-shadow:none;
perspective:${u(1200)}}
${R} .sn-gp-poker .sn-body::after{content:none}
${R} .sn-gp-poker .sn-text,${R} .sn-gp-poker .sn-icon,${R} .sn-gp-quiz .sn-text,${R} .sn-gp-quiz .sn-icon{
visibility:hidden}
${R} .sn-gp-flipper{position:absolute;top:0;left:0;right:0;bottom:0;transform-style:preserve-3d;
transition:transform .6s ease-in-out}
${R} .sn-gp-up{transform:rotateY(180deg)}
${R} .sn-gp-face{position:absolute;top:0;left:0;right:0;bottom:0;box-sizing:border-box;display:flex;
flex-direction:column;align-items:center;justify-content:center;border-radius:${u(24)};line-height:1;
white-space:nowrap;backface-visibility:hidden;box-shadow:0 ${u(8)} ${u(24)} ${C.shadow}}
${R} .sn-gp-face::before,${R} .sn-gp-face::after{content:"";position:absolute;width:${u(12)};height:${u(12)};
transform:rotate(45deg);background:currentColor}
${R} .sn-gp-face::before{top:${u(28)};left:${u(28)}}
${R} .sn-gp-back{background:${C.panel};color:${C.muted};
box-shadow:inset 0 0 0 ${u(10)} ${C.panel},inset 0 0 0 ${u(11)} ${C.edge},0 ${u(8)} ${u(24)} ${C.shadow}}
${R} .sn-gp-back::after{top:${u(28)};right:${u(28)}}
${R} .sn-gp-mono{margin-bottom:${u(44)};font-size:min(${u(170)},calc(var(--sn-h) * .4));font-weight:800}
${R} .sn-gp-tab{position:absolute;left:50%;bottom:${u(24)};transform:translateX(-50%);display:flex;
align-items:center;gap:${u(10)};padding:${u(8)} ${u(16)};border-radius:${u(10)};background:${C.amber};
color:${C.canvas};font-size:${u(32)};font-weight:700}
${R} .sn-gp-tab svg{flex:none;width:1em;height:1em}
${R} .sn-gp-front{background:${C.amber};color:${C.canvas};transform:rotateY(180deg)}
${R} .sn-gp-front::after{right:${u(28)};bottom:${u(28)}}
${R} .sn-gp-value{display:flex;align-items:center;justify-content:center;
font-size:min(${u(130)},calc(var(--sn-h) * .31));font-weight:800;font-variant-numeric:tabular-nums}
${R} .sn-gp-value svg{width:min(${u(120)},calc(var(--sn-h) * .29));height:min(${u(120)},calc(var(--sn-h) * .29))}
${R} .sn-gp-caption{margin-top:${u(16)};font-size:${u(32)};font-weight:700;letter-spacing:.04em}
${R} .sn-gp-results{position:absolute;bottom:0;width:${u(320)};box-sizing:border-box;padding:${u(16)} ${u(22)};
border:${u(1.5)} solid ${C.edge};border-radius:${u(20)};background:${C.panel};color:${C.text};
box-shadow:0 ${u(8)} ${u(24)} ${C.shadow};font-family:${FONT};font-size:${u(30)};line-height:1.3;opacity:0;
visibility:hidden;transition:opacity .15s linear,visibility 0s linear .15s}
${R} .sn-gp-results.sn-gp-on{opacity:1;visibility:visible;transition:opacity .3s ease-out .6s}
${R} .sn-gp-results-left{right:calc(100% + ${u(16)})}
${R} .sn-gp-results-right{left:calc(100% + ${u(16)})}
${R} .sn-gp-row{display:flex;justify-content:space-between;gap:${u(16)};white-space:nowrap}
${R} .sn-gp-who{overflow:hidden;text-overflow:ellipsis;color:${C.muted}}
${R} .sn-gp-vote{flex:none;font-weight:700;font-variant-numeric:tabular-nums}
${R} .sn-gp-average{margin-top:${u(8)};padding-top:${u(8)};border-top:${u(1.5)} solid ${C.edge}}
${R} .sn-gp-average .sn-gp-who{color:${C.text};font-weight:700}`;
  const slots = `
${R} .sn-gp-status .sn-body{top:0;bottom:auto;left:0;right:0;width:fit-content;max-width:100%;height:${u(64)};
margin:0 auto;justify-content:flex-start;padding:0 ${u(28)} 0 ${u(22)};border:${u(1)} solid ${C.edge};
border-radius:${u(32)};background:${C.panel};font-size:${u(30)} !important}
${R} .sn-gp-status .sn-body::after{content:none}
${R} .sn-gp-status .sn-body::before{content:"";flex:none;order:-2;width:${u(14)};height:${u(14)};
margin-right:${u(14)};border-radius:50%;background:${C.grey}}
${R} .sn-gp-dot-red .sn-body::before{background:${C.red}}
${R} .sn-gp-dot-amber .sn-body::before{background:${C.amber}}
${R} .sn-gp-status .sn-icon{--sn-tile:${u(40)};margin-right:${u(10)};background:transparent;color:${C.text}}
${R} .sn-gp-status .sn-icon svg{width:80%;height:80%}
${R} .sn-gp-status .sn-text{white-space:nowrap}
${R} .sn-gp-bubble .sn-body{overflow:visible;background:${C.panel}}
${R} .sn-gp-bubble .sn-body::before{content:"";position:absolute;top:100%;width:${u(46)};height:${u(32)};
background:${C.panel}}
${R} .sn-gp-tail-right .sn-body::before{right:${u(72)};clip-path:polygon(0 0,55% 0,100% 100%)}
${R} .sn-gp-tail-left .sn-body::before{left:${u(72)};clip-path:polygon(45% 0,100% 0,0 100%)}
${R} .sn-gp-answer{position:absolute;top:0;left:0;right:0;bottom:0;display:flex;flex-direction:column;
align-items:center;justify-content:center;line-height:1}
${R} .sn-gp-letter{font-size:min(${u(120)},calc(var(--sn-h) * .55));font-weight:800;color:${C.amber}}
${R} .sn-gp-answer .sn-gp-caption{margin-top:${u(10)};font-size:${u(28)};letter-spacing:.14em;color:${C.muted}}
${R} .sn-subtitles .sn-body{background:${C.strip};border:0;border-radius:${u(12)};box-shadow:none}
${R} .sn-subtitles.sn-alt-text .sn-body{color:${C.muted}}
${R} .sn-subtitles.sn-alt-text .sn-text{font-size:.8em !important}`;
  // Without motion, cards only fade, and a poker card crossfades to its face instead of turning over.
  const still = `
@media (prefers-reduced-motion:reduce){
${R} .sn-card .sn-enter{animation-name:sn-graphite-fade}
${R} .sn-card.sn-gone{translate:none}
${R} .sn-gp-pulse .sn-body{animation:none}
${R} .sn-gp-flipper,${R} .sn-gp-front{transform:none;transition:none}
${R} .sn-gp-face{backface-visibility:visible;transition:opacity .15s linear}
${R} .sn-gp-flipper:not(.sn-gp-up) .sn-gp-front,${R} .sn-gp-up .sn-gp-back{opacity:0}
}`;
  const cards = motion + poker + slots + still;

  const css = `
${R} .sn-el{--sn-accent:${C.amber};--sn-tile:min(calc(var(--sn-h) * .5),${u(120)});
--sn-dot:min(calc(var(--sn-h) * .1),${u(16)})}
${R} .sn-body{font-family:${FONT};color:${C.text};background:${C.glass};border:${u(1.5)} solid ${C.edge};
border-radius:min(calc(var(--sn-h) * .2),${u(26)});box-shadow:0 ${u(8)} ${u(24)} ${C.shadow}}
${R} .sn-kind-text .sn-body{padding:0 calc(.45em + ${u(10)})}
${R} .sn-alt-text .sn-body{color:${C.amber}}
${R} .sn-kind-frame .sn-body{background:transparent;border:${u(4)} solid ${C.amber};border-radius:${u(22)};
box-shadow:0 0 0 ${u(1.5)} ${C.canvas},0 ${u(8)} ${u(24)} ${C.shadow}}
${R} .sn-icon{display:flex;order:-1;flex:none;align-items:center;justify-content:center;box-sizing:border-box;
width:var(--sn-tile);height:var(--sn-tile);margin-right:calc(var(--sn-tile) * .42);
border-radius:calc(var(--sn-tile) * .26);background:${C.amber};color:${C.canvas};line-height:1}
${R} .sn-icon svg{display:block;width:64%;height:64%}
${R} .sn-icon-emoji{font-size:calc(var(--sn-tile) * .56)}
${R} .sn-tiled .sn-body{justify-content:center;padding:0 calc(var(--sn-h) * .26)}
${R} .sn-tiled .sn-text{flex:0 1 auto}
${R} .sn-tiled.sn-align-left .sn-body{justify-content:flex-start}
${R} .sn-tiled.sn-align-right .sn-body{justify-content:flex-end}
${R} .sn-tall .sn-body{flex-direction:column;justify-content:center}
${R} .sn-tall .sn-icon{margin:0 0 calc(var(--sn-tile) * .3)}
${R} .sn-active .sn-body{border-color:${C.amber};padding-right:calc(var(--sn-h) * .26 + var(--sn-dot) * 3)}
${R} .sn-active .sn-body::after{content:"";position:absolute;top:50%;right:calc(var(--sn-h) * .22);
width:var(--sn-dot);height:var(--sn-dot);margin-top:calc(var(--sn-dot) / -2);border-radius:50%;
background:${C.amber}}
${R} .sn-vote .sn-body{border-color:${C.amber}}
${R} .sn-vote .sn-icon{border-radius:50%}
${R} .sn-vote .sn-text:not([data-note=""])::after{content:attr(data-note);display:block;margin-top:.12em;
font-size:.62em;font-weight:500;color:${C.muted}}
${R} .sn-label{border-radius:${u(6)};background:${C.amber};color:${C.canvas};font-weight:700;letter-spacing:.02em}
${R} .sn-lower-third .sn-body{padding-left:calc(var(--sn-h) * .36 + ${u(6)})}
${R} .sn-lower-third .sn-body::before{content:"";position:absolute;top:24%;bottom:24%;left:calc(var(--sn-h) * .17);
width:max(${u(4)},calc(var(--sn-h) * .05));border-radius:${u(4)};background:${C.amber}}
${R} .sn-lower-third .sn-icon{background:${C.panel};color:${C.text};box-shadow:inset 0 0 0 ${u(1.5)} ${C.edge};
--sn-tile:min(calc(var(--sn-h) * .52),${u(110)})}
${R} .sn-on-air .sn-body,${R} .sn-clock .sn-body{justify-content:center;padding:0 calc(var(--sn-h) * .34);
border-radius:calc(var(--sn-h) / 2)}
${R} .sn-on-air .sn-body{text-transform:uppercase;letter-spacing:.06em}
${R} .sn-on-air .sn-text,${R} .sn-clock .sn-text{flex:0 1 auto}
${R} .sn-on-air .sn-body::before{content:"";flex:none;width:.42em;height:.42em;margin-right:.6em;border-radius:50%;
background:${C.calm}}
${R} .sn-live .sn-body::before{background:${C.amber};box-shadow:0 0 0 .2em ${C.halo}}
${R} .sn-clock .sn-body{color:${C.muted};font-variant-numeric:tabular-nums;letter-spacing:.02em}
${cards}`;

  const theme = {
    id: "graphite",
    name: "Graphite",
    description:
      "Graphite panels with warm amber: cards show an icon in an amber tile, the newest card is edged in amber, " +
      "labels get an amber bar, and the on-air pill lights up while OBS is live.",
    fonts: [FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["text", [["glass"], ["canvas"], ["panel"], ["strip"]]],
      ["amber", [["glass"], ["panel"]]],
      ["muted", [["glass"], ["panel"], ["strip"]]],
      ["canvas", [["amber"]]],
    ],
    titles: false,
    // Cards and labels show icons in tiles (render-icons.js).
    icons: true,
    swatch: { text: "amber", panel: "canvas" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
