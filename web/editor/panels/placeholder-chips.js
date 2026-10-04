// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the placeholder chips under the selected element's text. A click inserts "{{scene}}" or
   a condition's "{{id}}" at the caret, and a warning names placeholders that would show nothing on stream. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_TEXT_CHARS } = Editor.constants;
  const { findElement } = Editor.project;
  const { availablePlaceholders, insertToken, unknownPlaceholders } = Editor.placeholders;
  const { h, withFocusRestore } = Editor.dom;

  /** Wires the chips; returns the view with render. */
  Editor.placeholderChips = function setupPlaceholderChips(editor) {
    const { app, byId } = editor;
    const row = byId("text-placeholders");
    const warning = byId("prop-text-unknown");
    const textArea = byId("prop-text");
    let chipsKey = "";
    // The caret the streamer last left in the text, for the element it belongs to.
    let caret = null;

    function renderWarning() {
      const element = findElement(app.draft, app.formFor);
      const unknown = element && element.kind === "text" ? unknownPlaceholders(textArea.value, app.draft) : [];
      const tokens = unknown.map((name) => `{{${name}}}`).join(", ");
      const verb = unknown.length === 1 ? "is not a condition, so it shows" : "are not conditions, so they show";
      const text = unknown.length ? `${tokens} ${verb} nothing on stream.` : "";
      warning.hidden = !text;
      if (warning.textContent !== text) {
        warning.textContent = text;
      }
    }

    function render() {
      const items = availablePlaceholders(app.draft);
      const key = JSON.stringify(items);
      if (key !== chipsKey) {
        chipsKey = key;
        withFocusRestore(row, () => {
          const chips = items.map((item) =>
            h(
              "button",
              {
                type: "button",
                class: "placeholder-chip",
                "data-token": item.token,
                "data-focus-key": `placeholder:${item.token}`,
                title: `Shows ${item.meaning}`,
              },
              h("span", {}, item.name),
              " ",
              h("code", {}, item.token),
            )
          );
          row.replaceChildren(...chips);
        });
      }
      renderWarning();
    }

    const rememberCaret = () => {
      caret = { element: app.formFor, start: textArea.selectionStart, end: textArea.selectionEnd };
    };
    for (const type of ["keyup", "mouseup", "input", "select"]) {
      textArea.addEventListener(type, rememberCaret);
    }
    textArea.addEventListener("input", renderWarning);

    row.addEventListener("click", (event) => {
      const button = event.target.closest("[data-token]");
      if (!button || textArea.disabled) {
        return;
      }
      const token = button.getAttribute("data-token");
      const known = caret && caret.element === app.formFor;
      const end = textArea.value.length;
      const result = insertToken(textArea.value, known ? caret.start : end, known ? caret.end : end, token);
      if (Array.from(result.text).length > MAX_TEXT_CHARS) {
        editor.toast(`Text can have at most ${MAX_TEXT_CHARS} characters.`, "error");
        return;
      }
      textArea.value = result.text;
      textArea.focus();
      textArea.setSelectionRange(result.caret, result.caret);
      textArea.dispatchEvent(new Event("input", { bubbles: true }));
      editor.announce(`Inserted ${token}.`);
    });

    return { render };
  };
})();
