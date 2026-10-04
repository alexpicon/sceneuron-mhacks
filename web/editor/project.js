// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: edits to the draft project (elements, rules, and rule actions) and change tracking. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const Render = inNode ? require("../render-style.js") : window.SceneuronRender;
  const constants = inNode ? require("./constants.js") : window.SceneuronEditor.constants;
  const { KINDS, KIND_DEFAULTS, KIND_NAMES, MAX_ACTIONS, MAX_ELEMENTS, MAX_RULES, NUMBER_LIMITS } = constants;

  function clone(value) {
    return JSON.parse(JSON.stringify(value));
  }

  /** A copy of `value` whose object keys are sorted, so stringifying it ignores key order. */
  function canonical(value) {
    if (Array.isArray(value)) {
      return value.map(canonical);
    }
    if (value && typeof value === "object") {
      const sorted = {};
      for (const key of Object.keys(value).sort()) {
        sorted[key] = canonical(value[key]);
      }
      return sorted;
    }
    return value;
  }

  /** True when the draft differs from the saved project; automation mode is live state, not a draft edit. */
  function isDirty(saved, draft) {
    const comparable = (project) => {
      if (!project) {
        return null;
      }
      const copy = Object.assign({}, project);
      delete copy.automation_enabled;
      return JSON.stringify(canonical(copy));
    };
    return comparable(saved) !== comparable(draft);
  }

  /** The PUT body: the draft with the server's current automation mode, so saving never undoes a pause. */
  function projectForSave(draft, live) {
    const project = clone(draft);
    if (live && typeof live.automation_enabled === "boolean") {
      project.automation_enabled = live.automation_enabled;
    }
    return project;
  }

  function uniqueId(prefix, taken) {
    for (let n = 1;; n += 1) {
      const id = `${prefix}-${n}`;
      if (!taken.has(id)) {
        return id;
      }
    }
  }

  function findElement(project, id) {
    return (project && project.elements.find((element) => element.id === id)) || null;
  }

  /**
   * Appends a new element centered on the canvas. `reservedIds` (such as ids still live on the
   * server) are skipped so a new element never inherits another element's held values.
   */
  function addElement(project, kind, reservedIds) {
    if (!KINDS.includes(kind)) {
      throw new Error(`Unknown element kind ${kind}.`);
    }
    if (project.elements.length >= MAX_ELEMENTS) {
      throw new Error(`A project can have at most ${MAX_ELEMENTS} elements.`);
    }
    const taken = new Set(project.elements.map((element) => element.id).concat(reservedIds || []));
    const id = uniqueId(kind, taken);
    const props = Object.assign({}, Render.DEFAULT_PROPS, KIND_DEFAULTS[kind]);
    props.x = Math.round((project.canvas.width - props.width) / 2);
    props.y = Math.round((project.canvas.height - props.height) / 2);
    const element = { id, name: `${KIND_NAMES[kind]} ${id.slice(kind.length + 1)}`, kind, props };
    project.elements.push(element);
    return element;
  }

  /** Removes an element, every rule action that targets it, and the deck it shows, so the project stays valid. */
  function deleteElement(project, id) {
    const index = project.elements.findIndex((element) => element.id === id);
    if (index < 0) {
      return false;
    }
    project.elements.splice(index, 1);
    for (const rule of project.rules) {
      rule.actions = rule.actions.filter((action) => action.element !== id);
    }
    if (Array.isArray(project.decks)) {
      project.decks = project.decks.filter((deck) => deck.element !== id);
    }
    return true;
  }

  function moveElement(project, id, x, y) {
    const element = findElement(project, id);
    if (!element) {
      return false;
    }
    const [min, max] = NUMBER_LIMITS.x;
    const clamp = (value) => Math.min(max, Math.max(min, Math.round(value)));
    const next = { x: clamp(x), y: clamp(y) };
    if (element.props.x === next.x && element.props.y === next.y) {
      return false;
    }
    element.props.x = next.x;
    element.props.y = next.y;
    return true;
  }

  /** A new rule action of `type`; a flood covers the whole overlay, so it ignores `element`. */
  function makeAction(type, element) {
    if (type === "flood") {
      return { type, flood: "tongue" };
    }
    if (type === "set_props") {
      return { type, element, props: { visible: true } };
    }
    if (type === "effect") {
      return { type, element, effect: "flash" };
    }
    throw new Error(`Unknown action type ${type}.`);
  }

  function addRule(project) {
    if (project.rules.length >= MAX_RULES) {
      throw new Error(`A project can have at most ${MAX_RULES} rules.`);
    }
    const id = uniqueId("rule", new Set(project.rules.map((rule) => rule.id)));
    const first = project.elements[0];
    const actions = first ? [makeAction("set_props", first.id)] : [];
    const rule = { id, name: "New rule", enabled: true, when: "away", actions };
    project.rules.push(rule);
    return rule;
  }

  function deleteRule(project, id) {
    const index = project.rules.findIndex((rule) => rule.id === id);
    if (index >= 0) {
      project.rules.splice(index, 1);
    }
    return index >= 0;
  }

  function addAction(rule, elementId) {
    if (rule.actions.length >= MAX_ACTIONS) {
      throw new Error(`A rule can have at most ${MAX_ACTIONS} actions.`);
    }
    const action = makeAction("set_props", elementId);
    rule.actions.push(action);
    return action;
  }

  function actionVisibility(action) {
    const visible = action.props ? action.props.visible : undefined;
    return visible === true ? "show" : visible === false ? "hide" : "keep";
  }

  function setActionVisibility(action, choice) {
    action.props = action.props || {};
    if (choice === "show" || choice === "hide") {
      action.props.visible = choice === "show";
    } else {
      delete action.props.visible;
    }
  }

  function setActionText(action, text) {
    action.props = action.props || {};
    if (text === null || text === undefined) {
      delete action.props.text;
    } else {
      action.props.text = String(text);
    }
  }

  /** Fields a set_props action changes beyond the visibility and text this editor exposes. */
  function otherActionFields(action) {
    return Object.keys(action.props || {}).filter((field) => field !== "visible" && field !== "text");
  }

  const project = {
    actionVisibility,
    addAction,
    addElement,
    addRule,
    canonical,
    clone,
    deleteElement,
    deleteRule,
    findElement,
    isDirty,
    makeAction,
    moveElement,
    otherActionFields,
    projectForSave,
    setActionText,
    setActionVisibility,
    uniqueId,
  };
  if (inNode) {
    module.exports = project;
  } else {
    window.SceneuronEditor.project = project;
  }
})();
