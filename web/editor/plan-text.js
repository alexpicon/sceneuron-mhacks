// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the assistant's edit plans in plain words, such as "Add rule Hype on arms up: when
   Arms up turns on, show Hype and shake it", so the streamer can review a plan before applying it. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { canonical } = inNode ? require("./project.js") : window.SceneuronEditor.project;
  const { triggerLabel } = inNode ? require("./triggers.js") : window.SceneuronEditor.triggers;

  const KIND_WORDS = { text: "text", box: "box", frame: "frame" };
  const FIELD_WORDS = {
    x: "position",
    y: "position",
    width: "size",
    height: "size",
    visible: "visibility",
    text: "text",
    font_size: "font size",
    bold: "bold",
    color: "text color",
    background: "color",
    border_color: "border color",
    border_width: "border width",
    corner_radius: "corner radius",
    opacity: "opacity",
    align: "alignment",
    idle: "idle animation",
    enter: "entrance animation",
  };
  const CONDITION_WORDS = {
    name: "name",
    prompt: "what to watch for",
    detail: "what to report",
    enabled: "on or off",
    min_confidence: "minimum confidence",
    confirm: "looks to turn on",
    release: "looks to turn off",
  };

  /** The name of the item with this id, looked up in each project in turn, or the id itself. */
  function nameIn(list, id, ...projects) {
    for (const project of projects) {
      const found = ((project && project[list]) || []).find((item) => item.id === id);
      if (found) {
        return found.name;
      }
    }
    return id;
  }

  /** "a", "a and b", or "a, b, and c". */
  function joinWords(words) {
    if (words.length <= 2) {
      return words.join(" and ");
    }
    return `${words.slice(0, -1).join(", ")}, and ${words[words.length - 1]}`;
  }

  /** Field names in plain words, without repeats, in the order given. */
  function fieldWords(fields, words) {
    return Array.from(new Set(fields.map((field) => words[field] || field.replace(/_/g, " "))));
  }

  function quote(text) {
    const flat = String(text).replace(/\s+/g, " ").trim();
    return `“${flat.length > 60 ? `${flat.slice(0, 57)}...` : flat}”`;
  }

  function lowerFirst(text) {
    return text.charAt(0).toLowerCase() + text.slice(1);
  }

  /** What a rule's actions do; an action on the same element as the one before says "it". */
  function describeActions(actions, ...projects) {
    let previous = null;
    const phrases = (actions || []).map((action) => {
      if (action.type === "flood") {
        return "flood the screen with emojis";
      }
      const subject = action.element === previous ? "it" : nameIn("elements", action.element, ...projects);
      previous = action.element;
      if (action.type === "effect") {
        return `${action.effect} ${subject}`;
      }
      const props = action.props || {};
      const parts = [];
      if (props.visible === true || props.visible === false) {
        parts.push(`${props.visible ? "show" : "hide"} ${subject}`);
      }
      const owner = parts.length ? "its" : subject === "it" ? "its" : `${subject}'s`;
      if (typeof props.text === "string") {
        parts.push(`set ${owner} text to ${quote(props.text)}`);
      }
      const others = Object.keys(props).filter((field) => field !== "visible" && field !== "text");
      if (others.length) {
        parts.push(`change ${parts.length ? "its" : owner} ${joinWords(fieldWords(others, FIELD_WORDS))}`);
      }
      return parts.length ? parts.join(" and ") : `leave ${subject} as it is`;
    });
    return phrases.length ? joinWords(phrases) : "do nothing";
  }

  function describeRule(rule, ...projects) {
    const conditions = projects.flatMap((project) => (project && project.conditions) || []);
    const when = lowerFirst(triggerLabel(rule.when, conditions));
    return `${rule.name}: ${when}, ${describeActions(rule.actions, ...projects)}`;
  }

  function changedFields(before, after, fields) {
    const differs = (field) => JSON.stringify(before[field]) !== JSON.stringify(after[field]);
    return before ? Object.keys(fields).filter(differs) : [];
  }

  /** The deck an element shows in the first project that has one, or undefined. */
  function deckShownBy(element, ...projects) {
    const shown = (project) => ((project && project.decks) || []).find((deck) => deck.element === element);
    return projects.map(shown).find(Boolean);
  }

  /** How long a deck shows a raised card: "until it is lowered", or "for 15 seconds at a time". */
  function stays(deck) {
    const ms = deck.auto_hide_ms || 0;
    const [amount, unit] = ms % 60000 === 0 ? [ms / 60000, "minute"] : [ms / 1000, "second"];
    return ms ? `for ${amount} ${unit}${amount === 1 ? "" : "s"} at a time` : "until it is lowered";
  }

  /** Card names, the first seven in full and the rest as a count when there are more than eight. */
  function cardNames(cards) {
    const names = cards.map((card) => card.name);
    return joinWords(names.length > 8 ? names.slice(0, 7).concat(`${names.length - 7} more`) : names);
  }

  /** What replacing a deck changes: its name, cards added, changed, or removed, auto-hide, and camera. */
  function deckChanges(old, deck) {
    const changes = old.name === deck.name ? [] : [`rename to ${deck.name}`];
    const [oldCards, cards] = [old.cards || [], deck.cards || []];
    const byId = (list, id) => list.find((card) => card.id === id);
    // The saved project may list a card's keys in another order than the plan, which changes nothing.
    const same = (card) => JSON.stringify(canonical(byId(oldCards, card.id))) === JSON.stringify(canonical(card));
    const groups = [
      ["add", cards.filter((card) => !byId(oldCards, card.id))],
      ["change", cards.filter((card) => byId(oldCards, card.id) && !same(card))],
      ["remove", oldCards.filter((card) => !byId(cards, card.id))],
    ];
    for (const [verb, group] of groups.filter(([, group]) => group.length)) {
      changes.push(`${verb} ${group.length === 1 ? "card" : "cards"} ${cardNames(group)}`);
    }
    if ((old.auto_hide_ms || 0) !== (deck.auto_hide_ms || 0)) {
      changes.push(`show each card ${stays(deck)}`);
    }
    if (Boolean(old.camera) !== Boolean(deck.camera)) {
      changes.push(`turn hand signals ${deck.camera ? "on" : "off"}`);
    }
    return changes;
  }

  /**
   * One operation of a plan as `{ kind, verb, text }`: kind is "add", "change", or "remove", and
   * `${verb} ${text}` reads as a sentence. `before` is the project the plan was made for and `after`
   * the project it produces.
   */
  function describeOperation(op, before, after) {
    const add = (text) => ({ kind: "add", verb: "Add", text });
    const change = (text) => ({ kind: "change", verb: "Change", text });
    const remove = (text) => ({ kind: "remove", verb: "Remove", text });
    switch (op.op) {
      case "add_element": {
        const { element } = op;
        const kind = KIND_WORDS[element.kind] || "element";
        const deck = deckShownBy(element.id, after);
        if (deck) {
          return add(`${kind} element ${element.name} to show the ${deck.name} cards`);
        }
        const text = element.kind === "text" ? `: ${quote(element.props.text || "")}` : "";
        const hidden = element.props.visible === false ? ", hidden until a rule shows it" : "";
        return add(`${kind} element ${element.name}${text}${hidden}`);
      }
      case "update_element": {
        const words = fieldWords(Object.keys(op.props || {}), FIELD_WORDS);
        const name = nameIn("elements", op.id, before, after);
        if (op.name && op.name !== name) {
          words.unshift(`rename to ${op.name}`);
        }
        const text = typeof (op.props || {}).text === "string" ? ` (text ${quote(op.props.text)})` : "";
        return change(`${name}: ${words.length ? words.join(", ") : "nothing"}${text}`);
      }
      case "remove_element": {
        const parts = [`element ${nameIn("elements", op.id, before)}`, "the rule actions that use it"];
        // A plan that moves the deck to another element keeps it, so only a deck the result lacks goes too.
        const shown = deckShownBy(op.id, before);
        const kept = shown && after && (after.decks || []).some((deck) => deck.id === shown.id);
        return remove(joinWords(shown && !kept ? parts.concat(`the ${shown.name} deck it shows`) : parts));
      }
      case "add_condition":
        return add(`condition ${op.condition.name}: ${op.condition.prompt.replace(/\.\s*$/, "")}`);
      case "update_condition": {
        const old = ((before && before.conditions) || []).find((condition) => condition.id === op.condition.id);
        const fields = fieldWords(changedFields(old, op.condition, CONDITION_WORDS), CONDITION_WORDS);
        const name = old ? old.name : op.condition.name;
        const prompt = old && old.prompt !== op.condition.prompt ? ` (now ${quote(op.condition.prompt)})` : "";
        return change(`condition ${name}: ${fields.length ? fields.join(", ") : "nothing"}${prompt}`);
      }
      case "remove_condition":
        return remove(`condition ${nameIn("conditions", op.id, before)} and the rules it starts`);
      case "add_rule":
        return add(`rule ${describeRule(op.rule, after, before)}`);
      case "update_rule":
        return change(`rule ${describeRule(op.rule, after, before)}`);
      case "remove_rule":
        return remove(`rule ${nameIn("rules", op.id, before)}`);
      case "add_deck": {
        const { deck } = op;
        const count = deck.cards.length === 1 ? "1 card" : `${deck.cards.length} cards`;
        const camera = deck.camera ? ", and hand signals can raise them" : "";
        const slot = nameIn("elements", deck.element, after, before);
        const cards = `${count} (${cardNames(deck.cards)}), each shown ${stays(deck)}${camera}`;
        return add(`deck ${deck.name} on ${slot}: ${cards}`);
      }
      case "update_deck": {
        const old = ((before && before.decks) || []).find((deck) => deck.id === op.deck.id);
        const changes = old ? deckChanges(old, op.deck) : [];
        return change(`deck ${old ? old.name : op.deck.name}: ${changes.length ? changes.join("; ") : "nothing"}`);
      }
      case "remove_deck": {
        const deck = ((before && before.decks) || []).find((item) => item.id === op.id);
        const keeps = deck && (!after || (after.elements || []).some((element) => element.id === deck.element));
        const slot = keeps ? `, keeping the element ${nameIn("elements", deck.element, before)}` : "";
        return remove(`deck ${nameIn("decks", op.id, before)} and its cards${slot}`);
      }
      default:
        return change(`something this editor cannot describe (${String(op.op)})`);
    }
  }

  /** Every operation of a plan in plain words. */
  function describePlan(plan, before) {
    return (plan.operations || []).map((op) => describeOperation(op, before, plan.project));
  }

  const planText = { describeActions, describeOperation, describePlan, joinWords };
  if (inNode) {
    module.exports = planText;
  } else {
    window.SceneuronEditor.planText = planText;
  }
})();
