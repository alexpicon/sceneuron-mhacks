// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay theme "Windows 95": silver bevelled windows with a navy gradient title bar and minimize,
   maximize, and close buttons. Text sits in a window's white client area, in navy when its own color has a hue; a
   card is a dialog box with an information icon and an OK button, titled with its deck's name, and a subtitle bar
   is titled with its language; a frame is a window around the camera, with its title bar above the frame so the
   picture stays clear. Loaded before render-themes.js, which collects it. */
(function () {
  "use strict";

  const R = '.sn-root[data-theme="windows-95"]';
  const u = (n) => `calc(${n} * var(--sn-u))`;
  const FONT = 'Tahoma, Verdana, "Segoe UI", sans-serif';
  const ICON_FONT = 'Georgia, "Times New Roman", serif';
  const C = {
    face: "#c0c0c0",
    light: "#dfdfdf",
    highlight: "#ffffff",
    shadow: "#808080",
    dark: "#0a0a0a",
    text: "#000000",
    alt: "#000080",
    window: "#ffffff",
    title: "#000080",
    titleEnd: "#1060b0",
    titleText: "#ffffff",
    icon: "#000080",
  };
  // Lines `step` wide along the edges: the outer pair, then the inner pair, each as top-left and bottom-right.
  const bevel = (step, outerTopLeft, outerBottomRight, innerTopLeft, innerBottomRight) =>
    [
      `inset ${u(-step)} ${u(-step)} ${outerBottomRight}`,
      `inset ${u(step)} ${u(step)} ${outerTopLeft}`,
      `inset ${u(-2 * step)} ${u(-2 * step)} ${innerBottomRight}`,
      `inset ${u(2 * step)} ${u(2 * step)} ${innerTopLeft}`,
    ].join(",");
  const RAISED = bevel(2, C.light, C.dark, C.highlight, C.shadow);
  const BUTTON = bevel(2, C.highlight, C.dark, C.light, C.shadow);
  const SUNKEN = bevel(1.5, C.shadow, C.highlight, C.dark, C.light);

  const css = `
${R} .sn-el{--sn-accent:${C.titleEnd};
--sn-title:min(${u(30)},max(${u(24)},calc(var(--sn-h) * .2)),calc(var(--sn-h) * .35));
--sn-ok:min(${u(34)},calc(var(--sn-h) * .19));--sn-icon:min(${u(56)},calc(var(--sn-h) * .34))}
${R} .sn-body{font-family:${FONT};color:${C.text};background:${C.face};border-radius:0;box-shadow:${RAISED}}
${R} .sn-kind-text .sn-body{padding:calc(var(--sn-title) + ${u(10)}) calc(.4em + ${u(8)}) ${u(8)}}
${R} .sn-alt-text .sn-body{color:${C.alt}}
${R} .sn-label{display:none}
${R} .sn-body::after{content:"";position:absolute;z-index:-1;top:calc(var(--sn-title) + ${u(7)});left:${u(5)};
right:${u(5)};bottom:${u(5)};background:${C.window};box-shadow:${SUNKEN}}
${R} .sn-title{display:flex;align-items:center;gap:${u(2)};position:absolute;top:${u(5)};left:${u(5)};right:${u(5)};
height:var(--sn-title);box-sizing:border-box;padding:0 ${u(3)} 0 ${u(8)};
background:linear-gradient(90deg,${C.title},${C.titleEnd});color:${C.titleText};font-family:${FONT};
font-size:calc(var(--sn-title) * .56);font-weight:700;line-height:1;white-space:nowrap;text-align:left}
${R} .sn-title-text{flex:1 1 auto;min-width:0;overflow:hidden;text-overflow:ellipsis;line-height:1.3}
${R} .sn-button{flex:none;position:relative;width:calc(var(--sn-title) * .78);height:calc(var(--sn-title) * .68);
background:${C.face};box-shadow:${BUTTON}}
${R} .sn-button-close{margin-left:${u(4)}}
${R} .sn-button::before,${R} .sn-button::after{content:"";position:absolute;background:${C.text}}
${R} .sn-button::after{display:none}
${R} .sn-button-min::before{left:30%;right:38%;bottom:26%;height:${u(3)}}
${R} .sn-button-max::before{left:24%;right:26%;top:20%;bottom:24%;background:transparent;
border:${u(1.5)} solid ${C.text};border-top-width:${u(3)}}
${R} .sn-button-close::before,${R} .sn-button-close::after{display:block;left:24%;right:24%;
top:calc(50% - ${u(1.5)});height:${u(3)};transform:rotate(45deg)}
${R} .sn-button-close::after{transform:rotate(-45deg)}
${R} .sn-card .sn-body{padding:calc(var(--sn-title) + ${u(14)}) ${u(24)} calc(var(--sn-ok) + ${u(22)})}
${R} .sn-card.sn-wide .sn-body{padding:calc(var(--sn-title) + ${u(10)}) calc(var(--sn-ok) * 2.8 + ${u(40)})
${u(10)} ${u(24)}}
${R} .sn-card .sn-body::after{display:none}
${R} .sn-card .sn-body::before{content:"i";flex:none;display:flex;align-items:center;justify-content:center;
box-sizing:border-box;width:var(--sn-icon);height:var(--sn-icon);margin-right:${u(18)};
border:${u(2)} solid ${C.dark};border-radius:50%;
background:radial-gradient(circle at 35% 30%,${C.highlight} 55%,${C.light});color:${C.icon};
font-family:${ICON_FONT};font-size:calc(var(--sn-icon) * .66);font-style:italic;font-weight:700;line-height:1}
${R} .sn-card .sn-ok{display:flex;align-items:center;justify-content:center;position:absolute;left:50%;
bottom:${u(14)};width:calc(var(--sn-ok) * 2.8);height:var(--sn-ok);transform:translateX(-50%);
background:${C.face};color:${C.text};font-family:${FONT};font-size:calc(var(--sn-ok) * .5);font-weight:400;
line-height:1;white-space:nowrap;box-shadow:0 0 0 ${u(2)} ${C.dark},${BUTTON}}
${R} .sn-card.sn-wide .sn-ok{left:auto;right:${u(22)};bottom:auto;top:calc(50% + var(--sn-title) / 2);
transform:translateY(-50%)}
${R} .sn-card .sn-ok::before{content:"";position:absolute;inset:${u(6)};border:${u(1.5)} dotted ${C.text}}
${R} .sn-kind-frame .sn-body{top:calc(-1 * var(--sn-title) - ${u(5)});
background:linear-gradient(${C.face},${C.face}) top / 100% calc(var(--sn-title) + ${u(12)}) no-repeat;
box-shadow:${RAISED},inset 0 0 0 ${u(7)} ${C.face}}
${R} .sn-kind-frame .sn-body::after{display:none}
`;

  const theme = {
    id: "windows-95",
    name: "Windows 95",
    description: "Silver bevelled windows with navy title bars; cards are dialog boxes with an OK button.",
    fonts: [FONT, ICON_FONT],
    colors: C,
    // Each text color with the panels it sits on, as layers from the bottom up.
    readable: [
      ["text", [["face"], ["window"]]],
      ["alt", [["face"], ["window"]]],
      ["titleText", [["title"], ["titleEnd"]]],
      ["icon", [["highlight"], ["light"]]],
    ],
    // Title bars show each element's name, on stream.
    titles: true,
    swatch: { text: "titleText", panel: "title" },
    css,
  };

  if (typeof module !== "undefined" && module.exports) {
    module.exports = theme;
  } else {
    (window.SceneuronThemeDefinitions = window.SceneuronThemeDefinitions || []).push(theme);
  }
})();
