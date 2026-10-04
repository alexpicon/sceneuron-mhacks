// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of one rule card and its actions in the rules editor. Every control carries
   data attributes naming the rule, action, and field it edits, and a focus key that survives rebuilds. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { EFFECTS, MAX_ACTIONS, MAX_NAME_CHARS, MAX_TEXT_CHARS } = Editor.constants;
  const { PRESETS } = window.SceneuronReactionPresets;
  const { actionVisibility, findElement, otherActionFields } = Editor.project;
  const { encodeTrigger, triggerChoices } = Editor.triggers;
  const { check, field, h, hidden, option } = Editor.dom;

  function elementOptions(draft, selected) {
    const options = draft.elements.map((element) => option(element.id, `${element.name} (${element.id})`));
    if (!findElement(draft, selected)) {
      options.unshift(option(selected, `Missing element (${selected})`));
    }
    return options;
  }

  function actionItem(draft, rule, ri, action, ai) {
    const base = `rule-${ri}-action-${ai}`;
    const data = (name, extra) =>
      Object.assign(
        {
          id: `${base}-${name}`,
          "data-rule-index": ri,
          "data-action-index": ai,
          "data-action-field": name,
          "data-focus-key": `rule:${rule.id}:action:${ai}:${name}`,
        },
        extra || {},
      );
    const typeSelect = h(
      "select",
      data("type", { value: action.type }),
      option("set_props", "Show, hide, or set text"),
      option("effect", "Play effect"),
      option("flood", "Play a reaction"),
    );
    const parts = [field(`${base}-type`, "Action", typeSelect)];
    if (action.type !== "flood") {
      const elementSelect = h(
        "select",
        data("element", { value: action.element }),
        ...elementOptions(draft, action.element),
      );
      parts.push(field(`${base}-element`, "Element", elementSelect));
    }
    if (action.type === "flood") {
      const floods = PRESETS.map((preset) => option(preset.id, `${preset.emojis[0]} ${preset.name}`));
      const floodSelect = h("select", data("flood", { value: action.flood }), ...floods);
      const cooldown =
        "Floods the whole overlay with emojis for a few seconds, its line in the middle if it has one, at most " +
        "once per fun reactions cooldown.";
      parts.push(field(`${base}-flood`, "Reaction", floodSelect), h("p", { class: "hint" }, cooldown));
    } else if (action.type === "effect") {
      const effects = EFFECTS.map((effect) => option(effect, effect.charAt(0).toUpperCase() + effect.slice(1)));
      const effectSelect = h("select", data("effect", { value: action.effect }), ...effects);
      parts.push(field(`${base}-effect`, "Effect", effectSelect));
    } else {
      const visibility = h(
        "select",
        data("visibility", { value: actionVisibility(action) }),
        option("keep", "No change"),
        option("show", "Show"),
        option("hide", "Hide"),
      );
      const target = findElement(draft, action.element);
      const hasText = Boolean(action.props) && "text" in action.props;
      const textAllowed = hasText || (target && target.kind === "text");
      const setText = h("input", data("set-text", { type: "checkbox", checked: hasText, disabled: !textAllowed }));
      const text = h(
        "input",
        data("text", {
          type: "text",
          maxlength: MAX_TEXT_CHARS,
          value: hasText ? action.props.text : "",
          disabled: !hasText,
        }),
      );
      parts.push(field(`${base}-visibility`, "Visibility", visibility));
      parts.push(check(`${base}-set-text`, "Change text", setText), field(`${base}-text`, "New text", text));
      const others = otherActionFields(action);
      if (others.length) {
        parts.push(h("p", { class: "hint" }, `Also sets ${others.join(", ")} (kept as saved).`));
      }
    }
    const remove = h(
      "button",
      {
        type: "button",
        class: "danger",
        "data-rule-command": "delete-action",
        "data-rule-index": ri,
        "data-action-index": ai,
        "data-focus-key": `rule:${rule.id}:action:${ai}:delete`,
      },
      "Remove action",
      hidden(` ${ai + 1} of rule ${ri + 1}`),
    );
    parts.push(remove);
    return h("li", { class: "rule-action", "aria-label": `Action ${ai + 1}` }, ...parts);
  }

  /** The card for the rule at index `ri` of the draft's rules. */
  function ruleCard(draft, rule, ri) {
    const base = `rule-${ri}`;
    const data = (name, extra) =>
      Object.assign(
        {
          id: `${base}-${name}`,
          "data-rule-index": ri,
          "data-rule-field": name,
          "data-focus-key": `rule:${rule.id}:${name}`,
        },
        extra || {},
      );
    const command = (name, label, extra) =>
      h(
        "button",
        Object.assign(
          {
            type: "button",
            "data-rule-command": name,
            "data-rule-index": ri,
            "data-focus-key": `rule:${rule.id}:${name}`,
          },
          extra || {},
        ),
        label,
        hidden(` to rule ${ri + 1}`),
      );
    const groups = triggerChoices(draft.conditions, rule.when).map((group) =>
      h("optgroup", { label: group.label }, ...group.choices.map(([value, label]) => option(value, label)))
    );
    const when = h("select", data("when", { value: encodeTrigger(rule.when) }), ...groups);
    const actions = h(
      "ol",
      { class: "rule-actions", "aria-label": `Actions of rule ${ri + 1}` },
      ...rule.actions.map((action, ai) => actionItem(draft, rule, ri, action, ai)),
    );
    const addActionButton = command("add-action", "Add action", {
      disabled: rule.actions.length >= MAX_ACTIONS || draft.elements.length === 0,
    });
    const nameInput = h("input", data("name", { type: "text", maxlength: MAX_NAME_CHARS, value: rule.name }));
    const enabledInput = h("input", data("enabled", { type: "checkbox", checked: rule.enabled !== false }));
    const remove = h(
      "button",
      { type: "button", class: "danger", "data-rule-command": "delete-rule", "data-rule-index": ri },
      "Delete rule",
      hidden(` ${ri + 1}`),
    );
    return h(
      "article",
      { class: "rule", "aria-labelledby": `${base}-heading` },
      h("h3", { id: `${base}-heading` }, `Rule ${ri + 1}`),
      field(`${base}-name`, "Name", nameInput),
      field(`${base}-when`, "Trigger", when),
      check(`${base}-enabled`, "Enabled", enabledInput),
      actions,
      h("div", { class: "button-row" }, addActionButton, remove),
    );
  }

  Editor.ruleCards = { ruleCard };
})();
