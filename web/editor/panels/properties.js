// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the property form for the selected element: syncing its values, reading and validating
   edits into the draft, the badges that mark held and rule values, and notes on what the draft's theme draws its
   own way. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const Render = window.SceneuronRender;
  const Themes = window.SceneuronThemes;
  const { KIND_NAMES, MAX_NAME_CHARS, MAX_TEXT_CHARS, NUMBER_LIMITS } = Editor.constants;
  const { findElement } = Editor.project;
  const { isValidName, isValidText, joinColor, parseNumber, sameColor, splitColor } = Editor.values;
  const { clearInvalid, setInvalid } = Editor.dom;
  const { buildPropertyFields } = Editor.propertyFields;

  /** Builds and wires the property form; returns the view with render, syncValues, and renderBadges. */
  Editor.properties = function setupProperties(editor) {
    const { app, ui, byId } = editor;
    buildPropertyFields(ui.propsForm);
    const propName = byId("prop-name");
    const propInputs = Array.from(ui.propsForm.querySelectorAll("[data-prop]"));

    function setOutput(field, text) {
      const output = byId(`prop-${field}-output`);
      if (output) {
        output.textContent = text;
      }
      const range = byId(`prop-${field}-alpha`) || byId(`prop-${field}`);
      if (range && range.type === "range") {
        range.setAttribute("aria-valuetext", text);
      }
    }

    /** Copies the draft's values into the form; unless `force`, the focused control keeps its entry. */
    function syncValues(force) {
      const element = findElement(app.draft, app.formFor);
      if (!element) {
        return;
      }
      const active = document.activeElement;
      if (force || active !== propName) {
        propName.value = element.name;
        clearInvalid(propName);
      }
      for (const input of propInputs) {
        if (!force && input === active) {
          continue;
        }
        clearInvalid(input);
        const field = input.getAttribute("data-prop");
        const value = field in element.props ? element.props[field] : Render.DEFAULT_PROPS[field];
        const type = input.getAttribute("data-type");
        if (type === "bool") {
          input.checked = value !== false;
        } else if (type === "percent") {
          input.value = String(Math.round(Number(value) * 100));
          setOutput(field, `${input.value}%`);
        } else if (type === "color") {
          const parts = splitColor(value);
          if (input.getAttribute("data-part") === "alpha") {
            input.value = String(Math.round(parts.alpha * 100));
            setOutput(field, `${input.value}%`);
          } else {
            input.value = parts.rgb;
          }
        } else {
          input.value = value === undefined || value === null ? "" : String(value);
        }
      }
    }

    function renderBadges() {
      const live = editor.liveElement(app.formFor);
      const held = new Set(live ? live.overridden : []);
      const automated = new Set(live ? live.automated : []);
      for (const badge of ui.propsForm.querySelectorAll("[data-badge]")) {
        const field = badge.getAttribute("data-badge");
        const text = held.has(field) ? "held" : automated.has(field) ? "rule" : "";
        if (badge.textContent !== text) {
          badge.textContent = text;
        }
      }
    }

    function setNote(id, shown, text) {
      const node = byId(id);
      node.hidden = !shown;
      if (shown && node.textContent !== text) {
        node.textContent = text;
      }
    }

    /**
     * Says which fields the draft's theme draws its own way, and that a window theme shows the element's name on
     * stream; a card slot's window is titled with its deck's name instead.
     */
    function renderThemeNotes(element) {
      const theme = Themes.themeById(app.draft.theme);
      const themed = theme.id !== "none";
      const slot = (app.draft.decks || []).some((deck) => deck.element === element.id);
      const { name } = theme;
      setNote("theme-name-note", theme.titles && !slot, `The ${name} theme shows this name on stream, in a title bar.`);
      setNote(
        "theme-appearance-note",
        themed,
        `The ${name} theme draws its own backgrounds, borders, and corners; choose None in the Settings tab to ` +
          "use these.",
      );
      setNote(
        "theme-color-note",
        themed && element.kind === "text",
        `The ${name} theme draws text in its own color, or in its second color when this one has a hue, as ` +
          "yellow does.",
      );
    }

    function render() {
      const element = findElement(app.draft, app.selectedId);
      ui.propsEmpty.hidden = Boolean(element);
      ui.propsForm.hidden = !element;
      if (!element) {
        app.formFor = null;
        return;
      }
      if (app.formFor !== element.id) {
        app.formFor = element.id;
        syncValues(true);
      }
      byId("prop-id").textContent = element.id;
      byId("prop-kind").textContent = `${(KIND_NAMES[element.kind] || element.kind).toLowerCase()} element`;
      const isText = element.kind === "text";
      byId("text-fields").disabled = !isText;
      byId("text-note").hidden = isText;
      const isFrame = element.kind === "frame";
      for (const input of ui.propsForm.querySelectorAll('[data-prop="background"]')) {
        input.disabled = isFrame;
      }
      byId("background-note").hidden = !isFrame;
      renderThemeNotes(element);
      renderBadges();
    }

    function readPropInput(input, field, current, saved) {
      const type = input.getAttribute("data-type");
      if (type === "number") {
        const [min, max] = NUMBER_LIMITS[field];
        const value = parseNumber(input.value, min, max, false);
        return value === null ? { error: `Enter a number from ${min} to ${max}.` } : { value };
      }
      if (type === "percent") {
        setOutput(field, `${input.value}%`);
        return { value: Number(input.value) / 100 };
      }
      if (type === "bool") {
        return { value: input.checked };
      }
      if (type === "color") {
        // Changing only the hue keeps the exact saved alpha byte, which the percent slider cannot show.
        const alphaInput = byId(`prop-${field}-alpha`);
        const alpha = input === alphaInput ? Number(alphaInput.value) / 100 : splitColor(current).alpha;
        setOutput(field, `${Math.round(alpha * 100)}%`);
        // Reuse an existing spelling of the same color so returning to it is not an unsaved change.
        const value = joinColor(byId(`prop-${field}`).value, alpha);
        const spelling = [saved, current].find((candidate) => sameColor(value, candidate));
        return { value: spelling === undefined ? value : spelling };
      }
      if (field === "text" && !isValidText(input.value)) {
        return { error: `Use at most ${MAX_TEXT_CHARS} characters, without control characters.` };
      }
      return { value: input.value };
    }

    function onPropInput(event) {
      const input = event.target;
      const element = findElement(app.draft, app.formFor);
      if (!element) {
        return;
      }
      if (input === propName) {
        if (!isValidName(input.value)) {
          setInvalid(input, `Enter 1 to ${MAX_NAME_CHARS} characters.`);
          return;
        }
        clearInvalid(input);
        if (element.name !== input.value) {
          element.name = input.value;
          editor.draftChanged({ list: true, rules: true });
        }
        return;
      }
      const field = input.getAttribute("data-prop");
      if (!field) {
        return;
      }
      const savedElement = findElement(app.saved, element.id);
      const result = readPropInput(input, field, element.props[field], savedElement && savedElement.props[field]);
      if (result.error) {
        setInvalid(input, result.error);
        return;
      }
      clearInvalid(input);
      if (element.props[field] !== result.value) {
        element.props[field] = result.value;
        editor.draftChanged({ list: field === "visible" });
      }
    }

    ui.propsForm.addEventListener("input", onPropInput);
    ui.propsForm.addEventListener("change", onPropInput);
    ui.propsForm.addEventListener("submit", (event) => event.preventDefault());

    return { render, renderBadges, syncValues };
  };
})();
