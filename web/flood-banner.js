// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay floods: the banner line a preset such as "This meeting could have been an email" shows large in
   the middle of the canvas, between its first and last emoji, above the flood's emojis. It pops in, holds, and fades
   out within the flood. Planned by pure functions that the Node tests check; flood.js draws it. */
(function () {
  "use strict";

  const FONT =
    '"Helvetica Neue","Segoe UI",Arial,"Apple Color Emoji","Segoe UI Emoji","Noto Color Emoji",sans-serif';
  /** The font size, as a share of the canvas height; 86 pixels on a 1080-pixel canvas. */
  const FONT_SHARE = 0.08;
  /** The widest banner, as a share of the canvas width; longer lines wrap. */
  const WIDTH_SHARE = 0.8;
  /** Estimated advance of a letter and of an emoji, in ems, for a bold face; generous so a line rarely wraps. */
  const LETTER_EM = 0.6;
  const EMOJI_EM = 1.3;

  function round(value) {
    return Math.round(value * 10) / 10;
  }

  /**
   * The banner of `preset` on a canvas of `size` (`{width, height}` in pixels), or null when the preset shows none:
   * its text, between the preset's first and last emoji, the box's width and left edge, centered across the canvas,
   * and its font size, padding, corners, border, and colors.
   */
  function planBanner(preset, size) {
    if (!preset || typeof preset.banner !== "string" || !preset.banner) {
      return null;
    }
    const emojis = Array.isArray(preset.emojis) ? preset.emojis : [];
    const first = emojis[0] || "";
    const last = emojis.length > 1 ? emojis[emojis.length - 1] : "";
    const text = [first, preset.banner, last].filter(Boolean).join(" ");
    const fontSize = round(size.height * FONT_SHARE);
    const paddingX = round(fontSize * 0.55);
    const paddingY = round(fontSize * 0.3);
    const ems = [...preset.banner].length * LETTER_EM + [first, last].filter(Boolean).length * (EMOJI_EM + 0.3);
    const width = round(Math.min(size.width * WIDTH_SHARE, ems * fontSize + 2 * paddingX));
    return {
      text,
      width,
      left: round((size.width - width) / 2),
      fontSize,
      paddingX,
      paddingY,
      radius: round(fontSize * 0.4),
      border: round(Math.max(2, fontSize * 0.07)),
      background: /^#[0-9a-f]{6}$/i.test(preset.color) ? preset.color : "#1d4ed8",
    };
  }

  /**
   * The banner's Web Animations keyframes: it pops in tilted, settles, holds, and shrinks away fading. Each keyframe
   * eases the part after it, and the animation itself runs linearly, so the offsets are fractions of the time.
   */
  function bannerKeyframes() {
    const at = (scale, degrees) => `translateY(-50%) scale(${scale}) rotate(${degrees}deg)`;
    return [
      { transform: at(0.2, -8), opacity: 0, easing: "cubic-bezier(.2,.8,.3,1)" },
      { transform: at(1.08, 2), opacity: 1, offset: 0.12, easing: "ease-in-out" },
      { transform: at(1, -1), opacity: 1, offset: 0.2 },
      { transform: at(1, -1), opacity: 1, offset: 0.85, easing: "ease-in" },
      { transform: at(0.9, 0), opacity: 0 },
    ];
  }

  /**
   * A banner node for `plan` in `doc`, animated over `duration` ms; `setStyles(node, styles)` applies its styles. A
   * theme may give it its own panel, edge, and text colors through --sn-banner-panel, --sn-banner-edge, and
   * --sn-banner-text.
   */
  function createBanner(doc, plan, duration, setStyles) {
    const node = doc.createElement("div");
    node.classList.add("sn-flood-banner");
    node.textContent = plan.text;
    setStyles(node, {
      position: "absolute",
      left: `${plan.left}px`,
      top: plan.top === undefined ? "50%" : `${plan.top}px`,
      width: `${plan.width}px`,
      "box-sizing": "border-box",
      padding: `${plan.paddingY}px ${plan.paddingX}px`,
      "font-family": FONT,
      "font-size": `${plan.fontSize}px`,
      "font-weight": "800",
      "line-height": "1.15",
      "text-align": "center",
      // Lines of about the same length where the browser can; others break as usual.
      "text-wrap": "balance",
      color: plan.color || "var(--sn-banner-text, #ffffff)",
      background: plan.edge ? plan.background : `var(--sn-banner-panel, ${plan.background})`,
      border: `${plan.border}px solid ${plan.edge || "var(--sn-banner-edge, #ffffff)"}`,
      "border-radius": `${plan.radius}px`,
      "box-shadow": `0 ${round(plan.fontSize * 0.15)}px ${round(plan.fontSize * 0.6)}px rgba(0, 0, 0, 0.55)`,
      "text-shadow": plan.edge ? "none" : "0 3px 10px rgba(0, 0, 0, 0.45)",
      transform: "translateY(-50%)",
      "will-change": "transform, opacity",
    });
    if (typeof node.animate === "function") {
      node.animate(bannerKeyframes(), { duration, fill: "both" });
    }
    return node;
  }

  const api = { bannerKeyframes, createBanner, planBanner };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronFloodBanner = api;
  }
})();
