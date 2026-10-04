// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the project settings forms: the overlay canvas, presence detection, and fun reactions in the
   Settings tab, and the AI pace and budget in the AI tab. Each input names its project field with data-canvas,
   data-detection, data-reactions, or data-ai; data-scale converts the shown unit (seconds, percent) to the saved
   one, and a checkbox edits a true or false field. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { parseNumber } = Editor.values;
  const { clearInvalid, setInvalid } = Editor.dom;

  const GROUPS = { canvas: "canvas", detection: "detection", reactions: "reactions", ai: "ai" };

  /** Wires the settings forms; returns the view with sync, which copies the draft's settings into them. */
  Editor.settings = function setupSettings(editor) {
    const { app, ui, byId } = editor;
    const forms = [ui.settingsForm, byId("ai-policy-form")].filter(Boolean);
    const settingInputs = forms.flatMap((form) => Array.from(form.querySelectorAll("input")));

    /** The project field an input edits, as the settings group name and the field name. */
    function target(input) {
      for (const group of Object.keys(GROUPS)) {
        const name = input.getAttribute(`data-${group}`);
        if (name) {
          return { group, name };
        }
      }
      return null;
    }

    const scaleOf = (input) => Number(input.getAttribute("data-scale")) || 1;
    const shown = (value, scale) => String(Number((Number(value) / scale).toFixed(6)));

    function sync() {
      if (!app.draft) {
        return;
      }
      for (const input of settingInputs) {
        const field = target(input);
        const group = field && app.draft[field.group];
        if (field && input.type === "checkbox") {
          input.checked = Boolean(group && group[field.name]);
          continue;
        }
        input.value = group && field.name in group ? shown(group[field.name], scaleOf(input)) : "";
        clearInvalid(input);
      }
    }

    /** Why a pace value conflicts with the other one, or null: the refresh may not be shorter than the interval. */
    function paceProblem(name, value) {
      const ai = app.draft.ai || {};
      if (name === "refresh_ms" && value < ai.min_interval_ms) {
        return `Use at least the shortest time between looks (${shown(ai.min_interval_ms, 1000)} seconds).`;
      }
      if (name === "min_interval_ms" && value > ai.refresh_ms) {
        return `Use at most the time to look again anyway (${shown(ai.refresh_ms, 1000)} seconds).`;
      }
      return null;
    }

    function onInput(event) {
      const input = event.target;
      const field = input.tagName === "INPUT" ? target(input) : null;
      if (!app.draft || !field || !app.draft[field.group]) {
        return;
      }
      if (input.type === "checkbox") {
        app.draft[field.group][field.name] = input.checked;
        editor.draftChanged({});
        return;
      }
      const integer = input.getAttribute("data-integer") === "true";
      const min = Number(input.min);
      const max = Number(input.max);
      const parsed = parseNumber(input.value, min, max, integer);
      if (parsed === null) {
        setInvalid(input, `Enter a ${integer ? "whole number" : "number"} from ${min} to ${max}.`);
        return;
      }
      const scaled = Number((parsed * scaleOf(input)).toFixed(6));
      const value = input.getAttribute("data-integer-scaled") === "true" ? Math.round(scaled) : scaled;
      const problem = field.group === "ai" ? paceProblem(field.name, value) : null;
      if (problem) {
        setInvalid(input, problem);
        return;
      }
      clearInvalid(input);
      app.draft[field.group][field.name] = value;
      editor.draftChanged({});
    }

    for (const form of forms) {
      form.addEventListener("input", onInput);
      form.addEventListener("submit", (event) => event.preventDefault());
    }

    return { sync };
  };
})();
