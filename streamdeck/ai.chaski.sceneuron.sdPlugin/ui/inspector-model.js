// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron property inspector: which fields each action has, the choices in each list, and the
   settings a choice produces. No DOM here, so the Node tests can load it.

   The lists come from the plugin's status message. A saved choice the plugin does not know stays
   in its list, marked "not found" when Sceneuron is connected, so opening the inspector offline
   never rewrites a key's settings. */
(function () {
  "use strict";

  const KINDS = {
    "ai.chaski.sceneuron.card": "card",
    "ai.chaski.sceneuron.lower": "lower",
    "ai.chaski.sceneuron.element": "element",
    "ai.chaski.sceneuron.automation": "automation",
    "ai.chaski.sceneuron.dial": "dial",
    "ai.chaski.sceneuron.talk": "talk",
    "ai.chaski.sceneuron.reveal": "reveal",
    "ai.chaski.sceneuron.next-round": "next-round",
    "ai.chaski.sceneuron.reaction": "reaction",
  };

  const FIELDS = {
    card: ["deck", "card"],
    lower: ["deck"],
    element: ["element"],
    automation: [],
    dial: ["deck"],
    talk: [],
    reveal: [],
    "next-round": [],
    reaction: ["reaction"],
  };

  /** What the inspector shows until the plugin answers. */
  const WAITING = {
    type: "status",
    state: "connecting",
    port: 4466,
    message: "Waiting for the plugin...",
    decks: [],
    elements: [],
    reactions: [],
  };

  /** The action kind for a manifest UUID, or null for one this inspector does not know. */
  function kindOf(actionUuid) {
    return Object.prototype.hasOwnProperty.call(KINDS, actionUuid) ? KINDS[actionUuid] : null;
  }

  /** The settings fields an action kind has, in display order. */
  function fieldsFor(kind) {
    return FIELDS[kind] || [];
  }

  function options(entries, selected, blank, known) {
    const list = [{ value: "", label: blank }];
    for (const entry of entries) {
      list.push({ value: entry.id, label: entry.name });
    }
    if (selected && !entries.some((entry) => entry.id === selected)) {
      list.push({ value: selected, label: known ? `${selected} (not found)` : selected });
    }
    return list.map((option) => ({ ...option, selected: option.value === (selected || "") }));
  }

  function connected(status) {
    return status.state === "connected";
  }

  /** Deck choices; for a lower key the empty choice means every deck. */
  function deckOptions(status, kind, settings) {
    const blank = kind === "lower" ? "All decks" : "Choose a deck";
    return options(status.decks, settings.deck, blank, connected(status));
  }

  /** Card choices within the chosen deck. */
  function cardOptions(status, settings) {
    const deck = status.decks.find((entry) => entry.id === settings.deck);
    return options(deck ? deck.cards : [], settings.card, "Choose a card", connected(status) && Boolean(deck));
  }

  function elementOptions(status, settings) {
    return options(status.elements, settings.element, "Choose an element", connected(status));
  }

  /** Reaction choices: every preset, which the plugin knows whether or not Sceneuron is connected. */
  function reactionOptions(status, settings) {
    const reactions = Array.isArray(status.reactions) ? status.reactions : [];
    return options(reactions, settings.reaction, "Choose a reaction", reactions.length > 0);
  }

  /** Settings after the user picks `value` for `field`; a card key that changes deck takes its first card. */
  function updatedSettings(kind, status, settings, field, value) {
    const next = { ...settings };
    if (value) {
      next[field] = value;
    } else {
      delete next[field];
    }
    if (kind === "card" && field === "deck") {
      const deck = status.decks.find((entry) => entry.id === value);
      if (deck && deck.cards.length > 0) {
        next.card = deck.cards[0].id;
      } else {
        delete next.card;
      }
    }
    return next;
  }

  /** A status message from the plugin, or null when `payload` is something else. */
  function readStatus(payload) {
    if (!payload || payload.type !== "status" || !Array.isArray(payload.decks) || !Array.isArray(payload.elements)) {
      return null;
    }
    return payload;
  }

  const model = {
    WAITING,
    kindOf,
    fieldsFor,
    deckOptions,
    cardOptions,
    elementOptions,
    reactionOptions,
    updatedSettings,
    readStatus,
  };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = model;
  }
  if (typeof window !== "undefined") {
    window.SceneuronInspectorModel = model;
  }
})();
