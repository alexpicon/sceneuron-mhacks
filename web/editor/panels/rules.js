// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the rules editor: rendering the rule cards and applying their edits and commands to
   the draft. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_NAME_CHARS, MAX_RULES, MAX_TEXT_CHARS } = Editor.constants;
  const { addAction, addRule, deleteRule, makeAction, setActionText, setActionVisibility } = Editor.project;
  const { decodeTrigger } = Editor.triggers;
  const { isValidName, isValidText } = Editor.values;
  const { clearInvalid, focusKey, setInvalid, withFocusRestore } = Editor.dom;
  const { ruleCard } = Editor.ruleCards;

  /** Wires the rules editor; returns the view with render. */
  Editor.rules = function setupRules(editor) {
    const { app, ui } = editor;

    function render() {
      const rules = app.draft ? app.draft.rules : [];
      withFocusRestore(ui.rulesList, () => {
        ui.rulesList.replaceChildren(...rules.map((rule, ri) => ruleCard(app.draft, rule, ri)));
      });
      ui.rulesEmpty.hidden = rules.length > 0;
      ui.addRule.disabled = !app.draft || rules.length >= MAX_RULES;
    }

    function onRuleInput(event) {
      const target = event.target;
      const isToggle = target.tagName === "SELECT" || target.type === "checkbox";
      if ((event.type === "input") === isToggle) {
        return;
      }
      const rule = app.draft && app.draft.rules[Number(target.getAttribute("data-rule-index"))];
      if (!rule) {
        return;
      }
      const ruleField = target.getAttribute("data-rule-field");
      if (ruleField === "name") {
        if (!isValidName(target.value)) {
          setInvalid(target, `Enter 1 to ${MAX_NAME_CHARS} characters.`);
          return;
        }
        clearInvalid(target);
        rule.name = target.value;
      } else if (ruleField === "enabled") {
        rule.enabled = target.checked;
      } else if (ruleField === "when") {
        rule.when = decodeTrigger(target.value);
      }
      if (ruleField) {
        editor.draftChanged({});
        return;
      }
      const ai = Number(target.getAttribute("data-action-index"));
      const action = rule.actions[ai];
      if (!action) {
        return;
      }
      let rebuild = false;
      switch (target.getAttribute("data-action-field")) {
        case "type": {
          const element = action.element || (app.draft.elements[0] && app.draft.elements[0].id);
          if (target.value === "flood" || element) {
            rule.actions[ai] = makeAction(target.value, element);
          }
          rebuild = true;
          break;
        }
        case "flood":
          action.flood = target.value;
          break;
        case "element":
          action.element = target.value;
          rebuild = true;
          break;
        case "visibility":
          setActionVisibility(action, target.value);
          break;
        case "set-text":
          setActionText(action, target.checked ? "" : null);
          rebuild = true;
          break;
        case "text":
          if (!isValidText(target.value)) {
            setInvalid(target, `Use at most ${MAX_TEXT_CHARS} characters.`);
            return;
          }
          clearInvalid(target);
          setActionText(action, target.value);
          break;
        case "effect":
          action.effect = target.value;
          break;
        default:
          return;
      }
      editor.draftChanged({ rules: rebuild });
    }

    ui.rulesList.addEventListener("input", onRuleInput);
    ui.rulesList.addEventListener("change", onRuleInput);

    ui.rulesList.addEventListener("click", (event) => {
      const button = event.target.closest("[data-rule-command]");
      const rule = button && app.draft.rules[Number(button.getAttribute("data-rule-index"))];
      if (!rule) {
        return;
      }
      const command = button.getAttribute("data-rule-command");
      if (command === "add-action") {
        try {
          addAction(rule, app.draft.elements[0].id);
        } catch (error) {
          editor.report(error);
          return;
        }
        editor.draftChanged({ rules: true });
        focusKey(ui.rulesList, `rule:${rule.id}:action:${rule.actions.length - 1}:type`);
      } else if (command === "delete-action") {
        rule.actions.splice(Number(button.getAttribute("data-action-index")), 1);
        editor.draftChanged({ rules: true });
        focusKey(ui.rulesList, `rule:${rule.id}:add-action`);
      } else if (command === "delete-rule") {
        deleteRule(app.draft, rule.id);
        editor.draftChanged({ rules: true });
        ui.addRule.focus();
        editor.announce(`Deleted rule ${rule.name}.`);
      }
    });

    ui.addRule.addEventListener("click", () => {
      try {
        const rule = addRule(app.draft);
        editor.draftChanged({ rules: true });
        focusKey(ui.rulesList, `rule:${rule.id}:name`);
      } catch (error) {
        editor.report(error);
      }
    });

    return { render };
  };
})();
