// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: hand signals in the Cards tab: the choices of a card's Hand signal list, and what a signal shares
   with other cards, as the camera matches them. A deck raises only the first of its cards whose signal shows, so the
   editor refuses a signal that another card of the same deck has; every deck whose hand signals are on raises a card,
   so the same signal there is a warning; and a pose that shows two signals at once gets a note: a peace sign also
   counts as two fingers, and two hands show two signs, or a sign and the other hand's fingers. Where a vote decides
   which deck or card such a pose raises (signals-context.js), the warnings and notes say so. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { signalLabel } = inNode ? require("./cards-text.js") : window.SceneuronEditor.cardsText;
  const { byGate, byOrder, decides, gates, listed } = inNode
    ? require("./signals-context.js")
    : window.SceneuronEditor.signalsContext;

  /** The most fingers a card can ask for: two hands, as the server allows. */
  const MAX_FINGERS = 10;
  /** The signs the camera recognizes: the list value's name, the list label, and the words in a sentence. */
  const SIGNS = [
    ["thumbs_up", "Thumbs up", "a thumbs up"],
    ["thumbs_down", "Thumbs down", "a thumbs down"],
    ["open_palm", "Open palm", "an open palm"],
    ["peace", "Peace sign", "a peace sign"],
    ["pointing", "Pointing", "a pointing hand"],
    ["fist", "Fist", "a fist"],
  ];
  /**
   * The finger counts one hand showing a sign gives, the usual way first. The camera counts the extended fingers plus
   * the thumb when it is out. A thumbs up or down gives no count, and its hand is not counted as raised.
   */
  const SIGN_COUNTS = {
    fist: [[0, ""]],
    pointing: [[1, ""], [2, " with the thumb out"]],
    peace: [[2, ""], [3, " with the thumb out"]],
    open_palm: [[5, ""], [4, " with the thumb folded"]],
  };
  const THUMBS = ["thumbs_up", "thumbs_down"];
  /**
   * The most fingers one hand shows. The camera looks at two hands at most: it adds up their fingers, leaving out a
   * thumbs up or down, and sees each hand's sign.
   */
  const HAND_FINGERS = 5;
  const HELD = "A hand held up counts as a raised hand unless it shows a thumbs up or down";
  const THUMBS_HELD = "A thumbs up or down counts as a raised hand only with the arm up, the wrist above the nose, " +
    "or with the other hand held up";
  const SIGN_AND_COUNT =
    "The camera adds up both hands: a sign on one hand and fingers on the other can make the count";
  const THUMBS_AND_COUNT = "A thumbs up or down on one hand adds no fingers while the other hand shows the count";
  const TWO_SIGNS = "Two hands can show two signs at once";
  const KIND_ORDER = { hand_raised: 0, sign: 1, fingers: 2 };

  const capital = (text) => text.charAt(0).toUpperCase() + text.slice(1);
  const fingers = (count) => `${count} ${count === 1 ? "finger" : "fingers"}`;
  const words = (sign) => (SIGNS.find(([name]) => name === sign) || [])[2] || null;
  /** A signal in a sentence, such as "a peace sign" or "2 fingers up". */
  const named = (signal) => (kindOf(signal) === "sign" && words(signal.sign)) || signalLabel(signal);

  function kindOf(signal) {
    if (signal === "hand_raised") {
      return "hand_raised";
    }
    if (signal && Number.isInteger(signal.fingers)) {
      return "fingers";
    }
    return signal && typeof signal.sign === "string" ? "sign" : null;
  }

  /** The list value for a signal: "" for none, "hand_raised", "fingers:3", or "sign:peace". */
  function signalValue(signal) {
    const kind = kindOf(signal);
    if (kind === "fingers") {
      return `fingers:${signal.fingers}`;
    }
    return kind === "sign" ? `sign:${signal.sign}` : kind || "";
  }

  /** The signal a list value stands for: null for none, and undefined for a value the list never offers. */
  function signalFrom(value) {
    if (value === "" || value === "hand_raised") {
      return value === "" ? null : value;
    }
    const count = /^fingers:(\d{1,2})$/.exec(value);
    if (count) {
      return Number(count[1]) <= MAX_FINGERS ? { fingers: Number(count[1]) } : undefined;
    }
    const sign = /^sign:([a-z_]+)$/.exec(value);
    return sign ? { sign: sign[1] } : undefined;
  }

  /**
   * `[value, label]` choices: None, a raised hand, 0 to 10 fingers, then the signs. A signal the list does not offer,
   * such as a sign from a newer Sceneuron, is added at the end, so it shows and stays.
   */
  function signalChoices(current) {
    const choices = [["", "None"], ["hand_raised", "Raised hand"]];
    for (let count = 0; count <= MAX_FINGERS; count += 1) {
      choices.push([`fingers:${count}`, fingers(count)]);
    }
    choices.push(...SIGNS.map(([name, label]) => [`sign:${name}`, label]));
    const value = signalValue(current);
    if (!choices.some(([offered]) => offered === value)) {
      choices.push([value, capital(signalLabel(current))]);
    }
    return choices;
  }

  /**
   * How one pose shows a sign and a finger count. One hand gives the sign's own count. With two hands the other hand
   * adds 0 to 5 fingers to it, or gives the whole count beside a thumbs up or down.
   */
  function signAndCount(sign, count) {
    if (THUMBS.includes(sign)) {
      return count <= HAND_FINGERS ? THUMBS_AND_COUNT : null;
    }
    const counts = SIGN_COUNTS[sign] || [];
    const one = counts.find(([given]) => given === count);
    if (one) {
      return `${capital(words(sign))}${one[1]} also counts as ${fingers(count)}`;
    }
    return counts.some(([given]) => count >= given && count - given <= HAND_FINGERS) ? SIGN_AND_COUNT : null;
  }

  /**
   * How one pose shows both of two different signals, as a sentence, or null when none does. A raised hand is any
   * hand held up, except a thumbs up or down, or an arm up; two different signs show on two hands, and two finger
   * counts never show together.
   */
  function sharedPose(a, b) {
    const [first, second] = [a, b].sort((x, y) => KIND_ORDER[kindOf(x)] - KIND_ORDER[kindOf(y)]);
    const kinds = `${kindOf(first)} ${kindOf(second)}`;
    if (kinds === "hand_raised fingers") {
      return HELD;
    }
    if (kinds === "hand_raised sign" && words(second.sign)) {
      return THUMBS.includes(second.sign) ? THUMBS_HELD : HELD;
    }
    if (kinds === "sign sign") {
      return words(first.sign) && words(second.sign) && first.sign !== second.sign ? TWO_SIGNS : null;
    }
    return kinds === "sign fingers" ? signAndCount(first.sign, second.fingers) : null;
  }

  /** What a group of cards has, such as "a fist and a peace sign", "2 fingers up", or "0, 1, and 2 fingers up". */
  function signalsOf(cards) {
    const signals = cards.map((card) => card.signal);
    return signals.length > 1 && signals.every((signal) => kindOf(signal) === "fingers")
      ? `${listed(signals.map((signal) => String(signal.fingers)))} fingers up`
      : listed(signals.map(named));
  }

  /**
   * Which card a deck raises when one pose shows both the signal of card `name`, at `at` in the deck's `cards`, and
   * that of a card in `group`, the deck's other cards that pose shows.
   */
  function raisedFirst(cards, at, name, group) {
    const before = group.filter((card) => cards.indexOf(card) < at);
    if (before.length === 0 || group.length === 1) {
      return `only ${before.length ? group[0].name : name} then, since it comes first`;
    }
    return before.length === group.length
      ? `one of them then, not ${name}, since they come first`
      : "only whichever of the two comes first";
  }

  /** The cards whose signal one pose shows together with `signal`, by that pose's sentence, in their deck's order. */
  function sharedGroups(signal, cards) {
    const groups = new Map();
    for (const card of cards.filter((other) => other.signal && signalValue(other.signal) !== signalValue(signal))) {
      const pose = sharedPose(signal, card.signal);
      if (pose) {
        groups.set(pose, (groups.get(pose) || []).concat(card));
      }
    }
    return groups;
  }

  /**
   * What `signal`, on card `cardId` of deck `deckId`, shares with the project's other cards. `refusal` names a card of
   * the same deck with the same signal; `warnings` name cards with the same signal in other decks whose hand signals
   * are on; `notes` describe poses that show this signal and other cards', in this deck or in those decks, a note for
   * each deck, pose, and context that decides whether the camera raises those cards.
   */
  function signalConflicts(project, deckId, cardId, signal) {
    const found = { refusal: null, warnings: [], notes: [] };
    const decks = (project && project.decks) || [];
    const deck = decks.find((candidate) => candidate.id === deckId);
    if (!signal || !deck) {
      return found;
    }
    const value = signalValue(signal);
    const at = deck.cards.findIndex((card) => card.id === cardId);
    const name = at >= 0 ? deck.cards[at].name : cardId;
    const others = deck.cards.filter((_, index) => index !== at);
    const twin = others.find((other) => other.signal && signalValue(other.signal) === value);
    if (twin) {
      found.refusal =
        `${twin.name} already uses ${named(signal)} in this deck. Choose another hand signal for ${name}, or None.`;
    }
    const has = (group) => `${listed(group.map((card) => card.name))} ${group.length === 1 ? "has" : "have"} ` +
      signalsOf(group);
    const called = (group) => (group.length === 1 ? group[0].name : "one of them");
    const gate = gates(project, deckId, cardId, signal);
    const mine = gate(deckId, signal);
    const ahead = (card) => deck.cards.indexOf(card) < at;
    for (const [pose, shared] of sharedGroups(signal, others)) {
      for (const [group, but] of byOrder(shared, mine, (card) => gate(deckId, card.signal), ahead)) {
        const first = raisedFirst(deck.cards, at, name, group);
        found.notes.push(`${has(group)}. ${pose}, so this deck raises ${first}${but(called(group))}.`);
      }
    }
    const once = deck.camera ? "" : " once this deck's hand signals are on too";
    // What the context decides for this card and `them`, gated by `theirs`, or else that `raised` would be raised.
    const so = (theirs, them, raised) => {
      const decided = decides(mine, theirs, them);
      return decided ? `${once ? `${once.trim()}, ` : ""}${decided}` : `${raised} would be raised${once}`;
    };
    for (const listening of decks.filter((other) => other !== deck && other.camera)) {
      for (const other of listening.cards.filter((card) => card.signal && signalValue(card.signal) === value)) {
        found.warnings.push(`${listening.name}: ${other.name} also uses ${named(signal)}, and its deck's hand ` +
          `signals are on, so ${so(gate(listening.id, other.signal), other.name, "both cards")}.`);
      }
      for (const [pose, shared] of sharedGroups(signal, listening.cards)) {
        for (const [theirs, group] of byGate(shared, (card) => gate(listening.id, card.signal))) {
          const raised = group.length === 1 ? "both" : "this card and one of them";
          found.notes.push(`${listening.name}: ${has(group)}. ${pose}, so ${so(theirs, called(group), raised)}.`);
        }
      }
    }
    if (value === "sign:open_palm") {
      found.notes.push("Hands often open while talking, so an open palm can raise this card by accident.");
    }
    return found;
  }

  /**
   * Reads a card's Hand signal choice: `{ value }`, the signal or null for none, or `{ problem }` when the editor
   * refuses it.
   */
  function readSignal(project, deckId, cardId, raw) {
    const signal = signalFrom(raw);
    if (signal === undefined) {
      return { problem: "Choose a hand signal from the list." };
    }
    const { refusal } = signalConflicts(project, deckId, cardId, signal);
    return refusal ? { problem: refusal } : { value: signal };
  }

  const signalsText = { readSignal, sharedPose, signalChoices, signalConflicts, signalFrom, signalValue };
  if (inNode) {
    module.exports = signalsText;
  } else {
    window.SceneuronEditor.signalsText = signalsText;
  }
})();
