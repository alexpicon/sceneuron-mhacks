// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: edits to the draft's AI conditions: ids made from names, adding, renaming, and
   removing conditions, and keeping the rules and texts that use them in step. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const constants = inNode ? require("./constants.js") : window.SceneuronEditor.constants;
  const triggers = inNode ? require("./triggers.js") : window.SceneuronEditor.triggers;
  const placeholders = inNode ? require("./placeholders.js") : window.SceneuronEditor.placeholders;
  const {
    CONDITION_DEFAULTS,
    MAX_CONDITIONS,
    MAX_DETAIL_CHARS,
    MAX_ID_CHARS,
    MAX_PROMPT_CHARS,
    ON_AIR_PLACEHOLDER,
    ON_AIR_TIME_PLACEHOLDER,
    ROOM_PLACEHOLDER,
    SCENE_PLACEHOLDER,
    SEEN_PLACEHOLDER,
    SUBTITLES_PLACEHOLDER,
  } = constants;
  const { rulesTriggeredBy, triggerCondition } = triggers;
  const { placeholderNames, renamePlaceholder } = placeholders;

  /** Lowercase letters, digits, and single hyphens from a name, at most MAX_ID_CHARS long. */
  function slug(name) {
    const base = String(name)
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return base.slice(0, MAX_ID_CHARS).replace(/-+$/, "");
  }

  /** A condition id made from the name: unique among `taken`, never a placeholder's name, at most 40 characters. */
  function conditionId(name, taken) {
    const used = new Set(taken || []);
    used.add(SCENE_PLACEHOLDER).add(SEEN_PLACEHOLDER).add(ROOM_PLACEHOLDER).add(SUBTITLES_PLACEHOLDER);
    used.add(ON_AIR_PLACEHOLDER).add(ON_AIR_TIME_PLACEHOLDER);
    const base = slug(name) || "condition";
    if (!used.has(base)) {
      return base;
    }
    for (let n = 2;; n += 1) {
      const suffix = `-${n}`;
      const id = base.slice(0, MAX_ID_CHARS - suffix.length).replace(/-+$/, "") + suffix;
      if (!used.has(id)) {
        return id;
      }
    }
  }

  /** Appends a new condition with the default thresholds and an empty prompt to fill in. */
  function addCondition(project, name) {
    project.conditions = project.conditions || [];
    if (project.conditions.length >= MAX_CONDITIONS) {
      throw new Error(`A project can have at most ${MAX_CONDITIONS} conditions.`);
    }
    const label = name || "New condition";
    const id = conditionId(label, project.conditions.map((condition) => condition.id));
    const condition = Object.assign({ id, name: label, prompt: "", detail: "" }, CONDITION_DEFAULTS);
    project.conditions.push(condition);
    return condition;
  }

  /** Points rule triggers and placeholders in element and rule texts from one condition id to another. */
  function renameConditionId(project, from, to) {
    for (const rule of project.rules || []) {
      if (triggerCondition(rule.when) === from) {
        rule.when = { condition: to, state: rule.when.state };
      }
      for (const action of rule.actions || []) {
        if (action.props && typeof action.props.text === "string") {
          action.props.text = renamePlaceholder(action.props.text, from, to);
        }
      }
    }
    for (const element of project.elements || []) {
      element.props.text = renamePlaceholder(element.props.text || "", from, to);
    }
  }

  /**
   * Renames a condition. Until it is saved (its id is not in `savedIds`) the id follows the name,
   * and every rule and text that uses the old id follows it too. Returns the condition's id.
   */
  function renameCondition(project, condition, name, savedIds) {
    condition.name = name;
    if ((savedIds || []).includes(condition.id)) {
      return condition.id;
    }
    const others = project.conditions.filter((other) => other !== condition).map((other) => other.id);
    const id = conditionId(name, others);
    if (id !== condition.id) {
      renameConditionId(project, condition.id, id);
      condition.id = id;
    }
    return id;
  }

  /** Removes the condition and every rule it starts, which could not stay valid without it. */
  function removeCondition(project, id) {
    const index = (project.conditions || []).findIndex((condition) => condition.id === id);
    if (index < 0) {
      return [];
    }
    project.conditions.splice(index, 1);
    const removed = rulesTriggeredBy(project, id);
    project.rules = project.rules.filter((rule) => !removed.includes(rule));
    return removed;
  }

  /** Names of the elements and rules whose text shows this condition's detail. */
  function placeholderUsers(project, id) {
    const uses = (text) => typeof text === "string" && placeholderNames(text).includes(id);
    const elements = (project.elements || []).filter((element) => uses(element.props.text)).map((e) => e.name);
    const rules = (project.rules || [])
      .filter((rule) => (rule.actions || []).some((action) => action.props && uses(action.props.text)))
      .map((rule) => rule.name);
    return elements.concat(rules);
  }

  /** One line of 1 to MAX_PROMPT_CHARS characters without control characters. */
  function isValidPrompt(text) {
    const chars = Array.from(String(text));
    return chars.length >= 1 && chars.length <= MAX_PROMPT_CHARS && !chars.some(isControl);
  }

  /** At most MAX_DETAIL_CHARS characters on one line; empty asks for no detail. */
  function isValidDetail(text) {
    const chars = Array.from(String(text));
    return chars.length <= MAX_DETAIL_CHARS && !chars.some(isControl);
  }

  function isControl(char) {
    const code = char.codePointAt(0);
    return code < 0x20 || (code >= 0x7f && code <= 0x9f);
  }

  /** Joins lines that a paste or the Enter key put into a one-line field. */
  function oneLine(text) {
    return String(text).replace(/\s*[\r\n]+\s*/g, " ");
  }

  const conditionEdits = {
    addCondition,
    conditionId,
    isValidDetail,
    isValidPrompt,
    oneLine,
    placeholderUsers,
    removeCondition,
    renameCondition,
    renameConditionId,
    slug,
  };
  if (inNode) {
    module.exports = conditionEdits;
  } else {
    window.SceneuronEditor.conditionEdits = conditionEdits;
  }
})();
