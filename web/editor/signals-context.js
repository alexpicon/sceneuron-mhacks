// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: hand signals by context, as the camera decides them. A fist, pointing, a peace sign, and an open
   palm also give a finger count, so a deck that reads one of these signs and a deck whose cards use finger counts
   compete while both listen. Then the counting deck takes its counts only while a vote is open for it, and the signing
   deck takes those signs only while none of the counting decks it competes with has a vote open; a deck that competes
   with none takes every signal. This says when the camera raises a card, for the card editor's notes, and where hand
   signals go right now, from the live snapshot. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  /** The signs whose hand the camera also counts; a thumbs up or down gives no count. */
  const COUNTED_SIGNS = ["fist", "pointing", "peace", "open_palm"];

  const isCount = (signal) => Boolean(signal) && Number.isInteger(signal.fingers);
  const isCountedSign = (signal) => Boolean(signal) && COUNTED_SIGNS.includes(signal.sign);
  const counts = (deck) => Boolean(deck.camera) && deck.cards.some((card) => isCount(card.signal));
  const signs = (deck) => Boolean(deck.camera) && deck.cards.some((card) => isCountedSign(card.signal));

  /** "A", "A and B", or "A, B, and C"; `word` comes before the last item. */
  function listed(items, word = "and") {
    return items.length < 3 ? items.join(` ${word} `) : `${items.slice(0, -1).join(", ")}, ${word} ${items.at(-1)}`;
  }

  /**
   * When the camera raises a card, once card `cardId` of deck `deckId` has `signal` and that deck listens:
   * `gate(deckId, signal)` gives `{ when: "always" }`, `{ when: "vote", decks }` for only while a vote is open in the
   * one deck named, or `{ when: "no-vote", decks }` for only while none of the decks named has one open. Deck
   * `deckId` is named "this deck".
   */
  function gates(project, deckId, cardId, signal) {
    const decks = ((project && project.decks) || []).map((deck) => {
      if (deck.id !== deckId) {
        return deck;
      }
      const cards = deck.cards.map((card) => (card.id === cardId ? { ...card, signal } : card));
      return { ...deck, camera: true, cards };
    });
    const named = (deck) => (deck.id === deckId ? "this deck" : deck.name);
    return (id, cardSignal) => {
      const deck = decks.find((candidate) => candidate.id === id);
      const rivals = decks.filter((other) => other !== deck);
      if (deck && isCount(cardSignal) && counts(deck) && rivals.some(signs)) {
        return { when: "vote", decks: [named(deck)] };
      }
      if (deck && isCountedSign(cardSignal) && signs(deck) && rivals.some(counts)) {
        return { when: "no-vote", decks: rivals.filter(counts).map(named) };
      }
      return { when: "always", decks: [] };
    };
  }

  /** What tells two gates apart. */
  const key = (gate) => `${gate.when}:${gate.decks.join("\n")}`;

  /** `[gate, cards]` for each gate of `cards`, as `gateOf(card)` gives them, in the cards' order. */
  function byGate(cards, gateOf) {
    const groups = new Map();
    for (const card of cards) {
      const gate = gateOf(card);
      const id = key(gate);
      if (groups.has(id)) {
        groups.get(id)[1].push(card);
      } else {
        groups.set(id, [gate, [card]]);
      }
    }
    return Array.from(groups.values());
  }

  /** Who is raised among this card and `them`, as `[this card's, theirs]` say. */
  function who([mine, theirs], them) {
    const group = them === "one of them";
    if (mine && theirs) {
      return group ? "this card and one of them are raised" : "both are raised";
    }
    if (mine || theirs) {
      return `only ${mine ? "this card" : them} is raised`;
    }
    return group ? "none of these cards is raised" : "neither is raised";
  }

  /** When the camera raises a card gated by `gate`, as the end of a sentence. */
  function when(gate) {
    if (gate.when === "vote") {
      return `only while a vote is open in ${gate.decks[0]}`;
    }
    return gate.when === "no-vote" ? `only while no vote is open in ${listed(gate.decks, "or")}` : "either way";
  }

  /**
   * How the context decides whether a pose raises this card, gated by `mine`, and `them`, a card or "one of them",
   * gated by `theirs`, as the end of a sentence; null when the context decides nothing and both are raised.
   */
  function decides(mine, theirs, them) {
    if (mine.when === "always" && theirs.when === "always") {
      return null;
    }
    const decks = new Set(mine.decks.concat(theirs.decks));
    if (decks.size === 1) {
      const vote = `a vote in ${[...decks][0]} decides`;
      if ([mine.when, theirs.when].sort().join() === "no-vote,vote") {
        const [open, none] = mine.when === "vote" ? ["this card", them] : [them, "this card"];
        return `${vote}: ${open} is raised while one is open, and ${none} while none is`;
      }
      const raised = (open) => [mine, theirs].map((gate) => gate.when === "always" || (gate.when === "vote") === open);
      return `${vote}: ${who(raised(true), them)} while one is open, and ${who(raised(false), them)} while none is`;
    }
    return `the context decides: this card is raised ${when(mine)}, and ${them} ${when(theirs)}`;
  }

  /**
   * Splits `cards`, other cards of one deck whose signals one pose shows with this card's, by what decides which card
   * the deck raises: `[cards, but]` pairs, in the cards' order. The deck raises the first card the camera matches, so
   * the context matters only where it can keep that card from being matched; `but(them)` then ends the sentence that
   * names the first card with the card raised instead, `them` being a card or "one of them", and gives "" elsewhere.
   * `mine` gates this card, `gateOf(card)` the others, and `ahead(card)` says whether a card comes before this one.
   */
  function byOrder(cards, mine, gateOf, ahead) {
    // The order alone decides where both cards wait for the same, or where the card that comes first never waits.
    const ordered = { when: "order", decks: [] };
    const gate = (card) => {
      const theirs = gateOf(card);
      return (key(theirs) === key(mine) || (ahead(card) ? theirs : mine).when === "always") ? ordered : theirs;
    };
    return byGate(cards, gate).map(([theirs, group]) => [
      group,
      (them) => (theirs === ordered ? "" : but(mine, theirs, them)),
    ]);
  }

  /** The words for `byOrder` on a card gated by `mine` and `them`, gated by `theirs`, where the context matters. */
  function but(mine, theirs, them) {
    if (mine.when !== "always" && theirs.when !== "always") {
      return `, while both can be: this card is raised ${when(mine)}, and ${them} ${when(theirs)}`;
    }
    const [waits, only] = mine.when === "always" ? [theirs, "this card"] : [mine, them];
    const shut = waits.when === "vote"
      ? `no vote is open in ${waits.decks[0]}`
      : `a vote is open in ${listed(waits.decks, "or")}`;
    return `, but only ${only} while ${shut}`;
  }

  /**
   * Where hand signals go right now, from the live snapshot, as one line; "" while no decks compete for them.
   * `clock(ms)` shows a time on the snapshot's clock as the streamer's clock does.
   */
  function routeLine(live, clock) {
    const view = live && live.hand_signals;
    if (!view) {
      return "";
    }
    const decks = new Map(((live && live.decks) || []).map((deck) => [deck.id, deck]));
    const name = (id) => (decks.get(id) || { name: id }).name;
    const voting = view.to.filter((id) => view.count_decks.includes(id));
    if (voting.length === 0) {
      return `Hand signals go to ${listed(view.to.map(name))}: no vote is open in ` +
        `${listed(view.count_decks.map(name), "or")}.`;
    }
    const vote = (decks.get(voting[0]) || {}).vote;
    let why = "a vote is open there";
    if (voting.length > 1) {
      why = "a vote is open in each";
    } else if (vote && vote.by === "room") {
      why = "its shared room vote is open until the reveal";
    } else if (vote && Number.isFinite(vote.until_ms)) {
      why = `raising its card opened a vote until ${clock(vote.until_ms)}, or until the card is lowered`;
    }
    return `Finger counts go to ${listed(voting.map(name))}: ${why}.`;
  }

  /**
   * A `clock` for routeLine that converts each time once, with `convert(ms)`. The line is a live region, and a time
   * converted anew for each snapshot, whose receive time jitters, could flip between two seconds and announce the line
   * again.
   */
  function steadyClock(convert) {
    let last = { ms: null, text: "" };
    return (ms) => {
      if (last.ms !== ms) {
        last = { ms, text: convert(ms) };
      }
      return last.text;
    };
  }

  const signalsContext = { byGate, byOrder, decides, gates, listed, routeLine, steadyClock };
  if (inNode) {
    module.exports = signalsContext;
  } else {
    window.SceneuronEditor.signalsContext = signalsContext;
  }
})();
