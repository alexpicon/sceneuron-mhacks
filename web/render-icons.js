// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay icon tiles and roles, for themes that show icons (a theme's `icons`): a card slot shows its
   raised card's icon in a tile, or a check while its deck's cover hides the card, as for a shared room's vote, and
   the newest card is marked active; a text element that is not a card shows a starter element's icon or its own
   leading emoji, and the words lose the emoji a tile shows. Elements are also marked by role, so such a theme can
   draw cards, vote cards, labels, subtitles, and the on-air pill and timer each their own way. The icons and which
   card shows which come from icons.js. Needs icons.js, loaded before it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Icons = inNode ? require("./icons.js") : window.SceneuronIcons;
  if (!Icons || typeof Icons.cardIcon !== "function") {
    throw new Error("Sceneuron render-icons.js needs icons.js, loaded before it.");
  }

  const SVG = "http://www.w3.org/2000/svg";
  const STROKE = {
    fill: "none",
    stroke: "currentColor",
    "stroke-width": Icons.STROKE_WIDTH,
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
  };
  // Every class this module may set, so a theme without icons clears them all.
  const ROLES = ["sn-lower-third", "sn-subtitles", "sn-on-air", "sn-live", "sn-clock", "sn-active", "sn-vote"]
    .concat(["sn-align-left", "sn-align-right", "sn-tiled", "sn-tall"]);

  /**
   * What each card slot shows, by element id, from the first deck that uses it: the deck, its raised card, whether
   * its cover hides the card, the shared room's summary of its round, and whether its card went up last of all.
   */
  function slots(snapshot) {
    const found = new Map();
    let newest = null;
    for (const deck of snapshot && Array.isArray(snapshot.decks) ? snapshot.decks : []) {
      if (!deck || typeof deck.element !== "string" || found.has(deck.element)) {
        continue;
      }
      const slot = {
        deck: String(deck.id || ""),
        card: typeof deck.raised === "string" ? deck.raised : null,
        covered: deck.covered === true,
        summary: typeof deck.summary === "string" ? deck.summary : "",
        at: Number.isFinite(deck.raised_at_ms) ? deck.raised_at_ms : null,
        active: false,
      };
      found.set(deck.element, slot);
      if (slot.card && slot.at !== null && (!newest || slot.at > newest.at)) {
        newest = slot;
      }
    }
    if (newest) {
      newest.active = true;
    }
    return found;
  }

  /** What an element's tile shows, `icon` or `emoji`, and the words beside it; null when it shows no tile. */
  function tileFor(element, slot, text) {
    if (element.kind !== "text" || element.subtitles || element.on_air || element.clock) {
      return null;
    }
    if (slot && slot.covered) {
      return { icon: "check", emoji: null, text: String(text) };
    }
    if (slot && !slot.card) {
      return null;
    }
    const face = slot ? Icons.cardIcon(slot.deck, slot.card, text) : Icons.elementIcon(element.id, text);
    return face.icon || face.emoji ? face : null;
  }

  function iconSvg(doc, name) {
    const svg = doc.createElementNS(SVG, "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    for (const [key, value] of Object.entries(STROKE)) {
      svg.setAttribute(key, String(value));
    }
    for (const [tag, attributes] of Icons.ICONS[name] || []) {
      const node = doc.createElementNS(SVG, tag);
      for (const [key, value] of Object.entries(attributes)) {
        node.setAttribute(key, String(value));
      }
      svg.appendChild(node);
    }
    return svg;
  }

  /** Shows `tile` in the element's tile node, made on first use and hidden from screen readers; null hides it. */
  function showTile(state, tile) {
    const key = tile ? `${tile.icon || ""}|${tile.emoji || ""}` : "";
    if (state.tileKey === key) {
      return;
    }
    state.tileKey = key;
    if (!tile) {
      if (state.tile) {
        state.tile.remove();
        state.tile = null;
      }
      return;
    }
    const doc = state.body.ownerDocument;
    if (!state.tile) {
      state.tile = doc.createElement("span");
      state.tile.classList.add("sn-deco", "sn-icon");
      state.tile.setAttribute("aria-hidden", "true");
      state.body.appendChild(state.tile);
    }
    state.tile.classList.toggle("sn-icon-emoji", Boolean(tile.emoji));
    state.tile.textContent = tile.emoji || "";
    if (tile.icon) {
      state.tile.appendChild(iconSvg(doc, tile.icon));
    }
  }

  /** The words an element shows: without the emoji its tile shows, when its theme shows icons. */
  function words(element, text, look) {
    if (!look.icons) {
      return String(text);
    }
    const tile = tileFor(element, look.slots.get(element.id), text);
    return tile ? tile.text : String(text);
  }

  /**
   * Gives an element the tile and role classes of a theme with icons, and the room's summary as a note under a vote
   * card's words; under any other theme it has none of them.
   */
  function decorate(state, element, text, look) {
    const slot = look.icons ? look.slots.get(element.id) : undefined;
    // A lowered card keeps its tile while it fades out, so its panel does not change shape on the way out.
    const lowered = Boolean(slot && !slot.card && state.tile);
    const tile = look.icons ? tileFor(element, slot, text) || (lowered ? state.lastTile : null) : null;
    state.lastTile = tile;
    const props = element.props || {};
    const align = props.align || "center";
    const roles = {
      "sn-lower-third": look.icons && element.kind === "text" && !slot && !element.subtitles && !element.on_air
        && !element.clock,
      "sn-subtitles": look.icons && element.subtitles === true,
      "sn-on-air": look.icons && element.on_air === true,
      "sn-live": look.icons && element.on_air === true && look.live,
      "sn-clock": look.icons && element.clock === true,
      "sn-active": Boolean(slot && slot.active && !slot.covered),
      "sn-vote": Boolean(slot && slot.card && slot.covered),
      "sn-align-left": look.icons && align === "left",
      "sn-align-right": look.icons && align === "right",
      "sn-tiled": Boolean(tile),
      "sn-tall": Boolean(tile) && props.height > props.width,
    };
    for (const role of ROLES) {
      state.node.classList.toggle(role, roles[role]);
    }
    const note = roles["sn-vote"] ? slot.summary : "";
    if (state.text.getAttribute("data-note") !== note) {
      state.text.setAttribute("data-note", note);
    }
    showTile(state, tile);
  }

  /**
   * What a render of `snapshot` with the theme `definition` needs for icons: whether the theme shows them, each card
   * slot, and whether OBS is on air.
   */
  function iconLook(snapshot, definition) {
    const onAir = snapshot && snapshot.on_air;
    return {
      icons: Boolean(definition && definition.icons === true),
      slots: slots(snapshot),
      live: Boolean(onAir && Number.isFinite(onAir.since_ms)),
    };
  }

  const api = { decorate, iconLook, iconSvg, slots, tileFor, words };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronIconTiles = api;
  }
})();
