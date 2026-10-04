// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: "Add a pack" in the Cards tab: the ready-made packs the server offers, fitted to the saved
   canvas, and adding one to the draft, which the streamer then saves as usual. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { packSize } = Editor.cardsText;
  const { addPack, findDeck } = Editor.deckEdits;
  const { findElement } = Editor.project;
  const { h, hidden, withFocusRestore } = Editor.dom;

  /** Builds and wires the pack list; returns the view with load, which fetches the packs, and render. */
  Editor.packs = function setupPacks(editor) {
    const { app, api, byId } = editor;
    byId("packs-section").replaceChildren(
      h("h3", { id: "packs-heading" }, "Add a pack"),
      h(
        "p",
        { class: "hint" },
        "A pack adds a deck of cards and the element that shows them, the subtitles bar, or the live description " +
          "bar. Save afterwards to use it.",
      ),
      h("ul", { id: "pack-list", class: "pack-list" }),
      h("p", { id: "packs-status", class: "hint" }, "Loading the packs."),
    );
    const list = byId("pack-list");
    const status = byId("packs-status");
    let packs = [];

    function render() {
      withFocusRestore(list, () => {
        const items = packs.map((pack) => {
          const added = pack.decks.length > 0
            ? pack.decks.every((deck) => findDeck(app.draft, deck.id))
            : Boolean(app.draft) && pack.elements.every((element) => findElement(app.draft, element.id));
          return h(
            "li",
            {},
            h(
              "div",
              {},
              h("span", { class: "pack-name" }, pack.name),
              h("span", { class: "hint" }, added ? ` ${packSize(pack)}, in this project` : ` ${packSize(pack)}`),
              h("p", { class: "hint" }, pack.description),
            ),
            h(
              "button",
              { type: "button", "data-pack-id": pack.id, "data-focus-key": `pack:${pack.id}`, disabled: !app.draft },
              "Add",
              hidden(` ${pack.name}`),
            ),
          );
        });
        list.replaceChildren(...items);
      });
    }

    /** Fetches the packs again, so they follow the saved canvas size. */
    async function load() {
      try {
        const reply = await api.packs();
        packs = Array.isArray(reply) ? reply : [];
        status.textContent = packs.length ? "" : "Sceneuron offers no packs.";
        status.hidden = packs.length > 0;
        render();
      } catch (error) {
        status.textContent = `Could not load the packs: ${error.message}`;
        status.hidden = false;
      }
    }

    list.addEventListener("click", (event) => {
      const button = event.target.closest("[data-pack-id]");
      const pack = button && packs.find((candidate) => candidate.id === button.getAttribute("data-pack-id"));
      if (!pack || !app.draft) {
        return;
      }
      let decks;
      try {
        decks = addPack(app.draft, pack);
      } catch (error) {
        editor.report(error);
        return;
      }
      editor.draftChanged({ list: true, rules: true, decks: true });
      const heading = decks.length ? byId(`deck-${app.draft.decks.indexOf(decks[0])}-heading`) : null;
      if (heading) {
        heading.focus();
      }
      editor.announce(`Added ${pack.name} with ${packSize(pack)}. Save to use it.`);
    });

    return { load, render };
  };
})();
