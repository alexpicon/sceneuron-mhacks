// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay themes: the themes the overlay can draw and how the renderer applies one. A theme restyles
   every element through the root's data-theme attribute and its own CSS: it takes over the element's colors,
   borders, and corners, draws text whose own color has a hue in a second color, and may show decorations the
   renderer adds (a title bar with buttons, an OK button); render-fit.js shrinks text that no longer fits.
   Positions, sizes, text, and animations stay the element's own. Needs render-style.js and the theme files under
   themes/, loaded before it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Style = inNode ? require("./render-style.js") : window.SceneuronRender;
  if (!Style || typeof Style.computeStyle !== "function") {
    throw new Error("Sceneuron render-themes.js needs render-style.js, loaded before it.");
  }

  // The order the editor offers them in; sceneuron-core's Theme accepts exactly these ids.
  const THEME_IDS = Object.freeze(["none", "graphite", "hacker", "eighties", "windows-95", "classic-mac", "vaporwave"]);

  const NONE = Object.freeze({
    id: "none",
    name: "None",
    description: "Each element keeps its own colors, borders, and corners.",
    fonts: ['system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'],
    colors: { text: "#ffffff", panel: "#7c3aed" },
    readable: [],
    titles: false,
    swatch: { text: "text", panel: "panel" },
    css: "",
  });

  const definitions = inNode
    ? THEME_IDS.slice(1).map((id) => require(`./themes/${id}.js`))
    : window.SceneuronThemeDefinitions || [];
  const THEMES = Object.freeze(
    THEME_IDS.map((id) => {
      const theme = id === "none" ? NONE : definitions.find((candidate) => candidate.id === id);
      if (!theme) {
        throw new Error(`Sceneuron render-themes.js needs themes/${id}.js, loaded before it.`);
      }
      return theme;
    }),
  );

  // Decorations stay hidden unless a theme shows them. A themed panel is a stacking context, so a theme can lay
  // a pseudo-element between the panel and its text with z-index -1.
  const STYLESHEET = `
.sn-deco{display:none}
.sn-root:not([data-theme="none"]) .sn-body{z-index:0}
${THEMES.map((theme) => theme.css).join("")}`;

  // Computed styles that a theme draws itself instead of taking them from the element's properties.
  const THEMED_KEYS = ["background-color", "border", "border-radius", "color", "--sn-accent"];
  // The canvas height decorations are sized for; --sn-u scales them to the project's canvas.
  const AUTHORED_HEIGHT = 1080;
  const WIDE_RATIO = 3;
  // How far apart a text color's strongest and weakest channels are, out of 255, once it has a hue rather than
  // being white, black, or a gray.
  const HUE_SPREAD = 48;
  const BUTTONS = ["min", "max", "close"];

  /** The id of a theme the overlay draws: `value` when it names one, else "none". */
  function themeId(value) {
    return THEME_IDS.includes(value) ? value : "none";
  }

  function themeById(id) {
    return THEMES[THEME_IDS.indexOf(themeId(id))];
  }

  /**
   * Marks the root with the theme and sizes decorations to the canvas, so they keep their size on any canvas;
   * returns that size, the CSS length of one unit.
   */
  function applyTheme(root, theme, canvas) {
    if (root.getAttribute("data-theme") !== theme) {
      root.setAttribute("data-theme", theme);
    }
    const height = canvas && Number.isFinite(canvas.height) && canvas.height > 0 ? canvas.height : AUTHORED_HEIGHT;
    const unit = `${Style.round(height / AUTHORED_HEIGHT, 4)}px`;
    root.style.setProperty("--sn-u", unit);
    return unit;
  }

  /** Whether a text color has a hue, as yellow does, rather than being white, black, or a gray. */
  function hasHue(value) {
    const color = Style.parseColor(value);
    return Boolean(color) && Math.max(color.r, color.g, color.b) - Math.min(color.r, color.g, color.b) >= HUE_SPREAD;
  }

  /** The name of the deck each card slot shows, by element id, for titling dialogs. */
  function deckNames(snapshot) {
    const names = new Map();
    for (const deck of snapshot && Array.isArray(snapshot.decks) ? snapshot.decks : []) {
      if (deck && typeof deck.element === "string" && !names.has(deck.element)) {
        names.set(deck.element, typeof deck.name === "string" ? deck.name : "");
      }
    }
    return names;
  }

  /** An element's computed style without what the theme draws, plus its height for sizing decorations. */
  function themeStyle(style, theme) {
    if (theme === "none") {
      return style;
    }
    const themed = Object.assign({}, style, { "--sn-h": style.height });
    for (const key of THEMED_KEYS) {
      delete themed[key];
    }
    return themed;
  }

  function createDecorations(body) {
    const doc = body.ownerDocument;
    const make = (cls, parent, text) => {
      const node = doc.createElement(cls.includes("sn-deco") ? "div" : "span");
      node.classList.add(...cls.split(" "));
      if (text) {
        node.textContent = text;
      }
      parent.appendChild(node);
      return node;
    };
    const title = make("sn-deco sn-title", body);
    const titleText = make("sn-title-text", title);
    for (const button of BUTTONS) {
      make(`sn-button sn-button-${button}`, title);
    }
    const ok = make("sn-deco sn-ok", body, "OK");
    for (const node of [title, ok]) {
      node.setAttribute("aria-hidden", "true");
    }
    return { title, titleText, ok };
  }

  /**
   * Gives an element the decorations a theme may show: a title bar naming the element, the deck a card slot shows,
   * or the language of the subtitles a text shows, which window themes title instead of labeling, and an OK
   * button. Card slots get the sn-card class, which dialog themes draw with an icon, elements at least WIDE_RATIO
   * times as wide as tall get sn-wide, for dialogs laid out in a row, and elements whose own text color has a hue
   * get sn-alt-text, drawn in the theme's second text color, so text told apart by its color, as the subtitles
   * pack's translation is, stays apart. Without a theme the element has no decorations.
   */
  function decorate(state, element, theme, decks) {
    const themed = theme !== "none";
    const deck = decks.get(element.id);
    const { width, height, color } = Style.withDefaults(element.props);
    state.node.classList.toggle("sn-card", themed && deck !== undefined);
    state.node.classList.toggle("sn-wide", themed && width >= WIDE_RATIO * height);
    state.node.classList.toggle("sn-alt-text", themed && element.kind === "text" && hasHue(color));
    if (!themed) {
      if (state.deco) {
        state.deco.title.remove();
        state.deco.ok.remove();
        state.deco = null;
      }
      return;
    }
    if (!state.deco) {
      state.deco = createDecorations(state.body);
    }
    const bar = element.subtitle_bar;
    const language = bar && typeof bar.label === "string" ? bar.label : "";
    const title = deck || language || String(element.name || "");
    if (state.deco.titleText.textContent !== title) {
      state.deco.titleText.textContent = title;
    }
  }

  const api = {
    STYLESHEET,
    THEMES,
    THEME_IDS,
    applyTheme,
    deckNames,
    decorate,
    hasHue,
    themeById,
    themeId,
    themeStyle,
  };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronThemes = api;
  }
})();
