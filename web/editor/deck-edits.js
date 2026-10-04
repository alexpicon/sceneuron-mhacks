// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: edits to the draft's card decks: adding a ready-made pack, the live description bar, which has no
   deck, among them, deleting a deck, and changing a deck's settings and its cards' name, text, colors, trigger
   phrases, speech line, and hand signal. Every other field of a deck or card is kept as it is. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Render = inNode ? require("../render-style.js") : window.SceneuronRender;
  const { MAX_DECKS, MAX_ELEMENTS } = inNode ? require("./constants.js") : window.SceneuronEditor.constants;
  const { clone, deleteElement, findElement } = inNode ? require("./project.js") : window.SceneuronEditor.project;

  /** Card properties the Cards tab edits; a card never sets `visible`, because raising it does. */
  const CARD_PROPS = ["text", "color", "background"];
  const AUTO_HIDE_RANGE = [1000, 3600000];
  /** The pack of the live description bar, which has no deck. */
  const LIVE_DESCRIPTION = "live-description";

  function decksOf(project) {
    return (project && project.decks) || [];
  }

  function findDeck(project, id) {
    return decksOf(project).find((deck) => deck.id === id) || null;
  }

  function findCard(deck, id) {
    return (deck && deck.cards.find((card) => card.id === id)) || null;
  }

  /** The deck that the element with this id shows, or null. */
  function deckShownBy(project, element) {
    return decksOf(project).find((deck) => deck.element === element) || null;
  }

  /**
   * Adds a pack's card slot elements and decks to the draft, and returns the decks it added. Like the
   * server, it refuses a pack whose deck or element ids the project already uses, rather than renaming
   * them, so adding a pack twice never leaves two copies of it.
   */
  function addPack(project, pack) {
    const barsAdded = !pack.decks.length && pack.elements.every((element) => findElement(project, element.id));
    if (barsAdded || pack.decks.some((deck) => findDeck(project, deck.id))) {
      throw new Error(`${pack.name} is already in this project.`);
    }
    for (const element of pack.elements) {
      const taken = findElement(project, element.id);
      if (taken) {
        throw new Error(
          `${pack.name} needs the element id ${element.id}, which ${taken.name} already uses. Delete ${taken.name} ` +
            "first to add this pack.",
        );
      }
    }
    if (project.elements.length + pack.elements.length > MAX_ELEMENTS) {
      throw new Error(`${pack.name} does not fit: a project can have at most ${MAX_ELEMENTS} elements.`);
    }
    if (decksOf(project).length + pack.decks.length > MAX_DECKS) {
      throw new Error(`${pack.name} does not fit: a project can have at most ${MAX_DECKS} decks.`);
    }
    const decks = clone(pack.decks);
    project.elements.push(...clone(pack.elements));
    project.decks = decksOf(project).concat(decks);
    return decks;
  }

  /**
   * Adds the live description bar, from `packs` as the server offers them fitted to the saved canvas, and returns
   * `{ element, added }`: the bar's element, and false for `added` when the draft already had it.
   */
  function addLiveDescription(project, packs) {
    const pack = (Array.isArray(packs) ? packs : []).find((candidate) => candidate.id === LIVE_DESCRIPTION);
    if (!pack || pack.elements.length !== 1) {
      throw new Error("This Sceneuron offers no live description.");
    }
    const id = pack.elements[0].id;
    const present = findElement(project, id);
    if (present) {
      return { element: present, added: false };
    }
    addPack(project, pack);
    return { element: findElement(project, id), added: true };
  }

  /**
   * Removes a deck and its cards; with `withElement`, also its card slot element and the rule actions
   * that use it. Returns the removed deck, or null when there is none.
   */
  function deleteDeck(project, id, withElement) {
    const deck = findDeck(project, id);
    if (!deck) {
      return null;
    }
    project.decks = project.decks.filter((other) => other !== deck);
    if (withElement) {
      deleteElement(project, deck.element);
    }
    return deck;
  }

  /** Sets how long a raised card stays up: 0 until it is lowered, else 1 second to 1 hour. */
  function setAutoHide(deck, ms) {
    const [min, max] = AUTO_HIDE_RANGE;
    if (!Number.isInteger(ms) || (ms !== 0 && (ms < min || ms > max))) {
      throw new Error(`A card stays up until it is lowered (0) or for ${min} to ${max} ms.`);
    }
    deck.auto_hide_ms = ms;
  }

  /**
   * Sets a card's name, its hand signal, which null removes, its trigger phrases (a list) or speech line, which an
   * empty value removes, or one of the properties it shows: text, color (of the text), or background.
   */
  function setCardField(card, field, value) {
    if (field === "name") {
      card.name = value;
      return;
    }
    if (field === "signal") {
      if (value === null) {
        delete card.signal;
      } else {
        card.signal = value;
      }
      return;
    }
    if (field === "phrases" || field === "speech") {
      if (value.length === 0) {
        delete card[field];
      } else {
        card[field] = value;
      }
      return;
    }
    if (!CARD_PROPS.includes(field)) {
      throw new Error(`The Cards tab cannot set a card's ${field}.`);
    }
    card.props = card.props || {};
    card.props[field] = value;
  }

  /** What a card shows for a property: its own value, else its card slot's, else the default. */
  function cardValue(card, slot, field) {
    if (card.props && card.props[field] !== undefined) {
      return card.props[field];
    }
    if (slot && slot.props[field] !== undefined) {
      return slot.props[field];
    }
    return Render.DEFAULT_PROPS[field];
  }

  /** True when the saved project has this deck's card, so the server can raise it. */
  function isSavedCard(saved, deckId, cardId) {
    return Boolean(findCard(findDeck(saved, deckId), cardId));
  }

  const deckEdits = {
    addLiveDescription,
    addPack,
    cardValue,
    deckShownBy,
    deleteDeck,
    findCard,
    findDeck,
    isSavedCard,
    setAutoHide,
    setCardField,
  };
  if (inNode) {
    module.exports = deckEdits;
  } else {
    window.SceneuronEditor.deckEdits = deckEdits;
  }
})();
