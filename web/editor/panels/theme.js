// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the overlay theme picker at the top of the Settings tab. Choosing a theme changes the draft,
   so the preview shows it at once and the overlay once the draft is saved. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const Themes = window.SceneuronThemes;
  const { h } = Editor.dom;

  /** A small sample of the theme's colors and font; it adds nothing to the theme's name, so readers skip it. */
  function swatch(theme) {
    const node = h("span", { class: "theme-swatch", "aria-hidden": "true" }, "Aa");
    node.style.setProperty("color", theme.colors[theme.swatch.text]);
    node.style.setProperty("background-color", theme.colors[theme.swatch.panel]);
    node.style.setProperty("font-family", theme.fonts[0]);
    return node;
  }

  function option(theme) {
    const id = `theme-${theme.id}`;
    return h(
      "div",
      { class: "theme-option" },
      h("input", { type: "radio", id, name: "overlay-theme", value: theme.id, "aria-describedby": `${id}-hint` }),
      h("label", { for: id }, swatch(theme), h("span", { class: "theme-name" }, theme.name)),
      h("p", { id: `${id}-hint`, class: "hint" }, theme.description),
    );
  }

  /** Builds and wires the picker; returns the view with sync, which checks the draft's theme. */
  Editor.theme = function setupTheme(editor) {
    const { app, byId } = editor;
    const form = byId("theme-form");
    const titled = Themes.THEMES.filter((theme) => theme.titles).map((theme) => theme.name);
    form.replaceChildren(
      h(
        "fieldset",
        { class: "theme-picker" },
        h("legend", {}, "Overlay theme"),
        h(
          "p",
          { class: "hint" },
          "A theme restyles every card, subtitle, label, and frame; their places, sizes, and words stay. " +
            `${titled.join(" and ")} title each element with its name, on stream. The preview shows a theme ` +
            "now, and the overlay after you save.",
        ),
        ...Themes.THEMES.map(option),
      ),
    );
    const inputs = Array.from(form.querySelectorAll('input[name="overlay-theme"]'));

    function sync() {
      const current = Themes.themeId(app.draft && app.draft.theme);
      for (const input of inputs) {
        input.checked = input.value === current;
        input.disabled = !app.draft;
      }
    }

    form.addEventListener("change", (event) => {
      const input = event.target;
      if (input.name !== "overlay-theme" || !input.checked || !app.draft) {
        return;
      }
      app.draft.theme = input.value;
      editor.draftChanged({ theme: true });
      editor.announce(`Save to show the ${Themes.themeById(input.value).name} theme on the overlay.`);
    });
    form.addEventListener("submit", (event) => event.preventDefault());

    return { sync };
  };
})();
