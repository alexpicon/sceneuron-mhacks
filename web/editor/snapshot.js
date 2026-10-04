// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: reading live snapshots, composing the preview from the draft and the live state, the
   elements a plan preview marks, and the labels and times the status bar and event log show. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { ON_AIR_PLACEHOLDER, ON_AIR_TIME_PLACEHOLDER, PRESENCE_STATES } = inNode
    ? require("./constants.js")
    : window.SceneuronEditor.constants;
  const { previewValues, renderPlaceholders, shows, showsSubtitles } = inNode
    ? require("./placeholders.js")
    : window.SceneuronEditor.placeholders;

  /** Where an element sits; the preview keeps the draft's, so a covered card slot can still be moved. */
  const GEOMETRY = ["x", "y", "width", "height"];

  /**
   * The draft's card that each element shows while the live snapshot has that card's deck raised, or
   * `{ covered: true }` while the deck shows its cover in place of the card, as for a shared room's
   * hidden vote: the preview then shows what the overlay does, never the covered card.
   */
  function raisedCards(draft, live) {
    const shown = new Map();
    for (const view of live && Array.isArray(live.decks) ? live.decks : []) {
      const deck = (draft.decks || []).find((candidate) => candidate.id === view.id);
      const card = deck && view.raised ? deck.cards.find((candidate) => candidate.id === view.raised) : null;
      if (card) {
        shown.set(deck.element, view.covered ? { covered: true } : card);
      }
    }
    return shown;
  }

  /**
   * What the preview draws, layered like the overlay: draft (unsaved) base properties, the
   * fields rules set taken from the live snapshot, the draft's version of the card a live deck
   * raised (which also shows the element) or the live cover over it, and the fields manual holds
   * set, again from the live
   * snapshot. Placeholders in draft text show the live values, or a bracketed stand-in before
   * the AI reported any; live text arrives with its placeholders already filled in. Text marks
   * whether it shows live subtitles, OBS's scene, or the time on air, so a theme draws it as the
   * overlay does. The draft's theme and decks are drawn too, so a theme shows before it is saved,
   * with what the live decks raised, and the live clock and program, which the time on air counts
   * from.
   */
  function composePreview(draft, live) {
    const subtitles = live ? live.subtitles : null;
    const clock = live ? { now_ms: live.now_ms, on_air: live.on_air } : {};
    if (!draft) {
      return live
        ? { canvas: live.canvas, theme: live.theme, decks: live.decks, subtitles, elements: live.elements, ...clock }
        : { canvas: null, elements: [] };
    }
    const liveById = new Map((live && Array.isArray(live.elements) ? live.elements : []).map((e) => [e.id, e]));
    const valueOf = previewValues(draft, live);
    const cards = raisedCards(draft, live);
    const elements = draft.elements.map((element) => {
      const current = liveById.get(element.id);
      const props = Object.assign({}, element.props);
      let liveText = false;
      const takeLive = (fields) => {
        for (const field of fields) {
          if (current.props && field in current.props) {
            props[field] = current.props[field];
            liveText = liveText || field === "text";
          }
        }
      };
      takeLive(current ? current.automated || [] : []);
      const card = cards.get(element.id);
      if (card && card.covered) {
        // The cover's fields are not in the snapshot, so everything but the geometry comes from the overlay.
        for (const [field, value] of Object.entries((current && current.props) || {})) {
          if (!GEOMETRY.includes(field)) {
            props[field] = value;
          }
        }
        props.visible = true;
        liveText = true;
      } else if (card) {
        Object.assign(props, card.props, { visible: true });
        liveText = liveText && !(card.props && "text" in card.props);
      }
      takeLive(current ? current.overridden || [] : []);
      const flag = (name, own) => (liveText ? Boolean(current && current[name]) : own(String(props.text)));
      const subtitled = flag("subtitles", showsSubtitles);
      const onAir = flag("on_air", (text) => shows(text, ON_AIR_PLACEHOLDER));
      const counted = flag("clock", (text) => shows(text, ON_AIR_TIME_PLACEHOLDER));
      if (typeof props.text === "string" && !liveText) {
        props.text = renderPlaceholders(props.text, valueOf);
      }
      return {
        id: element.id,
        name: element.name,
        kind: element.kind,
        props,
        subtitles: subtitled,
        on_air: onAir,
        clock: counted,
        effect: current ? current.effect : null,
        overridden: current ? current.overridden || [] : [],
        automated: current ? current.automated || [] : [],
      };
    });
    const liveDecks = new Map((live && Array.isArray(live.decks) ? live.decks : []).map((deck) => [deck.id, deck]));
    const decks = (draft.decks || []).map((deck) => {
      const view = liveDecks.get(deck.id);
      const raised = view && deck.cards.some((card) => card.id === view.raised) ? view.raised : null;
      const shown = raised
        ? { raised, raised_at_ms: view.raised_at_ms, covered: view.covered, summary: view.summary }
        : {};
      return { id: deck.id, name: deck.name, element: deck.element, ...shown };
    });
    return { canvas: draft.canvas, theme: draft.theme, decks, subtitles, elements, ...clock };
  }

  /** Elements a plan adds, changes, makes rules act on, or shows a deck's cards in, for marking them in the preview. */
  function plannedIds(plan) {
    const ids = new Set();
    for (const op of plan && Array.isArray(plan.operations) ? plan.operations : []) {
      if (op.op === "add_element" && op.element) {
        ids.add(op.element.id);
      } else if (op.op === "update_element") {
        ids.add(op.id);
      } else if ((op.op === "add_rule" || op.op === "update_rule") && op.rule) {
        for (const action of op.rule.actions || []) {
          ids.add(action.element);
        }
      } else if ((op.op === "add_deck" || op.op === "update_deck") && op.deck) {
        ids.add(op.deck.element);
      }
    }
    return ids;
  }

  function isSnapshot(value) {
    return (
      Boolean(value) && typeof value === "object" && Array.isArray(value.elements) && Number.isFinite(value.revision)
    );
  }

  /** Drops snapshots older than the one shown, such as a stream event that lost a race with a response. */
  function acceptSnapshot(current, next) {
    return isSnapshot(next) && (!isSnapshot(current) || next.revision >= current.revision);
  }

  function presenceLabel(presence) {
    const state = presence && PRESENCE_STATES.includes(presence.state) ? presence.state : "unknown";
    const name = state.charAt(0).toUpperCase() + state.slice(1);
    const score = presence && Number.isFinite(presence.last_score) ? `, score ${presence.last_score.toFixed(2)}` : "";
    return name + score;
  }

  function visionLabel(vision) {
    const state = vision && vision.state;
    if (state === "running") {
      const latency = Number.isFinite(vision.latency_ms) ? `, ${Math.round(vision.latency_ms)} ms` : "";
      return `Camera running${latency}`;
    }
    if (state === "starting") {
      return "Camera starting";
    }
    if (state === "failed") {
      const message = vision.message ? String(vision.message) : "";
      const short = message.length > 80 ? `${message.slice(0, 77)}...` : message;
      return short ? `Camera failed: ${short}` : "Camera failed";
    }
    return "Camera idle";
  }

  /** Converts an event's server time to wall-clock time using the snapshot's own clock. */
  function eventWallTime(atMs, snapshotNowMs, receivedWallMs) {
    return receivedWallMs - Math.max(0, snapshotNowMs - atMs);
  }

  function formatClock(wallMs) {
    const date = new Date(wallMs);
    const pad = (value) => String(value).padStart(2, "0");
    return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  }

  const snapshot = {
    acceptSnapshot,
    composePreview,
    eventWallTime,
    formatClock,
    isSnapshot,
    plannedIds,
    presenceLabel,
    visionLabel,
  };
  if (inNode) {
    module.exports = snapshot;
  } else {
    window.SceneuronEditor.snapshot = snapshot;
  }
})();
