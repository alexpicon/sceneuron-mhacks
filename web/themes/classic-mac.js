// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "Classic Mac": the 1984 Macintosh in one-bit black and white, in a Chicago-like system
   font. Text sits in windows with a striped title bar, a close box, and a zoom box, and a hard drop shadow, and in
   inverse video when its own color has a hue; a card is an alert box with a double border, a caution icon, and an
   OK button; a subtitle bar is titled with its language; a frame is a window around the camera, with its title bar
   above the frame so the picture stays clear. Loaded before render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="classic-mac"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = 'Silom, Geneva, "Helvetica Neue", Tahoma, sans-serif';
  const C = { ink: "#000000", paper: "#ffffff" };

  const css = `
${R} .sn-el{--sn-accent:${C.ink};
--sn-title:min(${u(28)},max(${u(24)},calc(var(--sn-h) * .2)),calc(var(--sn-h) * .35));
--sn-ok:min(${u(36)},calc(var(--sn-h) * .19));--sn-icon:min(${u(56)},calc(var(--sn-h) * .34))}
${R} .sn-body{right:${u(5)};bottom:${u(5)};font-family:${FONT};font-synthesis:none;color:${C.ink};
background:${C.paper};border:${u(2)} solid ${C.ink};border-radius:0;box-shadow:${u(5)} ${u(5)} 0 ${C.ink}}
${R} .sn-kind-text .sn-body{padding:calc(var(--sn-title) + ${u(4)}) calc(.4em + ${u(6)}) ${u(4)}}
${R} .sn-alt-text .sn-body{background:${C.ink};color:${C.paper}}
${R} .sn-label{display:none}
${R} .sn-alt-text.sn-card .sn-body::before{background:${C.paper};color:${C.ink}}
${R} .sn-title{display:block;position:absolute;top:0;left:0;right:0;height:var(--sn-title);box-sizing:border-box;
border-bottom:${u(2)} solid ${C.ink};background:${C.paper};color:${C.ink};font-family:${FONT};
font-size:calc(var(--sn-title) * .6);font-weight:400;line-height:1;white-space:nowrap;text-align:center}
${R} .sn-title::before{content:"";position:absolute;left:${u(4)};right:${u(4)};top:22%;bottom:22%;
background:repeating-linear-gradient(180deg,${C.ink} 0 ${u(2)},transparent ${u(2)} ${u(4)})}
${R} .sn-title-text{position:absolute;left:50%;top:50%;max-width:64%;padding:${u(2)} ${u(10)};
transform:translate(-50%,-50%);background:${C.paper};overflow:hidden;text-overflow:ellipsis;line-height:1.25}
${R} .sn-button{position:absolute;top:50%;width:calc(var(--sn-title) * .56);height:calc(var(--sn-title) * .56);
box-sizing:border-box;transform:translateY(-50%);background:${C.paper};border:${u(2)} solid ${C.ink};
box-shadow:0 0 0 ${u(3)} ${C.paper}}
${R} .sn-button-min{display:none}
${R} .sn-button-close{left:${u(14)}}
${R} .sn-button-max{right:${u(14)}}
${R} .sn-button-max::before{content:"";position:absolute;left:0;top:0;width:55%;height:55%;
border-right:${u(2)} solid ${C.ink};border-bottom:${u(2)} solid ${C.ink}}
${R} .sn-card .sn-title{display:none}
${R} .sn-card .sn-body{padding:${u(18)} ${u(26)} calc(var(--sn-ok) + ${u(30)});
box-shadow:${u(5)} ${u(5)} 0 ${C.ink},inset 0 0 0 ${u(3)} ${C.paper},inset 0 0 0 ${u(7)} ${C.ink}}
${R} .sn-card.sn-wide .sn-body{padding:${u(14)} calc(var(--sn-ok) * 2.6 + ${u(48)}) ${u(14)} ${u(26)}}
${R} .sn-card .sn-body::before{content:"!";flex:none;display:flex;align-items:flex-end;justify-content:center;
box-sizing:border-box;width:var(--sn-icon);height:calc(var(--sn-icon) * .9);margin-right:${u(20)};
padding-bottom:calc(var(--sn-icon) * .05);background:${C.ink};color:${C.paper};
clip-path:polygon(50% 0,100% 100%,0 100%);font-family:${FONT};font-size:calc(var(--sn-icon) * .56);line-height:1}
${R} .sn-card .sn-ok{display:flex;align-items:center;justify-content:center;position:absolute;right:${u(26)};
bottom:${u(20)};width:calc(var(--sn-ok) * 2.6);height:var(--sn-ok);box-sizing:border-box;
border:${u(2)} solid ${C.ink};border-radius:calc(var(--sn-ok) * .32);background:${C.paper};color:${C.ink};
font-family:${FONT};font-size:calc(var(--sn-ok) * .5);font-weight:400;line-height:1;white-space:nowrap;
box-shadow:0 0 0 ${u(3)} ${C.paper},0 0 0 ${u(7)} ${C.ink}}
${R} .sn-card.sn-wide .sn-ok{bottom:auto;top:50%;transform:translateY(-50%)}
${R} .sn-kind-frame .sn-body{top:calc(-1 * var(--sn-title) - ${u(2)});background:transparent}
`;

  const theme = {
    id: "classic-mac",
    name: "Classic Mac",
    description: "One-bit black and white windows with striped title bars; cards are alert boxes.",
    fonts: [FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["ink", [["paper"]]],
      ["paper", [["ink"]]],
    ],
    // Title bars show each element's name, on stream.
    titles: true,
    swatch: { text: "ink", panel: "paper" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
