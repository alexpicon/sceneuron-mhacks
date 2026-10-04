// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: building DOM nodes safely, keeping focus across rebuilds, and marking invalid fields. */
(function () {
  "use strict";

  /** Creates an element; strings become text nodes, so data is never parsed as HTML. */
  function h(tag, attrs, ...children) {
    const node = document.createElement(tag);
    let value;
    for (const [key, attr] of Object.entries(attrs || {})) {
      if (attr === null || attr === undefined || attr === false) {
        continue;
      }
      if (key === "value") {
        value = attr;
      } else if (key === "checked") {
        node.checked = Boolean(attr);
      } else {
        node.setAttribute(key, attr === true ? "" : String(attr));
      }
    }
    for (const child of children) {
      if (child !== null && child !== undefined && child !== false) {
        node.append(child);
      }
    }
    if (value !== undefined) {
      node.value = String(value);
    }
    return node;
  }

  function hidden(text) {
    return h("span", { class: "visually-hidden" }, text);
  }

  function option(value, label) {
    return h("option", { value }, label);
  }

  /** A labelled control stacked under its label. */
  function field(id, label, control) {
    return h("div", { class: "field" }, h("label", { for: id }, label), control);
  }

  /** A checkbox followed by its label. */
  function check(id, label, control) {
    return h("div", { class: "check" }, control, h("label", { for: id }, label));
  }

  /** Rebuilds part of the page and puts focus back on the control with the same focus key. */
  function withFocusRestore(container, rebuild) {
    const active = document.activeElement;
    const key = active && container.contains(active) ? active.getAttribute("data-focus-key") : null;
    const hasCaret = key && typeof active.selectionStart === "number";
    const selection = hasCaret ? [active.selectionStart, active.selectionEnd] : null;
    rebuild();
    if (key) {
      focusKey(container, key, selection);
    }
  }

  function focusKey(container, key, selection) {
    const target = Array.from(container.querySelectorAll("[data-focus-key]")).find(
      (node) => node.getAttribute("data-focus-key") === key,
    );
    if (!target) {
      return false;
    }
    target.focus();
    if (selection) {
      target.setSelectionRange(selection[0], selection[1]);
    }
    return true;
  }

  function setInvalid(input, message) {
    const id = `${input.id}-error`;
    let note = document.getElementById(id);
    if (!note) {
      note = h("p", { id, class: "field-error" });
      (input.closest(".field") || input.parentElement).append(note);
    }
    note.textContent = message;
    input.setAttribute("aria-invalid", "true");
    input.setAttribute("aria-describedby", id);
  }

  function clearInvalid(input) {
    const note = document.getElementById(`${input.id}-error`);
    if (note) {
      note.remove();
    }
    input.removeAttribute("aria-invalid");
    input.removeAttribute("aria-describedby");
  }

  window.SceneuronEditor.dom = {
    check,
    clearInvalid,
    field,
    focusKey,
    h,
    hidden,
    option,
    setInvalid,
    withFocusRestore,
  };
})();
