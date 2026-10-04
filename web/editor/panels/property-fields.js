// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the property form, built from one description of every element property
   so labels, limits, and data attributes cannot drift apart, with the notes that say which fields a theme draws
   its own way. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_NAME_CHARS, MAX_TEXT_CHARS, NUMBER_LIMITS } = Editor.constants;
  const { h, option } = Editor.dom;

  /** A field's label with the badge that marks a held or rule value. */
  function label(prop, text) {
    return h("label", { for: `prop-${prop}` }, `${text} `, h("span", { class: "badge", "data-badge": prop }));
  }

  function number(prop, text) {
    const [min, max] = NUMBER_LIMITS[prop];
    const input = h("input", {
      id: `prop-${prop}`,
      type: "number",
      min,
      max,
      step: 1,
      "data-prop": prop,
      "data-type": "number",
    });
    return h("div", { class: "field" }, label(prop, text), input);
  }

  function checkbox(prop, text) {
    const input = h("input", { id: `prop-${prop}`, type: "checkbox", "data-prop": prop, "data-type": "bool" });
    return h("div", { class: "check" }, input, label(prop, text));
  }

  function percent(prop, text) {
    const id = `prop-${prop}`;
    const range = h("input", {
      id,
      type: "range",
      min: 0,
      max: 100,
      step: 1,
      "data-prop": prop,
      "data-type": "percent",
    });
    const output = h("output", { id: `${id}-output`, for: id });
    return h("div", { class: "field" }, label(prop, text), h("div", { class: "range-row" }, range, output));
  }

  /** A color picker plus an alpha slider; `short` names the color in the slider's hidden label. */
  function color(prop, text, short, attrs, ...extra) {
    const id = `prop-${prop}`;
    const data = { "data-prop": prop, "data-type": "color" };
    const picker = h("input", Object.assign({ id, type: "color" }, data, { "data-part": "rgb" }));
    const hiddenName = h("span", { class: "visually-hidden" }, `${short} `);
    const alphaLabel = h("label", { for: `${id}-alpha`, class: "sub-label" }, hiddenName, "Alpha");
    const range = { id: `${id}-alpha`, type: "range", min: 0, max: 100, step: 1 };
    const alpha = h("input", Object.assign(range, data, { "data-part": "alpha" }));
    const output = h("output", { id: `${id}-output`, for: `${id}-alpha` });
    const row = h("div", { class: "color-row" }, picker, alphaLabel, alpha, output);
    return h("div", Object.assign({ class: "field" }, attrs), label(prop, text), row, ...extra);
  }

  function select(prop, text, choices) {
    const options = choices.map(([value, name]) => option(value, name));
    const control = h("select", { id: `prop-${prop}`, "data-prop": prop, "data-type": "select" }, ...options);
    return h("div", { class: "field" }, label(prop, text), control);
  }

  function textArea(prop, text) {
    const attrs = { id: `prop-${prop}`, rows: 2, maxlength: MAX_TEXT_CHARS, "data-prop": prop, "data-type": "text" };
    return h("div", { class: "field" }, label(prop, text), h("textarea", attrs));
  }

  /** Where placeholder-chips.js lists the placeholders that the text can show, and warns about unknown ones. */
  function placeholderRow() {
    const row = { id: "text-placeholders", class: "chip-row", role: "group" };
    row["aria-labelledby"] = "text-placeholders-label";
    return h(
      "div",
      { class: "placeholders" },
      h("p", { id: "text-placeholders-label", class: "hint" }, "Insert live text from the AI:"),
      h("div", row),
      h("p", { id: "prop-text-unknown", class: "field-warning", hidden: true }),
    );
  }

  const grid = (...fields) => h("div", { class: "grid-2" }, ...fields);
  const group = (attrs, legend, ...fields) => h("fieldset", attrs, h("legend", {}, legend), ...fields);
  const note = (id, text) => h("p", { id, class: "hint", hidden: true }, text);

  /** Fills the empty property form with a field for every element property. */
  function buildPropertyFields(form) {
    const name = h("input", {
      id: "prop-name",
      type: "text",
      maxlength: MAX_NAME_CHARS,
      autocomplete: "off",
      "data-meta": "name",
    });
    const intro = h(
      "p",
      { class: "hint" },
      "Id ",
      h("code", { id: "prop-id" }),
      ", ",
      h("span", { id: "prop-kind" }),
      ". Badges mark values that a manual hold or a rule currently replaces in the live overlay.",
    );
    const backgroundNote = note("background-note", "Frames are transparent inside.");
    form.replaceChildren(
      h("div", { class: "field" }, h("label", { for: "prop-name" }, "Name"), name, note("theme-name-note", "")),
      intro,
      group(
        {},
        "Position and size",
        grid(number("x", "X"), number("y", "Y"), number("width", "Width"), number("height", "Height")),
      ),
      group(
        {},
        "Appearance",
        checkbox("visible", "Visible"),
        percent("opacity", "Opacity"),
        note("theme-appearance-note", ""),
        color("background", "Background", "Background", { id: "background-field" }, backgroundNote),
        color("border_color", "Border color", "Border", {}),
        grid(number("border_width", "Border width"), number("corner_radius", "Corner radius")),
      ),
      group(
        { id: "text-fields" },
        "Text",
        note("text-note", "Only text elements draw these fields."),
        textArea("text", "Text"),
        placeholderRow(),
        grid(
          number("font_size", "Font size"),
          select("align", "Alignment", [["left", "Left"], ["center", "Center"], ["right", "Right"]]),
        ),
        checkbox("bold", "Bold"),
        color("color", "Text color", "Text", {}, note("theme-color-note", "")),
      ),
      group(
        {},
        "Animation",
        grid(
          select("enter", "When shown", [["none", "None"], ["fade", "Fade"], ["slide-up", "Slide up"], ["pop", "Pop"]]),
          select("idle", "While visible", [["none", "None"], ["pulse", "Pulse"], ["glow", "Glow"], ["float", "Float"]]),
        ),
      ),
    );
  }

  Editor.propertyFields = { buildPropertyFields };
})();
