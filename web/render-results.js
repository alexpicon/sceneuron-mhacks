// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay results of a shared room's reveal: each vote by name and the average, read from the room's
   summary, such as "Maya 3 · You 5 · Average 4". Graphite draws them in its own panel beside its playing card
   (render-cards.js); under every other theme a card slot whose votes are revealed gets a panel beside it that wears
   the theme's own look, as its cards do, on the side toward the frame's center. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const DEFAULT_WIDTH = 1920;

  /** Each revealed vote and the average from a summary after a reveal; null before the reveal or without votes. */
  function results(summary) {
    const said = String(summary || "").trim();
    if (!said || /^Voted:/.test(said) || said === "No votes") {
      return null;
    }
    let average = null;
    const rows = [];
    for (const part of said.split(" · ")) {
      const avg = /^Average\s+(.+)$/.exec(part);
      if (avg) {
        average = avg[1];
        continue;
      }
      const cut = part.lastIndexOf(" ");
      if (cut > 0) {
        rows.push({ who: part.slice(0, cut), vote: part.slice(cut + 1) });
      }
    }
    return rows.length ? { rows, average } : null;
  }

  /** The side of an element toward the frame's center: "left" for one in the right half, else "right". */
  function towardCenter(props, canvas) {
    const width = canvas && Number.isFinite(canvas.width) && canvas.width > 0 ? canvas.width : DEFAULT_WIDTH;
    const x = Number(props && props.x) || 0;
    const w = Number(props && props.width) || 0;
    return x + w / 2 <= width / 2 ? "right" : "left";
  }

  /** The lines a themed panel shows: "Maya  3" for each vote, then "Average  4". */
  function lines(shown) {
    const rows = shown.rows.map((row) => `${row.who}  ${row.vote}`);
    return shown.average === null ? rows : rows.concat(`Average  ${shown.average}`);
  }

  /**
   * Shows the revealed votes of the element's card slot beside it, in a panel with the theme's card look, under any
   * theme but `skip` (Graphite, which draws its own); hides the panel otherwise.
   */
  function decorate(state, element, slot, look, skip) {
    const shown = look.theme !== skip && slot && slot.card && !slot.covered ? results(slot.summary) : null;
    const side = shown ? towardCenter(element.props, look.canvas) : "";
    const key = shown ? JSON.stringify([lines(shown), side]) : "";
    if (state.resultsKey === key) {
      return;
    }
    state.resultsKey = key;
    if (!shown) {
      if (state.results) {
        state.results.remove();
        state.results = null;
      }
      return;
    }
    if (!state.results) {
      // A second body inside the element, so each theme's rules for a card's body dress it too.
      state.results = state.node.ownerDocument.createElement("div");
      state.results.classList.add("sn-body", "sn-room-results");
      state.node.appendChild(state.results);
    }
    const box = state.results;
    box.textContent = lines(shown).join("\n");
    const styles = {
      position: "absolute",
      bottom: "0",
      top: "auto",
      width: "auto",
      height: "auto",
      "min-width": "calc(260 * var(--sn-u))",
      display: "block",
      padding: "calc(16 * var(--sn-u)) calc(24 * var(--sn-u))",
      "font-size": "calc(30 * var(--sn-u))",
      "line-height": "1.35",
      "text-align": "left",
      "white-space": "pre",
      left: side === "right" ? "calc(100% + 16 * var(--sn-u))" : "auto",
      right: side === "left" ? "calc(100% + 16 * var(--sn-u))" : "auto",
    };
    for (const [name, value] of Object.entries(styles)) {
      box.style.setProperty(name, value);
    }
  }

  const api = { decorate, lines, results, towardCenter };
  if (inNode) {
    module.exports = api;
  } else {
    window.SceneuronResults = api;
  }
})();
