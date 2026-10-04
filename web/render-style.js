// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay styles: default properties, colors, computed element styles, animation classes and
   plans, and canvas fitting. Pure functions shared by render.js, the editor, and the Node tests. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  const DEFAULT_PROPS = Object.freeze({
    x: 0,
    y: 0,
    width: 480,
    height: 120,
    visible: true,
    text: "",
    font_size: 48,
    bold: true,
    color: "#ffffff",
    background: "#00000000",
    border_color: "#00000000",
    border_width: 0,
    corner_radius: 0,
    opacity: 1,
    align: "center",
    idle: "none",
    enter: "fade",
  });

  const ENTER_CLASSES = { fade: "sn-enter-fade", "slide-up": "sn-enter-slide-up", pop: "sn-enter-pop" };
  const IDLE_CLASSES = { pulse: "sn-idle-pulse", glow: "sn-idle-glow", float: "sn-idle-float" };
  const EFFECT_CLASSES = { shake: "sn-fx-shake", pop: "sn-fx-pop", flash: "sn-fx-flash", bounce: "sn-fx-bounce" };
  const ALIGNMENTS = ["left", "center", "right"];

  function num(value, fallback) {
    return typeof value === "number" && Number.isFinite(value) ? value : fallback;
  }

  function round(value, places) {
    const factor = 10 ** places;
    return Math.round(value * factor) / factor;
  }

  function px(value) {
    return `${round(value, 2)}px`;
  }

  function withDefaults(props) {
    return Object.assign({}, DEFAULT_PROPS, props || {});
  }

  /** Parses `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa` into 0-255 channels and a 0-1 alpha. */
  function parseColor(value) {
    const match = typeof value === "string" ? /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(value.trim()) : null;
    if (!match) {
      return null;
    }
    let hex = match[1];
    if (hex.length <= 4) {
      hex = Array.from(hex, (digit) => digit + digit).join("");
    }
    const byte = (index) => parseInt(hex.slice(index, index + 2), 16);
    return { r: byte(0), g: byte(2), b: byte(4), a: hex.length === 8 ? byte(6) / 255 : 1 };
  }

  function cssColor(value, fallback) {
    const color = parseColor(value);
    if (!color) {
      return fallback === undefined ? "transparent" : fallback;
    }
    return `rgba(${color.r}, ${color.g}, ${color.b}, ${round(color.a, 3)})`;
  }

  /** The opaque color used for glows: a visible border, else the fill, else the text color. */
  function accentColor(kind, props) {
    const candidates = [];
    if (num(props.border_width, 0) > 0) {
      candidates.push(props.border_color);
    }
    if (kind !== "frame") {
      candidates.push(props.background);
    }
    candidates.push(props.color);
    for (const candidate of candidates) {
      const color = parseColor(candidate);
      if (color && color.a > 0) {
        return `rgba(${color.r}, ${color.g}, ${color.b}, 1)`;
      }
    }
    return "rgba(255, 255, 255, 1)";
  }

  /** Maps element kind and properties to CSS property names and string values. */
  function computeStyle(kind, props) {
    const p = withDefaults(props);
    const borderWidth = Math.max(0, num(p.border_width, 0));
    const style = {
      left: px(num(p.x, 0)),
      top: px(num(p.y, 0)),
      width: px(Math.max(0, num(p.width, DEFAULT_PROPS.width))),
      height: px(Math.max(0, num(p.height, DEFAULT_PROPS.height))),
      display: p.visible === false ? "none" : "block",
      "--sn-accent": accentColor(kind, p),
      opacity: String(round(Math.min(1, Math.max(0, num(p.opacity, 1))), 3)),
      "background-color": kind === "frame" ? "transparent" : cssColor(p.background),
      border: borderWidth > 0 ? `${px(borderWidth)} solid ${cssColor(p.border_color)}` : "none",
      "border-radius": px(Math.max(0, num(p.corner_radius, 0))),
    };
    if (kind === "text") {
      style.color = cssColor(p.color, "rgba(255, 255, 255, 1)");
      style["font-size"] = px(Math.max(1, num(p.font_size, DEFAULT_PROPS.font_size)));
      style["font-weight"] = p.bold === false ? "400" : "700";
      style["text-align"] = ALIGNMENTS.includes(p.align) ? p.align : "center";
    }
    return style;
  }

  function lookup(table, key) {
    return Object.prototype.hasOwnProperty.call(table, key) ? table[key] : "";
  }

  const enterClass = (enter) => lookup(ENTER_CLASSES, enter);
  const idleClass = (idle) => lookup(IDLE_CLASSES, idle);
  const effectClass = (effect) => lookup(EFFECT_CLASSES, effect);

  /**
   * Decides which animations an element runs, given what the previous render showed.
   * Enter animations play when an element becomes visible (including its first render);
   * effects replay only when the nonce changes after the first render, so reloading a page
   * never replays an old effect.
   */
  function planElement(previous, element, options) {
    const props = withDefaults(element && element.props);
    const visible = props.visible !== false;
    const effect = element && element.effect;
    const nonce = effect && Number.isFinite(effect.nonce) ? effect.nonce : null;
    return {
      visible,
      ghost: !visible && Boolean(options && options.ghostHidden),
      enterClass: visible ? enterClass(props.enter) : "",
      restartEnter: visible && (!previous || !previous.visible),
      idleClass: visible ? idleClass(props.idle) : "",
      effectClass: effect ? effectClass(effect.effect) : "",
      playEffect: Boolean(previous) && visible && nonce !== null && nonce !== previous.nonce,
      nonce,
    };
  }

  /** Scale and offsets that fit a canvas inside a viewport, centered. */
  function fitTransform(canvas, width, height) {
    const canvasWidth = Math.max(1, num(canvas && canvas.width, 1920));
    const canvasHeight = Math.max(1, num(canvas && canvas.height, 1080));
    const viewWidth = Math.max(0, num(width, 0));
    const viewHeight = Math.max(0, num(height, 0));
    const scale = Math.min(viewWidth / canvasWidth, viewHeight / canvasHeight);
    // Offsets are never negative; clamping removes floating-point noise such as -5e-14.
    return {
      scale,
      x: Math.max(0, (viewWidth - canvasWidth * scale) / 2),
      y: Math.max(0, (viewHeight - canvasHeight * scale) / 2),
      width: canvasWidth,
      height: canvasHeight,
    };
  }

  // render.js builds the public SceneuronRender API from these; `px` and `round` format its styles.
  const style = {
    DEFAULT_PROPS,
    computeStyle,
    cssColor,
    effectClass,
    enterClass,
    fitTransform,
    idleClass,
    parseColor,
    planElement,
    px,
    round,
    withDefaults,
  };
  if (inNode) {
    module.exports = style;
  } else {
    window.SceneuronRender = style;
  }
})();
