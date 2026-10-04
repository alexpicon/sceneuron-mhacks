// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the voice parts of the Cards tab: a deck's "Voice can raise these cards" and "Speak cards
   when raised" switches, and a card's trigger phrases and speech line, with the checks the server makes. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { phrasesFrom, phrasesProblem, speechProblem } = Editor.voiceText;
  const { check, field, h } = Editor.dom;

  /** What a deck's voice switch is for, and how many of its cards have trigger phrases. */
  function voiceNote(deck) {
    const phrases = deck.cards.filter((card) => card.phrases && card.phrases.length).length;
    const have = phrases === 0
      ? "None of these cards has a trigger phrase yet."
      : `${phrases} of these cards ${phrases === 1 ? "has" : "have"} trigger phrases.`;
    return "Say a card's name or text after the wake word, or while holding push to talk. Trigger phrases work " +
      `when voice control listens all the time, and pause while automation is paused. ${have}`;
  }

  /** The deck's voice and speech switches, with their notes. */
  function deckVoiceFields(deck, di) {
    const base = `deck-${di}`;
    const toggle = (name) =>
      h("input", {
        id: `${base}-${name}`,
        type: "checkbox",
        checked: Boolean(deck[name]),
        "aria-describedby": `${base}-${name}-note`,
        "data-deck-index": di,
        "data-deck-field": name,
        "data-focus-key": `deck:${deck.id}:${name}`,
      });
    return [
      check(`${base}-voice`, "Voice can raise these cards", toggle("voice")),
      h("p", { id: `${base}-voice-note`, class: "hint" }, voiceNote(deck)),
      check(`${base}-speak`, "Speak cards when raised", toggle("speak")),
      h(
        "p",
        { id: `${base}-speak-note`, class: "hint" },
        "Needs spoken cards on in the AI tab. A card says its speech line, or else its text without emoji.",
      ),
    ];
  }

  /** The card editor's trigger phrases and speech line for `card`. */
  function cardVoiceFields(deck, di, card) {
    const base = `deck-${di}-card`;
    const data = (name, extra) =>
      Object.assign(
        {
          id: `${base}-${name}`,
          "data-deck-index": di,
          "data-card-field": name,
          "data-focus-key": `deck:${deck.id}:card-${name}`,
        },
        extra,
      );
    const phrases = h("textarea", data("phrases", { rows: 2, "aria-describedby": `${base}-phrases-hint` }));
    phrases.value = (card.phrases || []).join("\n");
    const speech = h("input", data("speech", { type: "text", maxlength: 200, autocomplete: "off" }));
    speech.value = card.speech || "";
    return [
      field(`${base}-phrases`, "Trigger phrases", phrases),
      h(
        "p",
        { id: `${base}-phrases-hint`, class: "hint" },
        "One per line, up to four, such as \"let's take this offline\". Saying one raises this card.",
      ),
      field(`${base}-speech`, "Speech line (empty to say its text)", speech),
    ];
  }

  /**
   * Reads a card's voice field: `{ value }` with the phrases as a list or the speech line, or `{ problem }` when
   * the server would refuse it. Other fields are passed through.
   */
  function readCardVoice(name, raw) {
    if (name === "phrases") {
      const phrases = phrasesFrom(raw);
      const problem = phrasesProblem(phrases);
      return problem ? { problem } : { value: phrases };
    }
    if (name === "speech") {
      const problem = speechProblem(raw);
      return problem ? { problem } : { value: raw.trim() };
    }
    return { value: raw };
  }

  Editor.deckVoice = { cardVoiceFields, deckVoiceFields, readCardVoice };
})();
