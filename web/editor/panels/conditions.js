// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the AI conditions list in the AI tab: rendering the cards, keeping their live state
   current, applying field edits to the draft, and adding and removing conditions. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_CONDITIONS, MAX_DETAIL_CHARS, MAX_NAME_CHARS, MAX_PROMPT_CHARS } = Editor.constants;
  const { addCondition, isValidDetail, isValidPrompt, oneLine, placeholderUsers, removeCondition, renameCondition } =
    Editor.conditionEdits;
  const { rulesTriggeredBy } = Editor.triggers;
  const { joinWords } = Editor.planText;
  const { isValidName, parseNumber } = Editor.values;
  const { clearInvalid, focusKey, setInvalid, withFocusRestore } = Editor.dom;
  const { conditionCard, fillLive } = Editor.conditionCards;

  /** Ages shown on the cards move on between snapshots at this interval. */
  const AGE_REFRESH_MS = 5000;

  /** Wires the conditions list; returns the view with render and renderLive. */
  Editor.conditions = function setupConditions(editor) {
    const { app, byId } = editor;
    const list = byId("conditions-list");
    const empty = byId("conditions-empty");
    const addButton = byId("add-condition");

    const conditions = () => (app.draft && app.draft.conditions) || [];
    const savedIds = () => ((app.saved && app.saved.conditions) || []).map((condition) => condition.id);
    const cardOf = (ci) => list.querySelector(`article[data-condition-index="${ci}"]`);

    function render() {
      const saved = savedIds();
      withFocusRestore(list, () => {
        list.replaceChildren(...conditions().map((c, ci) => conditionCard(c, ci, saved.includes(c.id))));
      });
      conditions().forEach((condition, ci) => {
        if (!condition.prompt) {
          setInvalid(byId(`condition-${ci}-prompt`), "Describe what the AI should watch for before saving.");
        }
      });
      empty.hidden = conditions().length > 0;
      addButton.disabled = !app.draft || conditions().length >= MAX_CONDITIONS;
      renderLive();
    }

    function renderLive() {
      const live = app.live;
      const views = new Map(((live && live.conditions) || []).map((view) => [view.id, view]));
      const elapsed = Math.max(0, Date.now() - (app.liveReceivedAt || Date.now()));
      conditions().forEach((condition, ci) => {
        const card = cardOf(ci);
        const view = views.get(condition.id);
        if (card) {
          const age = view && live ? Math.max(0, live.now_ms - view.since_ms) + elapsed : 0;
          fillLive(card, view, condition, age);
        }
      });
    }

    /** Reads a field into the condition; returns an error message, or null when it was applied. */
    function apply(condition, name, target) {
      switch (name) {
        case "prompt": {
          const value = oneLine(target.value);
          if (!isValidPrompt(value)) {
            return `Describe it in 1 to ${MAX_PROMPT_CHARS} characters.`;
          }
          condition.prompt = value;
          return null;
        }
        case "detail": {
          const value = oneLine(target.value);
          if (!isValidDetail(value)) {
            return `Use at most ${MAX_DETAIL_CHARS} characters.`;
          }
          condition.detail = value;
          return null;
        }
        case "enabled":
          condition.enabled = target.checked;
          return null;
        case "min_confidence": {
          const value = parseNumber(target.value, 0, 100, false);
          if (value === null) {
            return "Enter a number from 0 to 100.";
          }
          condition.min_confidence = Number((value / 100).toFixed(4));
          return null;
        }
        case "confirm":
        case "release": {
          const value = parseNumber(target.value, 1, 10, true);
          if (value === null) {
            return "Enter a whole number from 1 to 10.";
          }
          condition[name] = value;
          return null;
        }
        default:
          return null;
      }
    }

    function onInput(event) {
      const target = event.target;
      const isToggle = target.type === "checkbox";
      if ((event.type === "input") === isToggle) {
        return;
      }
      const ci = Number(target.getAttribute("data-condition-index"));
      const condition = conditions()[ci];
      const name = target.getAttribute("data-condition-field");
      if (!condition || !name) {
        return;
      }
      if (name === "name") {
        if (!isValidName(target.value)) {
          setInvalid(target, `Enter 1 to ${MAX_NAME_CHARS} characters.`);
          return;
        }
        clearInvalid(target);
        const before = condition.id;
        renameCondition(app.draft, condition, target.value, savedIds());
        cardOf(ci).querySelector("[data-heading]").textContent = condition.name;
        const moved = before !== condition.id;
        editor.draftChanged({ rules: true, chips: true, conditions: moved, form: moved });
        return;
      }
      const problem = apply(condition, name, target);
      if (problem) {
        setInvalid(target, problem);
        return;
      }
      clearInvalid(target);
      editor.draftChanged({ chips: name === "detail" });
      if (name === "enabled") {
        renderLive();
      }
    }

    function confirmRemoval(ci, condition) {
      const rules = rulesTriggeredBy(app.draft, condition.id).map((rule) => rule.name);
      const users = placeholderUsers(app.draft, condition.id);
      if (!rules.length && !users.length) {
        remove(condition);
        return;
      }
      const card = cardOf(ci);
      const parts = [];
      if (rules.length) {
        const count = rules.length === 1 ? "1 rule starts" : `${rules.length} rules start`;
        parts.push(`${count} when ${condition.name} turns on or off: ${joinWords(rules)}. ` +
          "Rules cannot stay without their condition, so they are removed too.");
      }
      if (users.length) {
        parts.push(`Text in ${joinWords(users)} uses {{${condition.id}}} and will show nothing there.`);
      }
      card.querySelector("[data-confirm-text]").textContent = parts.join(" ");
      card.querySelector("[data-confirm]").hidden = false;
      const button = card.querySelector('[data-condition-command="remove-confirmed"]');
      button.textContent = rules.length ? "Remove condition and its rules" : "Remove condition";
      button.focus();
    }

    function remove(condition) {
      const rules = removeCondition(app.draft, condition.id);
      editor.draftChanged({ conditions: true, rules: true, chips: true });
      addButton.focus();
      const also = rules.length ? ` and ${rules.length === 1 ? "1 rule" : `${rules.length} rules`}` : "";
      editor.announce(`Removed ${condition.name}${also}. Save to apply it, or discard changes to undo.`);
    }

    list.addEventListener("input", onInput);
    list.addEventListener("change", onInput);
    list.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && event.target.getAttribute("data-condition-field") === "prompt") {
        event.preventDefault();
      }
    });
    list.addEventListener("click", (event) => {
      const button = event.target.closest("[data-condition-command]");
      const ci = button ? Number(button.getAttribute("data-condition-index")) : -1;
      const condition = conditions()[ci];
      if (!condition) {
        return;
      }
      const command = button.getAttribute("data-condition-command");
      if (command === "remove") {
        confirmRemoval(ci, condition);
      } else if (command === "remove-confirmed") {
        remove(condition);
      } else if (command === "cancel-remove") {
        cardOf(ci).querySelector("[data-confirm]").hidden = true;
        focusKey(list, `condition:${ci}:remove`);
      }
    });

    addButton.addEventListener("click", () => {
      try {
        const condition = addCondition(app.draft);
        editor.draftChanged({ conditions: true, rules: true, chips: true });
        focusKey(list, `condition:${conditions().indexOf(condition)}:name`);
        byId(`condition-${conditions().indexOf(condition)}-name`).select();
        editor.announce("Added a condition. Name it and describe what the AI should watch for.");
      } catch (error) {
        editor.report(error);
      }
    });

    window.setInterval(renderLive, AGE_REFRESH_MS);

    return { render, renderLive };
  };
})();
