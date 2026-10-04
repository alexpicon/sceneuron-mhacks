// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Cards tab's decks: raising and lowering cards live, following which card each deck shows
   and where hand signals go in the live snapshot, and applying deck settings, card edits, and deck deletions to the
   draft. The tab's other sections, packs.js, room.js, and stream-deck.js, fill the sections this view creates. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_NAME_CHARS, MAX_TEXT_CHARS } = Editor.constants;
  const { raisedStatus } = Editor.cardsText;
  const { cardValue, deleteDeck, findCard, isSavedCard, setAutoHide, setCardField } = Editor.deckEdits;
  const { findElement } = Editor.project;
  const { formatClock } = Editor.snapshot;
  const { isValidName, isValidText, joinColor, sameColor, splitColor } = Editor.values;
  const { clearInvalid, focusKey, h, setInvalid, withFocusRestore } = Editor.dom;
  const { deckCard, paintCard } = Editor.deckCards;
  const { readCardVoice } = Editor.deckVoice;
  const { readSignal } = Editor.signalsText;
  const { routeLine, steadyClock } = Editor.signalsContext;

  function markup() {
    return [
      h(
        "p",
        { class: "hint" },
        "A deck shows one card at a time in an element of the overlay, such as a planning poker vote or a ",
        "meeting signal. A card's button raises it live, or lowers it when it is up.",
      ),
      h("h3", { id: "decks-heading", tabindex: "-1" }, "Decks"),
      h("p", { id: "hand-signals-route", class: "hint", role: "status", hidden: true }),
      h("div", { id: "deck-list", class: "decks" }),
      h("p", { id: "decks-empty", class: "hint", hidden: true }, "No decks yet. Add a pack to get one."),
      h("section", { id: "packs-section", "aria-labelledby": "packs-heading" }),
      h("section", { id: "room-section", "aria-labelledby": "room-heading" }),
      h("section", { id: "stream-deck-section", "aria-labelledby": "stream-deck-heading" }),
    ];
  }

  function setText(node, text) {
    if (node.textContent !== text) {
      node.textContent = text;
    }
  }

  /** Builds and wires the Cards tab's decks; returns the view with render, refresh, and renderLive. */
  Editor.cards = function setupCards(editor) {
    const { app, api, byId } = editor;
    byId("panel-cards").replaceChildren(...markup());
    const list = byId("deck-list");
    const empty = byId("decks-empty");
    // The card each deck's editor shows, by deck id, across rebuilds.
    const editing = new Map();
    // Set when refresh() skips a redraw for a refused entry.
    let stale = false;

    const decks = () => (app.draft && app.draft.decks) || [];
    const articleOf = (di) => list.querySelector(`article[data-deck-index="${di}"]`);
    const editedCard = (deck) => findCard(deck, editing.get(deck.id)) || deck.cards[0];
    const hasRefusedEntry = () => Boolean(list.querySelector('[aria-invalid="true"]'));
    const voteClock = steadyClock((ms) => formatClock(app.liveReceivedAt + ms - app.live.now_ms));

    function render() {
      stale = false;
      // Each card editor stays open or closed across the rebuild, as the streamer left it.
      const open = new Set(
        Array.from(list.querySelectorAll("details[open]"), (details) => details.getAttribute("data-deck-id")),
      );
      withFocusRestore(list, () => {
        const state = (deck) => ({ editing: editing.get(deck.id), open: open.has(deck.id) });
        list.replaceChildren(...decks().map((deck, di) => deckCard(app.draft, deck, di, state(deck))));
      });
      empty.hidden = decks().length > 0;
      renderLive();
    }

    /**
     * Redraws the decks as the tab is shown, so they follow edits made on other tabs, such as a card slot's colors,
     * which cards without their own show. A refused entry keeps the decks as they are, so saving can point to it;
     * they redraw once the last refused entry is fixed.
     */
    function refresh() {
      if (hasRefusedEntry()) {
        stale = true;
      } else {
        render();
      }
    }

    /** Says where hand signals go and marks each deck's raised card; as live regions, both announce a change. */
    function renderLive() {
      const live = app.live;
      const route = byId("hand-signals-route");
      setText(route, routeLine(live, voteClock));
      route.hidden = !route.textContent;
      const views = new Map(((live && live.decks) || []).map((view) => [view.id, view]));
      decks().forEach((deck, di) => {
        const article = articleOf(di);
        if (!article) {
          return;
        }
        const view = views.get(deck.id);
        setText(article.querySelector('[data-live="status"]'), raisedStatus(deck, view));
        const hideAt = article.querySelector('[data-live="hide-at"]');
        const lowers = view && view.raised && Number.isFinite(view.hide_at_ms);
        const at = lowers ? formatClock(app.liveReceivedAt + view.hide_at_ms - live.now_ms) : "";
        setText(hideAt, at ? `It lowers by itself at ${at}.` : "");
        hideAt.hidden = !at;
        for (const button of article.querySelectorAll('[data-card-command="toggle"]')) {
          const id = button.getAttribute("data-card-id");
          button.setAttribute("aria-pressed", String(Boolean(view) && view.raised === id));
          button.disabled = !view || !isSavedCard(app.saved, deck.id, id);
        }
        // Lower stays enabled while nothing is up, so pressing it never drops the keyboard focus.
        article.querySelector('[data-card-command="lower"]').disabled = !view;
      });
    }

    /** Reads a card field into the card being edited; returns an error message, or null when it was applied. */
    function editCard(deck, di, target, name) {
      const card = editedCard(deck);
      const slot = findElement(app.draft, deck.element);
      let value = target.value;
      if (name === "name" && !isValidName(value)) {
        return `Enter 1 to ${MAX_NAME_CHARS} characters.`;
      }
      if (name === "text" && !isValidText(value)) {
        return `Use at most ${MAX_TEXT_CHARS} characters, without control characters.`;
      }
      const read = name === "signal" ? readSignal(app.draft, deck.id, card.id, value) : readCardVoice(name, value);
      if (read.problem) {
        return read.problem;
      }
      value = read.value;
      if (name === "color" || name === "background") {
        // Changing only the hue keeps the exact alpha byte, which the percent slider cannot show.
        const current = cardValue(card, slot, name);
        const alphaInput = byId(`deck-${di}-card-${name}-alpha`);
        const alpha = target === alphaInput ? Number(alphaInput.value) / 100 : splitColor(current).alpha;
        const percent = `${Math.round(alpha * 100)}%`;
        byId(`deck-${di}-card-${name}-output`).textContent = percent;
        alphaInput.setAttribute("aria-valuetext", percent);
        // Changing only the alpha keeps the draft's hue: the picker can still show one from before a skipped redraw.
        const rgb = target === alphaInput ? splitColor(current).rgb : byId(`deck-${di}-card-${name}`).value;
        value = joinColor(rgb, alpha);
        if (sameColor(value, current)) {
          return null;
        }
      }
      setCardField(card, name, value);
      const button = articleOf(di).querySelector(`[data-card-id=${JSON.stringify(card.id)}]`);
      if (name === "name") {
        button.querySelector("[data-card-name]").textContent = value;
        byId(`deck-${di}-edit-card`).selectedOptions[0].textContent = value;
      }
      paintCard(button, card, slot);
      editor.draftChanged({});
      return null;
    }

    function onInput(event) {
      const target = event.target;
      const isToggle = target.tagName === "SELECT" || target.type === "checkbox";
      if ((event.type === "input") === isToggle) {
        return;
      }
      const di = Number(target.getAttribute("data-deck-index"));
      const deck = decks()[di];
      if (!deck) {
        return;
      }
      const deckField = target.getAttribute("data-deck-field");
      if (deckField === "auto_hide_ms") {
        setAutoHide(deck, Number(target.value));
        editor.draftChanged({});
      } else if (["camera", "voice", "speak"].includes(deckField)) {
        deck[deckField] = target.checked;
        editor.draftChanged({});
        // Which decks listen decides what the card editors warn about.
        if (deckField === "camera") {
          refresh();
        }
      } else if (deckField === "edit-card") {
        editing.set(deck.id, target.value);
        render();
      }
      const cardField = target.getAttribute("data-card-field");
      if (cardField) {
        const problem = editCard(deck, di, target, cardField);
        if (problem) {
          setInvalid(target, problem);
        } else {
          clearInvalid(target);
          // The fixed entry is in the draft already, and the rebuild keeps the focus and the caret in it. An input
          // method's composition would go on in the replaced field, so the rebuild waits for it to end. A new hand
          // signal changes the notes on what it shares with other cards.
          stale = stale || cardField === "signal";
          if (stale && !event.isComposing && !hasRefusedEntry()) {
            render();
          }
        }
      }
    }

    function remove(deck, withSlot) {
      const slot = withSlot ? findElement(app.draft, deck.element) : null;
      deleteDeck(app.draft, deck.id, withSlot);
      editing.delete(deck.id);
      editor.draftChanged({ list: true, rules: true, decks: true });
      if (slot && app.selectedId === slot.id) {
        editor.select(app.draft.elements.length ? app.draft.elements[0].id : null);
      }
      byId("decks-heading").focus();
      const also = slot ? ` and the element ${slot.name}` : "";
      editor.announce(`Deleted the ${deck.name} deck${also}. Save to apply it, or discard changes to undo.`);
    }

    function showConfirm(di, shown) {
      const box = articleOf(di).querySelector("[data-confirm]");
      box.hidden = !shown;
      const key = shown ? "delete-confirmed" : "delete-deck";
      focusKey(list, `deck:${decks()[di].id}:${key}`);
    }

    list.addEventListener("input", onInput);
    list.addEventListener("change", onInput);
    list.addEventListener("compositionend", () => {
      if (stale && !hasRefusedEntry()) {
        render();
      }
    });
    list.addEventListener("keydown", (event) => {
      const box = event.target.closest("[data-confirm]");
      if (event.key === "Escape" && box) {
        event.preventDefault();
        showConfirm(Number(box.closest("article").getAttribute("data-deck-index")), false);
      }
    });
    list.addEventListener("click", (event) => {
      const button = event.target.closest("[data-card-command]");
      const di = button ? Number(button.getAttribute("data-deck-index")) : -1;
      const deck = decks()[di];
      if (!deck) {
        return;
      }
      const command = button.getAttribute("data-card-command");
      if (command === "toggle") {
        editor.act(api.card(deck.id, button.getAttribute("data-card-id"), "toggle"));
      } else if (command === "lower") {
        editor.act(api.card(deck.id, null, "lower"));
      } else if (command === "delete-deck") {
        showConfirm(di, true);
      } else if (command === "cancel-delete") {
        showConfirm(di, false);
      } else if (command === "delete-confirmed") {
        const alsoSlot = byId(`deck-${di}-delete-slot`);
        remove(deck, Boolean(alsoSlot && alsoSlot.checked));
      }
    });

    return { refresh, render, renderLive };
  };
})();
