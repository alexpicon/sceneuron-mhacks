// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay floods in the Graphite theme: instead of raining emojis, a flood throws a fan of line icons up
   and outward from the bottom center of the canvas, in amber and off-white, that fade out by the end of the preset.
   With reduced motion each icon only fades in and out where it lands. Each preset shows one icon from icons.js.
   Planned by pure functions that the Node tests check; flood.js draws it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Icons = inNode ? require("./icons.js") : window.SceneuronIcons;
  const SVG = "http://www.w3.org/2000/svg";
  /** The icon each preset throws, by preset id; any other preset throws the fallback. */
  const ICON_BY_PRESET = Object.freeze({
    email: "envelope",
    "mind-blown": "spark",
    "ship-it": "rocket",
    coffee: "mug",
    tongue: "reaction",
    "absolutely-right": "check-circle",
    "more-tokens": "coin",
    "works-on-my-gpu": "chip",
    meatproxy: "proxy",
    clanker: "robot",
  });
  const FALLBACK_ICON = "spark";
  const COLORS = Object.freeze(["#D6A76C", "#F2F0ED"]);
  /** The fan's half-width around straight up, in degrees. */
  const FAN = 60;
  const EASING = "cubic-bezier(.16,1,.3,1)";

  /** A seeded generator of numbers in [0, 1), so one flood always looks the same (mulberry32). */
  function generator(seed) {
    let state = Math.floor(Math.abs(Number(seed) || 0)) >>> 0;
    return () => {
      state = (state + 0x6d2b79f5) >>> 0;
      let t = state;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  /** The icon the preset `id` throws. */
  function iconOf(id) {
    return Object.prototype.hasOwnProperty.call(ICON_BY_PRESET, id) ? ICON_BY_PRESET[id] : FALLBACK_ICON;
  }

  function clamp(value, low, high) {
    return Math.min(Math.max(value, low), high);
  }

  /**
   * The burst of `icons` on a canvas of `{width, height}` over `durationMs`: 18 to 24 icons, 32 to 96 pixels at
   * 1080 pixels high, that start at the bottom center and fly up and outward in a fan, each with where it starts
   * (its top left), how far it travels, its color, turn, and timing. Every icon lands inside the canvas and ends
   * within the duration. `seed` picks the layout.
   */
  function planBurst(icons, { width, height }, seed, durationMs) {
    const random = generator(seed);
    const between = (low, high) => low + (high - low) * random();
    const pick = (list) => list[Math.floor(random() * list.length)];
    const scale = height / 1080;
    const count = 18 + Math.floor(random() * 7);
    const items = [];
    for (let index = 0; index < count; index += 1) {
      const size = between(32, 96) * scale;
      const startX = width / 2 + between(-0.03, 0.03) * width;
      const startY = height - size / 2 - between(0, 0.03) * height;
      const angle = ((-90 + between(-FAN, FAN)) * Math.PI) / 180;
      const reach = between(0.35, 0.85) * height;
      const endX = clamp(startX + Math.cos(angle) * reach, size / 2, width - size / 2);
      const endY = clamp(startY + Math.sin(angle) * reach, size / 2, height - size / 2);
      const delay = Math.round(between(0, 0.2) * durationMs);
      items.push({
        motion: "burst",
        icon: pick(icons),
        color: pick(COLORS),
        x: startX - size / 2,
        y: startY - size / 2,
        size,
        dx: endX - startX,
        dy: endY - startY,
        rotate: between(-35, 35),
        spin: between(-120, 120),
        delay,
        duration: Math.floor(between(0.75, 1) * (durationMs - delay)),
      });
    }
    return items;
  }

  /** The burst of the preset `preset` on a canvas of `size`, over the preset's length. */
  function planPreset(preset, size, seed) {
    return planBurst([iconOf(preset.id)], size, seed, preset.duration_ms);
  }

  function round(value) {
    return Math.round(value * 10) / 10;
  }

  /**
   * The Web Animations keyframes of one planned icon: it grows in at the bottom, flies out, and fades by its end;
   * with `reduced` motion it stays where it lands and only fades in and out.
   */
  function burstKeyframes(item, reduced) {
    const at = (share, turn, scale) =>
      `translate(${round(item.dx * share)}px, ${round(item.dy * share)}px) rotate(${round(turn)}deg) scale(${scale})`;
    if (reduced) {
      const still = at(1, item.rotate, 1);
      return [
        { transform: still, opacity: 0 },
        { transform: still, opacity: 1, offset: 0.25 },
        { transform: still, opacity: 1, offset: 0.7 },
        { transform: still, opacity: 0 },
      ];
    }
    return [
      { transform: at(0, item.rotate, 0.4), opacity: 0 },
      { transform: at(0.15, item.rotate + item.spin * 0.15, 1), opacity: 1, offset: 0.1 },
      { transform: at(0.8, item.rotate + item.spin * 0.8, 1), opacity: 1, offset: 0.65 },
      { transform: at(1, item.rotate + item.spin, 1), opacity: 0 },
    ];
  }

  /** An inline SVG of one planned icon in `doc`, stroked and lightly filled in its color. */
  function createParticle(doc, item, setStyles) {
    const svg = doc.createElementNS(SVG, "svg");
    svg.classList.add("sn-flood-icon");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("width", String(round(item.size)));
    svg.setAttribute("height", String(round(item.size)));
    svg.setAttribute("data-icon", item.icon);
    const paint = {
      fill: "currentColor",
      "fill-opacity": "0.22",
      stroke: "currentColor",
      "stroke-width": String(Icons.STROKE_WIDTH + 0.25),
      "stroke-linecap": "round",
      "stroke-linejoin": "round",
    };
    for (const [key, value] of Object.entries(paint)) {
      svg.setAttribute(key, value);
    }
    for (const [tag, attributes] of Icons.ICONS[item.icon] || []) {
      const node = doc.createElementNS(SVG, tag);
      for (const [key, value] of Object.entries(attributes)) {
        node.setAttribute(key, String(value));
      }
      svg.appendChild(node);
    }
    setStyles(svg, {
      position: "absolute",
      left: `${round(item.x)}px`,
      top: `${round(item.y)}px`,
      width: `${round(item.size)}px`,
      height: `${round(item.size)}px`,
      color: item.color,
      overflow: "visible",
      "will-change": "transform, opacity",
    });
    return svg;
  }

  /** Draws the burst `items` into `layer`, animating each one; `reduced` keeps them still. */
  function drawBurst(layer, items, reduced, setStyles) {
    const doc = layer.ownerDocument;
    for (const item of items) {
      const node = createParticle(doc, item, setStyles);
      layer.appendChild(node);
      if (typeof node.animate === "function") {
        const timing = { duration: item.duration, delay: item.delay, fill: "both" };
        node.animate(burstKeyframes(item, reduced), { ...timing, easing: reduced ? "ease-in-out" : EASING });
      }
    }
  }

  /** Whether the viewer of `doc` asked for reduced motion. */
  function prefersReducedMotion(doc) {
    const view = doc && doc.defaultView;
    return Boolean(view && typeof view.matchMedia === "function" &&
      view.matchMedia("(prefers-reduced-motion: reduce)").matches);
  }

  const api = {
    COLORS,
    ICON_BY_PRESET,
    burstKeyframes,
    drawBurst,
    generator,
    iconOf,
    planBurst,
    planPreset,
    prefersReducedMotion,
  };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronFloodBurst = api;
  }
})();
