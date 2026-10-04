// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the words of the Cards tab: how long cards stay up, hand signals, which card a deck
   shows, pack sizes, and the Stream Deck pairing code and its countdown. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /** The auto-hide delays the Cards tab offers, in milliseconds; 0 keeps a card up until it is lowered. */
  const AUTO_HIDE_CHOICES = [0, 5000, 10000, 15000, 30000, 60000, 120000, 300000, 600000, 1800000, 3600000];
  const SIGNS = {
    thumbs_up: "thumbs up",
    thumbs_down: "thumbs down",
    open_palm: "an open palm",
    peace: "a peace sign",
    pointing: "pointing",
    fist: "a fist",
  };

  function plural(count, word) {
    return `${count} ${word}${count === 1 ? "" : "s"}`;
  }

  /** "1 hour", "2 minutes", "15 seconds", or "1.5 seconds". */
  function durationLabel(ms) {
    if (ms > 0 && ms % 3600000 === 0) {
      return plural(ms / 3600000, "hour");
    }
    if (ms > 0 && ms % 60000 === 0) {
      return plural(ms / 60000, "minute");
    }
    return plural(ms / 1000, "second");
  }

  /** `[ms, label]` choices for how long a raised card stays up, including `current` when it is not offered. */
  function autoHideChoices(current) {
    const values = AUTO_HIDE_CHOICES.includes(current) || !Number.isFinite(current)
      ? AUTO_HIDE_CHOICES
      : AUTO_HIDE_CHOICES.concat(current).sort((a, b) => a - b);
    return values.map((ms) => [ms, ms === 0 ? "Until it is lowered" : durationLabel(ms)]);
  }

  /** What the camera must see to raise a card, such as "3 fingers" or "thumbs up", or null for no signal. */
  function signalLabel(signal) {
    if (signal === "hand_raised") {
      return "a raised hand";
    }
    if (signal && Number.isInteger(signal.fingers)) {
      return `${plural(signal.fingers, "finger")} up`;
    }
    if (signal && typeof signal.sign === "string") {
      return SIGNS[signal.sign] || signal.sign.replace(/_/g, " ");
    }
    return null;
  }

  /**
   * Which card a deck shows, from its live view, or what keeps it from showing one. It names the deck, since
   * the status of every deck is announced, including changes nobody made in the editor, such as an auto-hide. A
   * card covered for a shared room's vote is named here, in the editor, but the overlay shows only the cover.
   */
  function raisedStatus(deck, view) {
    if (!view) {
      return `${deck.name} is not saved yet. Save to raise these cards live.`;
    }
    if (!view.raised) {
      return `${deck.name}: no card is up.`;
    }
    const card = deck.cards.find((candidate) => candidate.id === view.raised);
    const name = card ? card.name : view.raised;
    const shows = `${deck.name} shows ${name}`;
    if (view.covered) {
      return `${shows}, covered as a hidden vote.`;
    }
    const by = { camera: ", raised by the camera", voice: ", raised by voice" }[view.raised_by] || "";
    return `${shows}${by}.`;
  }

  /** How many cards a pack brings, such as "10 cards", or for a pack without cards, its elements. */
  function packSize(pack) {
    if (!pack.decks.length) {
      return plural(pack.elements.length, "element");
    }
    return plural(pack.decks.reduce((sum, deck) => sum + deck.cards.length, 0), "card");
  }

  /** A six-digit pairing code in two groups, such as "123 456", so it is easy to read out and type. */
  function groupCode(code) {
    const text = String(code);
    return /^\d{6}$/.test(text) ? `${text.slice(0, 3)} ${text.slice(3)}` : text;
  }

  /** Time left as "1:58", never below "0:00". */
  function countdown(ms) {
    const seconds = Math.max(0, Math.ceil(ms / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  }

  const cardsText = {
    autoHideChoices,
    countdown,
    durationLabel,
    groupCode,
    packSize,
    raisedStatus,
    signalLabel,
  };
  if (inNode) {
    module.exports = cardsText;
  } else {
    window.SceneuronEditor.cardsText = cardsText;
  }
})();
