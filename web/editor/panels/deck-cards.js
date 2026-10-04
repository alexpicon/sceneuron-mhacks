// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of one deck in the Cards tab: which card is up, a button per card that raises or
   lowers it live, its settings, its card editor, and the prompt to delete it. Controls carry the deck's index and
   the field or command they stand for, and a focus key by deck id that survives rebuilds. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_NAME_CHARS, MAX_TEXT_CHARS } = Editor.constants;
  const { autoHideChoices, signalLabel } = Editor.cardsText;
  const { cardValue, findCard } = Editor.deckEdits;
  const { findElement } = Editor.project;
  const { splitColor } = Editor.values;
  const { check, field, h, hidden, option } = Editor.dom;
  const { cardVoiceFields, deckVoiceFields } = Editor.deckVoice;
  const { signalChoices, signalConflicts, signalValue } = Editor.signalsText;

  /** Colors a card's button the way the overlay shows the card. CSSOM writes pass the page's style policy. */
  function paintCard(button, card, slot) {
    button.style.setProperty("--card-color", cardValue(card, slot, "color"));
    button.style.setProperty("--card-background", cardValue(card, slot, "background"));
  }

  function cardButton(deck, di, card, slot) {
    const button = h(
      "button",
      {
        type: "button",
        class: "card-button",
        "aria-pressed": "false",
        "data-card-command": "toggle",
        "data-deck-index": di,
        "data-card-id": card.id,
        "data-focus-key": `deck:${deck.id}:card:${card.id}`,
      },
      h("span", { "data-card-name": "" }, card.name),
      // aria-pressed tells assistive technology; this tells the eye without relying on color.
      h("span", { class: "card-up", "aria-hidden": "true" }, "Up"),
    );
    paintCard(button, card, slot);
    return button;
  }

  /** A color picker plus an alpha slider for the card field `name`. */
  function colorField(deck, di, name, label, value) {
    const id = `deck-${di}-card-${name}`;
    const percent = Math.round(splitColor(value).alpha * 100);
    const part = (kind, extra) =>
      Object.assign(
        {
          "data-deck-index": di,
          "data-card-field": name,
          "data-part": kind,
          "data-focus-key": `deck:${deck.id}:card-${name}:${kind}`,
        },
        extra,
      );
    const picker = h("input", part("rgb", { id, type: "color", value: splitColor(value).rgb }));
    const alphaId = `${id}-alpha`;
    const alpha = h(
      "input",
      part("alpha", { id: alphaId, type: "range", min: 0, max: 100, step: 1, value: percent }),
    );
    alpha.setAttribute("aria-valuetext", `${percent}%`);
    const alphaLabel = h("label", { for: alphaId, class: "sub-label" }, hidden(`${label} `), "Alpha");
    const output = h("output", { id: `${id}-output`, for: alphaId }, `${percent}%`);
    return h(
      "div",
      { class: "field" },
      h("label", { for: id }, label),
      h("div", { class: "color-row" }, picker, alphaLabel, alpha, output),
    );
  }

  /**
   * The card's Hand signal list, then what its signal shares with other cards, which the list's description reads:
   * warnings first, then notes on poses that show two signals.
   */
  function signalField(draft, deck, di, card) {
    const id = `deck-${di}-card-signal`;
    const { refusal, warnings, notes } = signalConflicts(draft, deck.id, card.id, card.signal);
    const items = [refusal, ...warnings]
      .filter(Boolean)
      .map((text) => h("li", { class: "field-warning" }, text))
      .concat(notes.map((text) => h("li", { class: "hint" }, text)));
    const select = h(
      "select",
      {
        id,
        value: signalValue(card.signal),
        "aria-describedby": items.length ? `${id}-notes` : null,
        "data-deck-index": di,
        "data-card-field": "signal",
        "data-focus-key": `deck:${deck.id}:card-signal`,
      },
      ...signalChoices(card.signal).map(([value, label]) => option(value, label)),
    );
    const list = items.length ? h("ul", { id: `${id}-notes`, class: "signal-notes" }, ...items) : null;
    return [field(id, "Hand signal", select), list];
  }

  /** The editor for one card at a time: which card, then its name, text, colors, hand signal, and voice. */
  function cardEditor(draft, deck, di, slot, state) {
    const card = findCard(deck, state.editing) || deck.cards[0];
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
    const choose = h(
      "select",
      {
        id: `deck-${di}-edit-card`,
        value: card.id,
        "data-deck-index": di,
        "data-deck-field": "edit-card",
        "data-focus-key": `deck:${deck.id}:edit-card`,
      },
      ...deck.cards.map((other) => option(other.id, other.name)),
    );
    const name = h("input", data("name", { type: "text", maxlength: MAX_NAME_CHARS, autocomplete: "off" }));
    name.value = card.name;
    const text = h("textarea", data("text", { rows: 2, maxlength: MAX_TEXT_CHARS }));
    text.value = cardValue(card, slot, "text");
    return h(
      "details",
      { class: "card-editor", open: state.open, "data-deck-id": deck.id },
      h("summary", {}, "Edit cards", hidden(` of ${deck.name}`)),
      field(`deck-${di}-edit-card`, "Card", choose),
      field(`${base}-name`, "Name", name),
      field(`${base}-text`, "Text", text),
      colorField(deck, di, "color", "Text color", cardValue(card, slot, "color")),
      colorField(deck, di, "background", "Background", cardValue(card, slot, "background")),
      ...signalField(draft, deck, di, card),
      ...cardVoiceFields(deck, di, card),
    );
  }

  function confirmBox(deck, di, slot, command) {
    const base = `deck-${di}`;
    const count = deck.cards.length === 1 ? "1 card" : `${deck.cards.length} cards`;
    const alsoSlot = slot
      ? check(
        `${base}-delete-slot`,
        `Also delete its card slot, the element ${slot.name}`,
        h("input", { id: `${base}-delete-slot`, type: "checkbox", checked: true }),
      )
      : null;
    return h(
      "div",
      { class: "confirm-box", "data-confirm": "", hidden: true },
      h("p", { id: `${base}-delete-text` }, `Delete the ${deck.name} deck and its ${count}?`),
      alsoSlot,
      h(
        "div",
        { class: "button-row" },
        command("delete-confirmed", "Delete deck", { class: "danger", "aria-describedby": `${base}-delete-text` }),
        command("cancel-delete", "Keep it"),
      ),
    );
  }

  /** The note under the hand signal switch: what signals need, when they pause, and how many cards have one. */
  function cameraNote(deck) {
    const signals = deck.cards.filter((card) => signalLabel(card.signal)).length;
    const have = signals === 0
      ? "None of these cards has one yet."
      : `${signals} of these cards ${signals === 1 ? "has" : "have"} one.`;
    return "Hand signals need macOS and Apple Vision, and pause while automation is paused. The camera image " +
      `stays on this computer. ${have}`;
  }

  /**
   * The deck at index `di` of the draft. `state.editing` is the id of the card its editor shows, and
   * `state.open` whether that editor is open.
   */
  function deckCard(draft, deck, di, state) {
    const base = `deck-${di}`;
    const slot = findElement(draft, deck.element);
    const command = (name, label, extra, ...children) =>
      h(
        "button",
        Object.assign(
          {
            type: "button",
            "data-card-command": name,
            "data-deck-index": di,
            "data-focus-key": `deck:${deck.id}:${name}`,
          },
          extra,
        ),
        label,
        ...children,
      );
    const ms = deck.auto_hide_ms || 0;
    const autoHide = h(
      "select",
      {
        id: `${base}-auto-hide`,
        value: ms,
        "data-deck-index": di,
        "data-deck-field": "auto_hide_ms",
        "data-focus-key": `deck:${deck.id}:auto_hide_ms`,
      },
      ...autoHideChoices(ms).map(([value, label]) => option(value, label)),
    );
    const camera = h("input", {
      id: `${base}-camera`,
      type: "checkbox",
      checked: Boolean(deck.camera),
      "aria-describedby": `${base}-camera-note`,
      "data-deck-index": di,
      "data-deck-field": "camera",
      "data-focus-key": `deck:${deck.id}:camera`,
    });
    const buttons = h(
      "div",
      { class: "card-buttons", role: "group", "aria-label": `${deck.name} cards` },
      ...deck.cards.map((card) => cardButton(deck, di, card, slot)),
    );
    return h(
      "article",
      { class: "deck", "aria-labelledby": `${base}-heading`, "data-deck-index": di },
      h("h4", { id: `${base}-heading`, tabindex: "-1" }, deck.name),
      h("p", { class: "hint" }, slot ? `Shown by the element ${slot.name}.` : "Its card slot element is missing."),
      h("p", { class: "deck-status", role: "status", "data-live": "status" }),
      h("p", { class: "hint", "data-live": "hide-at", hidden: true }),
      buttons,
      h("div", { class: "button-row" }, command("lower", "Lower", {}, hidden(` ${deck.name}`))),
      field(`${base}-auto-hide`, "Each card stays up", autoHide),
      check(`${base}-camera`, "Hand signals can raise these cards", camera),
      h("p", { id: `${base}-camera-note`, class: "hint" }, cameraNote(deck)),
      ...deckVoiceFields(deck, di),
      cardEditor(draft, deck, di, slot, state),
      h(
        "div",
        { class: "button-row" },
        command("delete-deck", "Delete deck", { class: "danger" }, hidden(` ${deck.name}`)),
      ),
      confirmBox(deck, di, slot, command),
    );
  }

  Editor.deckCards = { deckCard, paintCard };
})();
