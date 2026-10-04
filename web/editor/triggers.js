// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: what starts a rule. A rule's `when` is "away", "present", or an object naming a
   condition and the state it turns to; select controls carry it as "away", "present", "on:<id>", or
   "off:<id>". */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const PRESENCE = { away: "When I leave", present: "When I come back" };

  /** The select value for a rule's `when`; an unreadable trigger gives "". */
  function encodeTrigger(when) {
    if (when === "away" || when === "present") {
      return when;
    }
    const valid = when && typeof when === "object" && typeof when.condition === "string" && when.condition;
    return valid && (when.state === "on" || when.state === "off") ? `${when.state}:${when.condition}` : "";
  }

  /** The `when` for a select value; throws on a value encodeTrigger never makes. */
  function decodeTrigger(value) {
    if (value === "away" || value === "present") {
      return value;
    }
    const match = /^(on|off):(.+)$/.exec(String(value));
    if (!match) {
      throw new Error(`Unknown trigger ${value}.`);
    }
    return { condition: match[2], state: match[1] };
  }

  /** The id of the condition a trigger watches, or null for presence triggers. */
  function triggerCondition(when) {
    return when && typeof when === "object" && typeof when.condition === "string" ? when.condition : null;
  }

  function conditionName(conditions, id) {
    const found = (conditions || []).find((condition) => condition.id === id);
    return found ? found.name : `a missing condition (${id})`;
  }

  /** Plain words for a trigger, such as "When I leave" or "When Waving turns on". */
  function triggerLabel(when, conditions) {
    if (when === "away" || when === "present") {
      return PRESENCE[when];
    }
    const id = triggerCondition(when);
    if (id === null) {
      return "When something unknown happens";
    }
    return `When ${conditionName(conditions, id)} turns ${when.state === "off" ? "off" : "on"}`;
  }

  /**
   * The choices for a rule's trigger select, grouped for optgroups: presence first, then each
   * condition turning on and off. A trigger on a condition the draft no longer has is offered too,
   * so the select can show it.
   */
  function triggerChoices(conditions, current) {
    const presence = Object.keys(PRESENCE).map((value) => [value, PRESENCE[value]]);
    const ai = [];
    for (const condition of conditions || []) {
      for (const state of ["on", "off"]) {
        ai.push([`${state}:${condition.id}`, `When ${condition.name} turns ${state}`]);
      }
    }
    const groups = [{ label: "Camera presence", choices: presence }];
    if (ai.length) {
      groups.push({ label: "AI conditions", choices: ai });
    }
    const value = encodeTrigger(current);
    const known = groups.some((group) => group.choices.some(([choice]) => choice === value));
    if (value && !known) {
      groups.push({ label: "Missing", choices: [[value, triggerLabel(current, conditions)]] });
    }
    return groups;
  }

  /** The rules of `project` that start when the condition with this id turns on or off. */
  function rulesTriggeredBy(project, id) {
    return ((project && project.rules) || []).filter((rule) => triggerCondition(rule.when) === id);
  }

  const triggers = { decodeTrigger, encodeTrigger, rulesTriggeredBy, triggerChoices, triggerCondition, triggerLabel };
  if (inNode) {
    module.exports = triggers;
  } else {
    window.SceneuronEditor.triggers = triggers;
  }
})();
