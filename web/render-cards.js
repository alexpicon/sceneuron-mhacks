// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay card looks of the Graphite theme: a planning poker card is a playing card, face down with an
   amber "Voted" tab while its deck's cover hides it and flipped face up, on the same node, in the render that
   uncovers it, so every overlay drawing one room flips together; a status card is a compact pill with a colored
   dot; a classroom card is a speech bubble whose tail points toward the frame's center, and a quiz answer shows
   its letter large. The newest card pulses once when it is raised. Which look a card gets is decided by cardLook,
   a pure function; themes/graphite.js draws the looks. Needs icons.js and render-icons.js, loaded before it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Icons = inNode ? require("./icons.js") : window.SceneuronIcons;
  const Tiles = inNode ? require("./render-icons.js") : window.SceneuronIconTiles;
  const Results = inNode ? require("./render-results.js") : window.SceneuronResults;
  if (!Icons || !Tiles || typeof Tiles.iconSvg !== "function" || !Results) {
    throw new Error("Sceneuron render-cards.js needs icons.js, render-icons.js, and render-results.js, loaded before it.");
  }

  const THEME = "graphite";
  const CAPTION = "STORY POINTS";
  const BREAK_CAPTION = "COFFEE BREAK";
  const ANSWER_CAPTION = "ANSWER";
  const DEFAULT_WIDTH = 1920;
  // The dot of each built-in status card: red while busy elsewhere, amber while away, gray while quiet.
  const DOTS = { presenting: "red", "on-call": "red", brb: "amber", lunch: "amber", muted: "grey", focus: "grey" };
  // Every class this module may set on an element's node, so a card that changes look clears the old one.
  const CLASSES = ["sn-gp-poker", "sn-gp-status", "sn-gp-bubble", "sn-gp-quiz", "sn-gp-tail-left"]
    .concat(["sn-gp-tail-right", "sn-gp-dot-red", "sn-gp-dot-amber", "sn-gp-dot-grey"]);
  const PULSE = "sn-gp-pulse";

  const own = (object, key) => Object.prototype.hasOwnProperty.call(object, key);

  /** The icon a coffee break shows: the mug when icons.js draws one, else the coffee cup. */
  function breakIcon() {
    return own(Icons.ICONS, "mug") ? "mug" : "coffee";
  }

  /**
   * The words on a face-down card's tab: the room's summary with its colon as a middle dot, such as
   * "Voted · 3 of 5", or else the cover's words without a leading check mark.
   */
  function tabText(summary, cover) {
    const said = String(summary || "").trim();
    if (said) {
      const parts = /^([^:]+):\s*(.+)$/.exec(said);
      return parts ? `${parts[1].trim()} · ${parts[2]}` : said;
    }
    const words = String(cover || "").replace(/^[\s✓✔✅]+/u, "").trim();
    return words || "Voted";
  }

  /** Which end of a bubble's bottom edge its tail hangs from: the one nearer the frame's center. */
  function tailSide(props, canvas) {
    const width = canvas && Number.isFinite(canvas.width) && canvas.width > 0 ? canvas.width : DEFAULT_WIDTH;
    const x = Number(props && props.x) || 0;
    const w = Number(props && props.width) || 0;
    return x + w / 2 <= width / 2 ? "right" : "left";
  }

  /**
   * The Graphite look of a card slot showing `slot` with words `text`, or null for any other theme, an empty slot,
   * or a deck without a look of its own. `kind` is "poker" (with `face` "back" and its `tab`, or "front" with its
   * `value`, `icon`, and `caption`), "status" (with its `dot`), "bubble" (with its `tail`), or "quiz" (with its
   * `letter` and `tail`).
   */
  function cardLook(theme, slot, text, props, canvas) {
    if (theme !== THEME || !slot || !slot.card) {
      return null;
    }
    const words = String(text);
    if (slot.deck === "poker") {
      if (slot.covered) {
        return { kind: "poker", face: "back", tab: tabText(slot.summary, words) };
      }
      const shown = { results: Results.results(slot.summary), side: tailSide(props, canvas) };
      if (slot.card === "break") {
        return { kind: "poker", face: "front", value: "", icon: breakIcon(), caption: BREAK_CAPTION, ...shown };
      }
      const lead = Icons.leadingEmoji(words);
      const value = slot.card === "unsure" ? "?" : lead ? lead.rest : words.trim();
      return { kind: "poker", face: "front", value, icon: null, caption: CAPTION, ...shown };
    }
    if (slot.covered) {
      return null;
    }
    if (slot.deck === "status") {
      return { kind: "status", dot: own(DOTS, slot.card) ? DOTS[slot.card] : "grey" };
    }
    if (slot.deck === "classroom") {
      const tail = tailSide(props, canvas);
      return /^answer-/.test(slot.card) ? { kind: "quiz", letter: words.trim(), tail } : { kind: "bubble", tail };
    }
    return null;
  }

  /** The classes an element's node takes for `card`. */
  function classesOf(card) {
    if (!card) {
      return [];
    }
    if (card.kind === "status") {
      return ["sn-gp-status", `sn-gp-dot-${card.dot}`];
    }
    if (card.kind === "bubble" || card.kind === "quiz") {
      const bubble = ["sn-gp-bubble", `sn-gp-tail-${card.tail}`];
      return card.kind === "quiz" ? bubble.concat("sn-gp-quiz") : bubble;
    }
    return ["sn-gp-poker"];
  }

  function make(doc, tag, cls, parent) {
    const node = doc.createElement(tag);
    node.classList.add(...cls.split(" "));
    if (parent) {
      parent.appendChild(node);
    }
    return node;
  }

  /** A playing card's two faces on one node that turns over. */
  function makeCard(doc) {
    const node = make(doc, "div", "sn-gp-flipper");
    const back = make(doc, "div", "sn-gp-face sn-gp-back", node);
    make(doc, "span", "sn-gp-mono", back).textContent = "S";
    const tab = make(doc, "span", "sn-gp-tab", back);
    tab.appendChild(Tiles.iconSvg(doc, "check"));
    const front = make(doc, "div", "sn-gp-face sn-gp-front", node);
    return {
      kind: "poker",
      node,
      tab: make(doc, "span", "sn-gp-tab-text", tab),
      value: make(doc, "span", "sn-gp-value", front),
      caption: make(doc, "span", "sn-gp-caption", front),
      faceKey: null,
      // Beside the card, not on it, so it does not turn over with it.
      results: make(doc, "div", "sn-gp-results"),
      resultsKey: "",
    };
  }

  /** A quiz answer's large letter over its caption. */
  function makeAnswer(doc) {
    const node = make(doc, "div", "sn-gp-answer");
    const letter = make(doc, "span", "sn-gp-letter", node);
    make(doc, "span", "sn-gp-caption", node).textContent = ANSWER_CAPTION;
    return { kind: "quiz", node, letter };
  }

  /**
   * Shows a poker card's face. The front's words are written only once it is uncovered, so a covered card's value
   * is never in the page, and turning it face up in the same render starts the flip.
   */
  function fillCard(parts, card) {
    const doc = parts.node.ownerDocument;
    const up = card.face === "front";
    if (!up) {
      if (parts.tab.textContent !== card.tab) {
        parts.tab.textContent = card.tab;
      }
      if (parts.faceKey !== null) {
        parts.value.textContent = "";
        parts.caption.textContent = "";
        parts.faceKey = null;
      }
    } else {
      const key = `${card.value}|${card.icon || ""}|${card.caption}`;
      if (parts.faceKey !== key) {
        parts.faceKey = key;
        parts.value.textContent = card.value;
        if (card.icon) {
          parts.value.appendChild(Tiles.iconSvg(doc, card.icon));
        }
        parts.caption.textContent = card.caption;
      }
    }
    parts.node.classList.toggle("sn-gp-up", up);
    fillResults(parts, up ? card : null);
  }

  /** Lists each revealed vote and the average beside the card, on the side toward the frame's center. */
  function fillResults(parts, card) {
    const shown = card && card.results;
    const key = shown ? JSON.stringify([shown, card.side]) : "";
    if (parts.resultsKey === key) {
      return;
    }
    parts.resultsKey = key;
    const box = parts.results;
    const doc = box.ownerDocument;
    box.textContent = "";
    box.classList.toggle("sn-gp-on", Boolean(shown));
    box.classList.toggle("sn-gp-results-left", Boolean(shown) && card.side === "left");
    box.classList.toggle("sn-gp-results-right", Boolean(shown) && card.side !== "left");
    if (!shown) {
      return;
    }
    for (const row of shown.rows) {
      const line = make(doc, "div", "sn-gp-row", box);
      make(doc, "span", "sn-gp-who", line).textContent = row.who;
      make(doc, "span", "sn-gp-vote", line).textContent = row.vote;
    }
    if (shown.average !== null) {
      const line = make(doc, "div", "sn-gp-row sn-gp-average", box);
      make(doc, "span", "sn-gp-who", line).textContent = "Average";
      make(doc, "span", "sn-gp-vote", line).textContent = shown.average;
    }
  }

  /** Keeps the element's face nodes for `card`, made on first use and dropped when the card has none. */
  function showFace(state, card) {
    const kind = card && (card.kind === "poker" || card.kind === "quiz") ? card.kind : "";
    if (state.gp && state.gp.kind !== kind) {
      state.gp.node.remove();
      if (state.gp.results) {
        state.gp.results.remove();
      }
      state.gp = null;
    }
    if (!kind) {
      return;
    }
    const fresh = !state.gp;
    if (fresh) {
      const doc = state.body.ownerDocument;
      state.gp = kind === "poker" ? makeCard(doc) : makeAnswer(doc);
      state.gp.node.setAttribute("aria-hidden", "true");
    }
    if (kind === "poker") {
      fillCard(state.gp, card);
    } else if (state.gp.letter.textContent !== card.letter) {
      state.gp.letter.textContent = card.letter;
    }
    // Appended once filled, so a card first shown face up is not seen turning over.
    if (fresh) {
      state.body.appendChild(state.gp.node);
      if (state.gp.results) {
        state.body.appendChild(state.gp.results);
      }
    }
  }

  /** Pulses the newest card once each time it is raised, though not for a card already up when the page loads. */
  function pulse(state, slot, on) {
    const at = on && slot && slot.card ? slot.at : null;
    const previous = state.gpAt;
    state.gpAt = at;
    if (!on) {
      state.node.classList.remove(PULSE);
      return;
    }
    if (previous === undefined || at === null || at === previous || !slot.active) {
      return;
    }
    state.node.classList.remove(PULSE);
    void state.node.offsetWidth;
    state.node.classList.add(PULSE);
  }

  /** Gives a card slot its Graphite look; under any other theme it has none of it. */
  function decorate(state, element, text, look) {
    const graphite = look.theme === THEME;
    const slot = graphite ? look.slots.get(element.id) : undefined;
    const now = element.kind === "text" ? cardLook(look.theme, slot, text, element.props, look.canvas) : null;
    // A lowered card keeps its look while it fades out; dropping it at once would show the slot's full-width panel.
    const lowered = graphite && slot !== undefined && !slot.card;
    const card = now || (lowered ? state.gpLast || null : null);
    state.gpLast = card;
    const classes = classesOf(card);
    for (const cls of CLASSES) {
      state.node.classList.toggle(cls, classes.includes(cls));
    }
    showFace(state, card);
    pulse(state, slot, graphite && slot !== undefined);
    Results.decorate(state, element, look.slots.get(element.id), look, THEME);
  }

  /** What a render of `snapshot` with theme `theme` needs for card looks; `slots` come from render-icons.js. */
  function cardsLook(snapshot, theme, slots) {
    return { theme, slots, canvas: snapshot && snapshot.canvas };
  }

  const api = { cardLook, cardsLook, classesOf, decorate, results: Results.results, tabText, tailSide };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronCards = api;
  }
})();
