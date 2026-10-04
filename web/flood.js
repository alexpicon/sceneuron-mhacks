// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay floods: when a snapshot carries a flood this page has not played, fills the whole canvas with
   its preset's emojis, which rain down and burst for its few seconds, above every element, with its banner line
   large in the middle when it has one (see flood-banner.js), then removes them; the Graphite theme throws a burst
   of line icons instead (see flood-burst.js). The presets come from reaction-presets.js. Shared by the overlay
   page and the editor preview; the layout is planned by pure functions that the Node tests check. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { PRESETS } = inNode ? require("./reaction-presets.js") : window.SceneuronReactionPresets;
  const Banner = inNode ? require("./flood-banner.js") : window.SceneuronFloodBanner;
  const Burst = inNode ? require("./flood-burst.js") : window.SceneuronFloodBurst;
  /** The flood plan's timings are written for this length and stretched to each preset's. */
  const PLAN_MS = 3000;
  // The rain falls in columns across the canvas, a few waves of them; bursts throw a ring of emojis outward.
  const COLUMNS = 16;
  const WAVES = 4;
  const BURSTS = 3;
  const PER_BURST = 10;
  const FONT = '"Apple Color Emoji","Segoe UI Emoji","Noto Color Emoji",sans-serif';

  /** The preset named `kind`; an unknown name shows the first preset, the tongue. */
  function presetOf(kind) {
    return PRESETS.find((preset) => preset.id === kind) || PRESETS[0];
  }

  /** How long a flood of `kind` lasts; a page that learns of a flood later than this after it began skips it. */
  function durationOf(kind) {
    return presetOf(kind).duration_ms;
  }

  function canvasSize(canvas) {
    const side = (value, fallback) => (Number.isFinite(value) && value > 0 ? value : fallback);
    return { width: side(canvas && canvas.width, 1920), height: side(canvas && canvas.height, 1080) };
  }

  /**
   * The emojis of one flood of `kind` on `canvas`, each with where it starts, how far it travels, and when: rain in
   * every column of the canvas, wave after wave, and bursts that throw emojis outward from points across its middle.
   * Every animation ends within the preset's length. `seed` picks the layout; a flood's nonce is used.
   */
  function planFlood(kind, canvas, seed) {
    const { emojis } = presetOf(kind);
    const stretch = durationOf(kind) / PLAN_MS;
    const plan = planRain(emojis, canvasSize(canvas), seed);
    return plan.map((item) => ({
      ...item,
      delay: Math.round(item.delay * stretch),
      // Rounded down, so a stretched emoji still ends within the preset's length.
      duration: Math.floor(item.duration * stretch),
    }));
  }

  /** The flood's emojis planned for PLAN_MS. */
  function planRain(emojis, { width, height }, seed) {
    const random = Burst.generator(seed);
    const between = (low, high) => low + (high - low) * random();
    const pick = () => emojis[Math.floor(random() * emojis.length)];
    const scale = Math.min(width, height) / 1080;
    const items = [];
    for (let wave = 0; wave < WAVES; wave += 1) {
      for (let column = 0; column < COLUMNS; column += 1) {
        const size = between(80, 170) * scale;
        const delay = Math.round(between(wave * 300, wave * 300 + 280));
        items.push({
          motion: "fall",
          emoji: pick(),
          x: ((column + random()) * width) / COLUMNS - size / 2,
          y: -size,
          size,
          dx: between(-0.08, 0.08) * width,
          dy: height + size * 2,
          spin: between(-540, 540),
          delay,
          duration: Math.round(between(1400, Math.min(1700, PLAN_MS - delay))),
        });
      }
    }
    for (let burst = 0; burst < BURSTS; burst += 1) {
      const centerX = between(0.15 + burst * 0.25, 0.35 + burst * 0.25) * width;
      const centerY = between(0.3, 0.7) * height;
      const delay = 250 + burst * 450;
      for (let index = 0; index < PER_BURST; index += 1) {
        const size = between(110, 210) * scale;
        const angle = ((index + random() * 0.6) / PER_BURST) * 2 * Math.PI;
        const reach = between(0.25, 0.45) * Math.max(width, height);
        items.push({
          motion: "burst",
          emoji: pick(),
          x: centerX - size / 2,
          y: centerY - size / 2,
          size,
          dx: Math.cos(angle) * reach,
          dy: Math.sin(angle) * reach,
          spin: between(-200, 200),
          delay,
          duration: Math.round(between(1300, PLAN_MS - delay)),
        });
      }
    }
    return items;
  }

  /** The Web Animations keyframes of one planned emoji. */
  function keyframes(item) {
    const end = `translate(${round(item.dx)}px, ${round(item.dy)}px)`;
    if (item.motion === "burst") {
      const early = `translate(${round(item.dx * 0.2)}px, ${round(item.dy * 0.2)}px)`;
      return [
        { transform: "translate(0px, 0px) scale(0.2) rotate(0deg)", opacity: 0 },
        { transform: `${early} scale(1) rotate(0deg)`, opacity: 1, offset: 0.2 },
        { transform: `${end} scale(1.4) rotate(${round(item.spin)}deg)`, opacity: 0 },
      ];
    }
    return [
      { transform: "translate(0px, 0px) rotate(0deg)", opacity: 1 },
      { transform: `${end} rotate(${round(item.spin)}deg)`, opacity: 1 },
    ];
  }

  function round(value) {
    return Math.round(value * 10) / 10;
  }

  /** Whether a flood that began at `flood.at_ms` is still playing at the snapshot's `nowMs`. */
  function isFresh(flood, nowMs) {
    if (!flood || !Number.isFinite(flood.at_ms) || !Number.isFinite(nowMs)) {
      return false;
    }
    return nowMs - flood.at_ms < durationOf(flood.flood);
  }

  function stop(state) {
    if (state.layer) {
      state.layer.remove();
      state.layer = null;
    }
    if (state.timer && state.clear) {
      state.clear(state.timer);
    }
    state.timer = 0;
  }

  function setStyles(node, styles) {
    for (const [name, value] of Object.entries(styles)) {
      node.style.setProperty(name, value);
    }
  }

  function play(root, state, flood, canvas, options) {
    stop(state);
    const doc = root.ownerDocument;
    const size = canvasSize(canvas);
    const { width, height } = size;
    const duration = durationOf(flood.flood);
    const layer = doc.createElement("div");
    layer.classList.add("sn-flood");
    layer.setAttribute("aria-hidden", "true");
    layer.setAttribute("data-flood", flood.flood);
    setStyles(layer, {
      position: "absolute",
      left: "0px",
      top: "0px",
      width: `${width}px`,
      height: `${height}px`,
      overflow: "hidden",
      "pointer-events": "none",
      "z-index": "2147483647",
    });
    const theme = root.getAttribute("data-theme") === "graphite" ? "graphite" : null;
    const reduced = options && "reducedMotion" in options ? options.reducedMotion : Burst.prefersReducedMotion(doc);
    const items = theme ? [] : planFlood(flood.flood, canvas, flood.nonce);
    if (theme) {
      Burst.drawBurst(layer, Burst.planPreset(presetOf(flood.flood), size, flood.nonce), reduced, setStyles);
    }
    for (const item of items) {
      const node = doc.createElement("span");
      node.textContent = item.emoji;
      setStyles(node, {
        position: "absolute",
        left: `${round(item.x)}px`,
        top: `${round(item.y)}px`,
        "font-size": `${round(item.size)}px`,
        "line-height": "1",
        "font-family": FONT,
        "will-change": "transform, opacity",
      });
      layer.appendChild(node);
      if (typeof node.animate === "function") {
        const easing = item.motion === "burst" ? "cubic-bezier(.15,.7,.3,1)" : "cubic-bezier(.4,0,.9,.55)";
        node.animate(keyframes(item), { duration: item.duration, delay: item.delay, easing, fill: "both" });
      }
    }
    const banner = Banner.planBanner(presetOf(flood.flood), size, theme);
    if (banner) {
      layer.appendChild(Banner.createBanner(doc, banner, duration, setStyles));
    }
    root.appendChild(layer);
    state.layer = layer;
    const schedule = (options && options.schedule) || ((callback, ms) => setTimeout(callback, ms));
    state.clear = (options && options.cancel) || ((timer) => clearTimeout(timer));
    state.timer = schedule(() => {
      state.timer = 0;
      stop(state);
    }, duration);
  }

  const rootStates = new WeakMap();

  /**
   * Plays the snapshot's flood inside `root`, the scaled canvas the renderer draws into, when its nonce is new to
   * this root and it is still fresh; returns whether it started one. The first snapshot a root sees never plays,
   * so loading or reloading a page never replays an old flood. A newer flood replaces one still playing.
   * `options.schedule(callback, ms)` and `options.cancel(timer)` replace setTimeout and clearTimeout, and
   * `options.reducedMotion` replaces the viewer's motion preference.
   */
  function update(root, snapshot, options) {
    const flood = snapshot && snapshot.flood;
    const nonce = flood && Number.isFinite(flood.nonce) ? flood.nonce : null;
    const state = rootStates.get(root);
    if (!state) {
      rootStates.set(root, { nonce, layer: null, timer: 0, clear: null });
      return false;
    }
    if (nonce === null || nonce === state.nonce) {
      return false;
    }
    state.nonce = nonce;
    if (!isFresh(flood, snapshot.now_ms)) {
      return false;
    }
    play(root, state, flood, snapshot.canvas, options);
    return true;
  }

  const api = { durationOf, isFresh, keyframes, planFlood, presetOf, update };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronFlood = api;
  }
})();
