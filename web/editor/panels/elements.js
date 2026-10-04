// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the element list, with selection by click or arrow keys, and adding, among them the ready-made
   live description bar, and deleting elements. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { KIND_NAMES, MAX_ELEMENTS } = Editor.constants;
  const { addElement, deleteElement, findElement } = Editor.project;
  const { addLiveDescription, deckShownBy } = Editor.deckEdits;
  const { h, withFocusRestore } = Editor.dom;

  /** Wires the element list and its buttons; returns the view with render. */
  Editor.elements = function setupElements(editor) {
    const { app, ui, views, api } = editor;
    const kindButtons = Array.from(document.querySelectorAll("[data-add-kind]"));
    const addDescription = editor.byId("add-live-description");
    const addButtons = kindButtons.concat(addDescription);

    function elementMeta(element) {
      const live = editor.liveElement(element.id);
      const props = live ? live.props : element.props;
      const parts = [KIND_NAMES[element.kind] || element.kind, props.visible === false ? "hidden" : "visible"];
      if (!live) {
        parts.push("not saved");
      } else {
        if (live.overridden && live.overridden.length) {
          parts.push("held");
        }
        if (live.automated && live.automated.length) {
          parts.push("rule");
        }
      }
      return parts.join(", ");
    }

    function render() {
      const elements = app.draft ? app.draft.elements : [];
      for (const button of addButtons) {
        button.disabled = !app.draft || elements.length >= MAX_ELEMENTS;
      }
      ui.deleteElement.disabled = !findElement(app.draft, app.selectedId);
      const items = elements.map((element) => [element.id, element.name, elementMeta(element)]);
      const key = JSON.stringify([items, app.selectedId]);
      if (key === app.listKey) {
        return;
      }
      app.listKey = key;
      withFocusRestore(ui.list, () => {
        const nodes = items.map(([id, name, meta]) =>
          h(
            "li",
            {},
            h(
              "button",
              {
                type: "button",
                class: "element-item",
                "data-element-id": id,
                "data-focus-key": `element:${id}`,
                "aria-current": id === app.selectedId ? "true" : null,
              },
              h("span", { class: "element-name" }, name),
              h("span", { class: "element-meta" }, meta),
            ),
          )
        );
        ui.list.replaceChildren(...nodes);
      });
    }

    ui.list.addEventListener("click", (event) => {
      const button = event.target.closest("[data-element-id]");
      if (button) {
        editor.select(button.getAttribute("data-element-id"));
      }
    });

    ui.list.addEventListener("keydown", (event) => {
      const buttons = Array.from(ui.list.querySelectorAll("button"));
      const index = buttons.indexOf(document.activeElement);
      const moves = { ArrowDown: index + 1, ArrowUp: index - 1, Home: 0, End: buttons.length - 1 };
      if (index < 0 || !(event.key in moves)) {
        return;
      }
      const target = buttons[Math.min(buttons.length - 1, Math.max(0, moves[event.key]))];
      event.preventDefault();
      target.focus();
      editor.select(target.getAttribute("data-element-id"));
    });

    function selectAdded(element, message) {
      app.selectedId = element.id;
      editor.draftChanged({ list: true, rules: true });
      views.properties.render();
      views.live.render();
      editor.announce(message);
    }

    for (const button of kindButtons) {
      button.addEventListener("click", () => {
        if (!app.draft) {
          return;
        }
        try {
          const reserved = app.live ? app.live.elements.map((element) => element.id) : [];
          const element = addElement(app.draft, button.getAttribute("data-add-kind"), reserved);
          selectAdded(element, `Added ${element.name}. Save to show it in the overlay.`);
        } catch (error) {
          editor.report(error);
        }
      });
    }

    // The bar is a pack the server fits to the saved canvas, so it is fetched on each click.
    addDescription.addEventListener("click", async () => {
      if (!app.draft) {
        return;
      }
      try {
        const packs = await api.packs();
        if (!app.draft) {
          return;
        }
        const { element, added } = addLiveDescription(app.draft, packs);
        const message = added
          ? `Added ${element.name}. Save to show it in the overlay.`
          : `${element.name} is already in this project, and is selected now.`;
        selectAdded(element, message);
      } catch (error) {
        editor.report(error);
      }
    });

    ui.deleteElement.addEventListener("click", () => {
      const element = findElement(app.draft, app.selectedId);
      if (!element) {
        return;
      }
      const index = app.draft.elements.indexOf(element);
      const deck = deckShownBy(app.draft, element.id);
      deleteElement(app.draft, element.id);
      const next = app.draft.elements[Math.min(index, app.draft.elements.length - 1)];
      app.selectedId = next ? next.id : null;
      editor.draftChanged({ list: true, rules: true, decks: true });
      views.properties.render();
      views.live.render();
      if (ui.deleteElement.disabled) {
        addButtons[0].focus();
      }
      const also = deck ? ` and the ${deck.name} deck it shows` : "";
      editor.announce(`Deleted ${element.name}${also}. Save to apply it, or discard changes to undo.`);
    });

    return { render };
  };
})();
