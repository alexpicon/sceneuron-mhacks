// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay renderer, shared by the overlay page and the editor preview: keeps one positioned
   node per element in the DOM, runs its animations, draws the snapshot's theme and subtitle bars, and counts the time
   on air. Needs render-style.js, render-themes.js, render-fit.js, render-icons.js, render-cards.js,
   render-clock.js, and render-subtitles.js, loaded before it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Style = inNode ? require("./render-style.js") : window.SceneuronRender;
  if (!Style || typeof Style.computeStyle !== "function") {
    throw new Error("Sceneuron render.js needs render-style.js, loaded before it.");
  }
  const { computeStyle, fitTransform, planElement, px, round, withDefaults } = Style;
  const Themes = inNode ? require("./render-themes.js") : window.SceneuronThemes;
  const Fit = inNode ? require("./render-fit.js") : window.SceneuronFit;
  const Tiles = inNode ? require("./render-icons.js") : window.SceneuronIconTiles;
  const Cards = inNode ? require("./render-cards.js") : window.SceneuronCards;
  const Clock = inNode ? require("./render-clock.js") : window.SceneuronClock;
  const Subtitles = inNode ? require("./render-subtitles.js") : window.SceneuronSubtitles;
  if (!Themes || !Fit || !Tiles || !Cards || !Clock || !Subtitles) {
    const needs = "render-themes.js, render-fit.js, render-icons.js, render-cards.js, render-clock.js, "
      + "and render-subtitles.js";
    throw new Error(`Sceneuron render.js needs ${needs} first.`);
  }

  const KINDS = ["text", "box", "frame"];
  // Keys that position the outer element; every other computed key styles the visible body.
  const OUTER_KEYS = new Set(["left", "top", "width", "height", "display", "--sn-accent", "--sn-h"]);

  const STYLESHEET = `
.sn-root{position:absolute;left:0;top:0;overflow:hidden;transform-origin:0 0}
.sn-el{position:absolute;box-sizing:border-box}
.sn-layer,.sn-body{position:absolute;top:0;left:0;right:0;bottom:0}
.sn-layer{transform-origin:50% 50%}
.sn-body{box-sizing:border-box;display:flex;align-items:center;overflow:hidden;line-height:1.15;
white-space:pre-wrap;overflow-wrap:anywhere;
font-family:system-ui,-apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif}
.sn-kind-text .sn-body{padding:0 .4em}
.sn-text{flex:1 1 auto;min-width:0}
.sn-ghost{opacity:.4}
.sn-enter-fade{animation:sn-enter-fade .45s ease-out both}
.sn-enter-slide-up{animation:sn-enter-slide-up .55s cubic-bezier(.2,.8,.2,1) both}
.sn-enter-pop{animation:sn-enter-pop .5s cubic-bezier(.2,.9,.3,1.3) both}
.sn-idle-pulse{animation:sn-idle-pulse 2.4s ease-in-out infinite}
.sn-idle-glow{animation:sn-idle-glow 1.6s ease-in-out infinite alternate}
.sn-idle-float{animation:sn-idle-float 3.2s ease-in-out infinite}
.sn-fx-shake{animation:sn-fx-shake .6s ease-in-out}
.sn-fx-pop{animation:sn-fx-pop .45s ease-out}
.sn-fx-flash{animation:sn-fx-flash .7s linear}
.sn-fx-bounce{animation:sn-fx-bounce .8s ease-out}
@keyframes sn-enter-fade{from{opacity:0}to{opacity:1}}
@keyframes sn-enter-slide-up{from{opacity:0;transform:translateY(60px)}to{opacity:1;transform:none}}
@keyframes sn-enter-pop{0%{opacity:0;transform:scale(.6)}70%{opacity:1;transform:scale(1.06)}
100%{opacity:1;transform:scale(1)}}
@keyframes sn-idle-pulse{0%,100%{transform:scale(1)}50%{transform:scale(1.04)}}
@keyframes sn-idle-glow{from{filter:drop-shadow(0 0 2px var(--sn-accent))}
to{filter:drop-shadow(0 0 22px var(--sn-accent))}}
@keyframes sn-idle-float{0%,100%{transform:translateY(0)}50%{transform:translateY(-12px)}}
@keyframes sn-fx-shake{0%,100%{transform:translateX(0)}15%,45%,75%{transform:translateX(-14px)}
30%,60%,90%{transform:translateX(14px)}}
@keyframes sn-fx-pop{0%,100%{transform:scale(1)}40%{transform:scale(1.2)}}
@keyframes sn-fx-flash{0%,50%,100%{opacity:1;filter:brightness(1)}25%,75%{opacity:.2;filter:brightness(2)}}
@keyframes sn-fx-bounce{0%,100%{transform:translateY(0)}30%{transform:translateY(-40px)}55%{transform:translateY(0)}
75%{transform:translateY(-16px)}}
`;

  function applyFit(root, canvas, width, height) {
    const fit = fitTransform(canvas, width, height);
    root.style.setProperty("width", px(fit.width));
    root.style.setProperty("height", px(fit.height));
    root.style.setProperty("transform", `translate(${px(fit.x)}, ${px(fit.y)}) scale(${round(fit.scale, 5)})`);
    return fit;
  }

  const styledDocuments = new WeakSet();

  // A constructed stylesheet is used because the pages' CSP blocks inline <style> elements.
  function ensureStyles(doc) {
    if (!doc || styledDocuments.has(doc)) {
      return;
    }
    styledDocuments.add(doc);
    const Sheet = doc.defaultView && doc.defaultView.CSSStyleSheet;
    if (Sheet && "adoptedStyleSheets" in doc) {
      const sheet = new Sheet();
      sheet.replaceSync(STYLESHEET + Subtitles.STYLESHEET + Themes.STYLESHEET);
      doc.adoptedStyleSheets = [...doc.adoptedStyleSheets, sheet];
      return;
    }
    const style = doc.createElement("style");
    style.setAttribute("data-sceneuron-render", "");
    style.textContent = STYLESHEET + Subtitles.STYLESHEET + Themes.STYLESHEET;
    (doc.head || doc.documentElement).appendChild(style);
  }

  function setSlot(state, slot, layer, cls, restart) {
    const current = state[slot];
    if (current && (current !== cls || restart)) {
      layer.classList.remove(current);
    }
    if (cls && (current !== cls || restart)) {
      if (restart) {
        void layer.offsetWidth;
      }
      layer.classList.add(cls);
    }
    state[slot] = cls;
  }

  function createNodes(doc, id) {
    const make = (cls, parent) => {
      const node = doc.createElement(cls === "sn-text" ? "span" : "div");
      node.classList.add(...cls.split(" "));
      if (parent) {
        parent.appendChild(node);
      }
      return node;
    };
    const node = make("sn-el");
    node.setAttribute("data-element-id", id);
    const enter = make("sn-layer sn-enter", node);
    const idle = make("sn-layer sn-idle", enter);
    const fx = make("sn-layer sn-fx", idle);
    const body = make("sn-body", fx);
    const text = make("sn-text", body);
    const state = { node, enter, fx, idle, body, text, applied: {}, previous: null, kind: "", content: null };
    state.enterCls = state.idleCls = state.fxCls = "";
    enter.addEventListener("animationend", (event) => {
      if (event.target === enter) {
        setSlot(state, "enterCls", enter, "", false);
      }
    });
    fx.addEventListener("animationend", (event) => {
      if (event.target === fx) {
        setSlot(state, "fxCls", fx, "", false);
      }
    });
    return state;
  }

  /** Updates one element's nodes; `look` holds the theme, its unit, and the deck name of each card slot. */
  function updateElement(state, element, options, look) {
    const kind = KINDS.includes(element.kind) ? element.kind : "box";
    if (state.kind !== kind) {
      if (state.kind) {
        state.node.classList.remove(`sn-kind-${state.kind}`);
      }
      state.node.classList.add(`sn-kind-${kind}`);
      state.kind = kind;
    }
    const props = withDefaults(element.props);
    const plan = planElement(state.previous, element, options);
    const style = Themes.themeStyle(computeStyle(kind, props), look.theme);
    if (plan.ghost) {
      style.display = "block";
    }
    for (const key of Object.keys(state.applied)) {
      if (!(key in style)) {
        (OUTER_KEYS.has(key) ? state.node : state.body).style.removeProperty(key);
        delete state.applied[key];
      }
    }
    for (const [key, value] of Object.entries(style)) {
      if (state.applied[key] !== value) {
        (OUTER_KEYS.has(key) ? state.node : state.body).style.setProperty(key, value);
        state.applied[key] = value;
      }
    }
    state.node.classList.toggle("sn-ghost", plan.ghost);
    // Marks a hidden element, so a theme can play its way out before it goes.
    const gone = !plan.visible && !plan.ghost;
    state.node.classList.toggle("sn-gone", gone);
    const text = kind === "text";
    const words = text ? Tiles.words(element, props.text, look.tiles) : "";
    const content = text && element.clock ? Clock.fillTime(words, look.onAirMs) : words;
    const font = Themes.themeById(look.theme).fonts[0];
    // A hidden element keeps the words it last showed, so a lowered card fades out as it was, not emptied.
    if (!gone || state.shownWords === undefined) {
      Subtitles.fill(state, content, text ? Subtitles.barOf(element) : null, font);
      state.shownWords = content;
    }
    Themes.decorate(state, element, look.theme, look.decks);
    Tiles.decorate(state, element, props.text, look.tiles);
    Cards.decorate(state, element, props.text, look.cards);
    if (plan.restartEnter) {
      setSlot(state, "enterCls", state.enter, plan.enterClass, true);
    } else if (!plan.visible) {
      setSlot(state, "enterCls", state.enter, "", false);
    }
    setSlot(state, "idleCls", state.idle, plan.idleClass, false);
    if (plan.playEffect && plan.effectClass) {
      setSlot(state, "fxCls", state.fx, plan.effectClass, true);
    } else if (!plan.visible) {
      setSlot(state, "fxCls", state.fx, "", false);
    }
    state.previous = { visible: plan.visible, nonce: plan.nonce };
  }

  const rootStates = new WeakMap();

  /**
   * Creates or updates one absolutely positioned div per snapshot element inside `root`.
   * `options.ghostHidden` draws hidden elements faintly instead of removing them, for editing.
   */
  function render(root, snapshot, options) {
    const doc = root.ownerDocument;
    ensureStyles(doc);
    root.classList.add("sn-root");
    let states = rootStates.get(root);
    if (!states) {
      states = new Map();
      rootStates.set(root, states);
    }
    const look = { theme: Themes.themeId(snapshot && snapshot.theme), decks: Themes.deckNames(snapshot) };
    look.unit = Themes.applyTheme(root, look.theme, snapshot && snapshot.canvas);
    look.lines = Fit.subtitleLines(snapshot);
    look.tiles = Tiles.iconLook(snapshot, Themes.themeById(look.theme));
    look.cards = Cards.cardsLook(snapshot, look.theme, look.tiles.slots);
    look.onAirMs = Clock.onAirMs(root, snapshot, options);
    const elements = snapshot && Array.isArray(snapshot.elements) ? snapshot.elements : [];
    const seen = new Set();
    let previousNode = null;
    let counting = false;
    for (const element of elements) {
      if (!element || typeof element.id !== "string" || seen.has(element.id)) {
        continue;
      }
      seen.add(element.id);
      let state = states.get(element.id);
      if (!state) {
        state = createNodes(doc, element.id);
        states.set(element.id, state);
      }
      updateElement(state, element, options || {}, look);
      const expected = previousNode ? previousNode.nextSibling : root.firstChild;
      if (expected !== state.node) {
        root.insertBefore(state.node, expected);
      }
      // Text is measured in place, once the node is in the page.
      Fit.fitText(state, look, element.subtitles === true);
      previousNode = state.node;
      counting = counting || (element.clock === true && state.previous.visible);
    }
    for (const [id, state] of states) {
      if (!seen.has(id)) {
        state.node.remove();
        states.delete(id);
      }
    }
    Clock.keepCounting(root, look.onAirMs, counting, () => render(root, snapshot, options));
  }

  const api = {
    DEFAULT_PROPS: Style.DEFAULT_PROPS,
    STYLESHEET,
    applyFit,
    computeStyle,
    cssColor: Style.cssColor,
    effectClass: Style.effectClass,
    enterClass: Style.enterClass,
    fitTransform,
    idleClass: Style.idleClass,
    parseColor: Style.parseColor,
    planElement,
    render,
    withDefaults,
  };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronRender = api;
  }
})();
