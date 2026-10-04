// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron line icons, one source for the overlay's themes and the Stream Deck plugin's keys: each icon is drawn
   in a 24 by 24 box with round, unfilled strokes STROKE_WIDTH wide, in whatever color its user strokes it. It also
   says which icon each card of the built-in packs shows, with the short label a key gives it, and which icon a few
   starter elements show; a custom card shows its leading emoji, or a fallback icon. */
(function () {
  "use strict";

  const STROKE_WIDTH = 1.75;

  // Shapes in a short form: "c cx cy r" a circle, "r x y width height radius" a rectangle, and "p d" a path.
  const DRAWINGS = {
    timer: ["c 12 13.5 7.5", "p M10 2.5h4M12 2.5V6M12 13.5l3-3.5M18.2 7.3l1.4-1.4"],
    "off-track": [
      "c 5.5 18.5 2.5",
      "c 18.5 5.5 2.5",
      "p M8 18.5h6.5a3.25 3.25 0 0 0 0-6.5h-5a3.25 3.25 0 0 1 0-6.5H16",
      "p M3 3l18 18",
    ],
    question: ["c 12 12 9", "p M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 16.8h.01"],
    "thumbs-up": [
      "r 3 10.5 4 9.5 1",
      "p M7 11.5l3.4-7a1.9 1.9 0 0 1 3.6 1V10h4.6a2 2 0 0 1 2 2.3l-1.2 6.2a2 2 0 0 1-2 1.5H7",
    ],
    "thumbs-down": [
      "r 3 4 4 9.5 1",
      "p M7 12.5l3.4 7a1.9 1.9 0 0 0 3.6-1V14h4.6a2 2 0 0 0 2-2.3l-1.2-6.2a2 2 0 0 0-2-1.5H7",
    ],
    "volume-off": ["p M3.5 9.5H7l5-4v13l-5-4H3.5z", "p M16 9.5l5 5M21 9.5l-5 5"],
    chat: [
      "p M4 4.5h10a1.5 1.5 0 0 1 1.5 1.5v6a1.5 1.5 0 0 1-1.5 1.5H8.5L5 16.5v-3H4" +
        "A1.5 1.5 0 0 1 2.5 12V6A1.5 1.5 0 0 1 4 4.5z",
      "p M18.5 9H20a1.5 1.5 0 0 1 1.5 1.5v6A1.5 1.5 0 0 1 20 18h-1v3l-3.5-3h-5A1.5 1.5 0 0 1 9 16.5V16",
    ],
    skip: ["p M5 5.5v13l9.5-6.5z", "p M18.5 5.5v13"],
    hand: [
      "p M9 15V5.5a1.5 1.5 0 0 1 3 0V11M12 10.5V3.5a1.5 1.5 0 0 1 3 0V11",
      "p M15 11V5.5a1.5 1.5 0 0 1 3 0V14a7 7 0 0 1-7 7h-.5a6.5 6.5 0 0 1-5.4-2.9L3.6 13.6a1.6 1.6 0 0 1 2.6-1.8L9 15",
    ],
    repeat: ["p M17 2.5l3 3-3 3M4 11.5v-1a5 5 0 0 1 5-5h11", "p M7 21.5l-3-3 3-3M20 12.5v1a5 5 0 0 1-5 5H4"],
    gauge: ["p M4.5 16a7.5 7.5 0 0 1 15 0M12 16l-3.5-3.5M3 19.5h18"],
    "check-circle": ["c 12 12 9", "p M8 12.5l2.8 2.8L16.2 9"],
    check: ["p M5 12.5l4.5 4.5L19 7.5"],
    mic: ["p M9 5.5a3 3 0 0 1 6 0v5a3 3 0 0 1-6 0z", "p M5.5 10.5a6.5 6.5 0 0 0 13 0M12 17v4M9 21h6"],
    "mic-off": [
      "p M9 5.5a3 3 0 0 1 6 0v5a3 3 0 0 1-6 0z",
      "p M5.5 10.5a6.5 6.5 0 0 0 13 0M12 17v4M9 21h6",
      "p M3.5 3.5l17 17",
    ],
    coffee: [
      "p M5 9h11v5a5 5 0 0 1-5 5h-1a5 5 0 0 1-5-5z",
      "p M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16M8 2.5V6M11 2.5V6M14 2.5V6",
    ],
    utensils: ["p M5 2.5V8a2.5 2.5 0 0 0 5 0V2.5M7.5 2.5v19", "p M18 21.5v-19c-2.2.8-3.5 3.3-3.5 6.5v5H18"],
    headphones: ["p M4 16v-4a8 8 0 0 1 16 0v4", "r 3.5 14 4 6.5 1.5", "r 16.5 14 4 6.5 1.5"],
    monitor: ["r 2.5 4 19 12.5 1.5", "p M8.5 20.5h7M12 16.5v4"],
    phone: [
      "p M8.6 3.5H6A2 2 0 0 0 4 5.6 15 15 0 0 0 18.4 20a2 2 0 0 0 2.1-2v-2.6a1 1 0 0 0-.7-.9l-3.2-1a1 1 0 0 0-1 .3" +
        "l-1.4 1.6a12 12 0 0 1-5.6-5.6l1.6-1.4a1 1 0 0 0 .3-1l-1-3.2a1 1 0 0 0-.9-.7z",
    ],
    person: ["c 12 8 4", "p M4.5 20.5a7.5 7.5 0 0 1 15 0"],
    box: ["p M12 2.5l8.5 4.75v9.5L12 21.5l-8.5-4.75v-9.5z", "p M3.5 7.25L12 12l8.5-4.75M12 12v9.5"],
    paw: [
      "c 5.5 11 1.7",
      "c 9.5 6.5 1.7",
      "c 14.5 6.5 1.7",
      "c 18.5 11 1.7",
      "p M8 18c0-3 1.8-5.5 4-5.5s4 2.5 4 5.5c0 1.7-1.6 2.5-4 2.5s-4-.8-4-2.5z",
    ],
    clock: ["c 12 12 9", "p M12 7v5l3.5 2"],
    spark: ["p M12 3.5l1.9 5.1 5.1 1.9-5.1 1.9L12 17.5l-1.9-5.1L5 10.5l5.1-1.9z", "p M18.5 15v4M16.5 17h4"],
    reveal: ["r 3 5.5 9.5 13 2", "r 11.5 3.5 9.5 13 2", "p M13.6 10s1.2-2 2.7-2 2.7 2 2.7 2-1.2 2-2.7 2-2.7-2-2.7-2z"],
    "next-round": ["p M19.5 12a7.5 7.5 0 1 1-2.2-5.3", "p M19.5 4v3.5H16"],
    lower: ["r 6 3 12 9.5 2", "p M12 14.5v6M9 17.5l3 3 3-3"],
    pause: ["c 12 12 9", "p M10 9v6M14 9v6"],
    play: ["c 12 12 9", "p M10 8.5l5.5 3.5-5.5 3.5z"],
    reaction: ["c 11 13 7.5", "p M8 15s1.2 2 3 2 3-2 3-2M8.8 10.8h.01M13.2 10.8h.01", "p M19.5 2v4M17.5 4h4"],
    card: ["r 5.5 3 13 18 2", "p M9 9.5h6M9 13h6"],
    dial: ["c 12 11 7.5", "p M12 3.5v4M3.5 21h17"],
    eye: ["p M2.5 12s3.5-6.5 9.5-6.5 9.5 6.5 9.5 6.5-3.5 6.5-9.5 6.5S2.5 12 2.5 12z", "c 12 12 2.8"],
    mug: ["p M4.5 7h11v8.5a4.5 4.5 0 0 1-4.5 4.5H9a4.5 4.5 0 0 1-4.5-4.5z", "p M15.5 9a3.4 3.4 0 0 1 0 6.8"],
    envelope: ["r 3 5.5 18 13 2", "p M3.5 7l8.5 6 8.5-6"],
    rocket: [
      "p M12 2.5c3 2.2 4.5 5.5 4.5 9.5v4h-9v-4c0-4 1.5-7.3 4.5-9.5z",
      "c 12 9.5 1.8",
      "p M7.5 12.5L5 15v3.5l2.5-1.5M16.5 12.5L19 15v3.5l-2.5-1.5",
      "p M10 16v1.5c0 1.6.8 2.8 2 4 1.2-1.2 2-2.4 2-4V16",
    ],
    "check-circle": ["c 12 12 9", "p M8 12.5l2.8 2.8L16.5 9"],
    coin: ["c 12 12 9", "c 12 12 6", "p M12 9v6"],
    chip: ["r 6 6 12 12 2", "r 9.5 9.5 5 5 1", "p M9.5 2.5V6M14.5 2.5V6M9.5 18v3.5M14.5 18v3.5M2.5 9.5H6M2.5 14.5H6M18 9.5h3.5M18 14.5h3.5"],
    proxy: [
      "c 8 9 3",
      "p M2.5 20a5.5 5.5 0 0 1 11 0",
      "p M14 3.5h6.5a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H18l-2 2v-2h-2a1 1 0 0 1-1-1v-4a1 1 0 0 1 1-1z",
    ],
    robot: ["r 5 8 14 11 3", "c 12 4 1.2", "p M12 5.2V8M9.5 12.5h.01M14.5 12.5h.01M9.5 15.5h5M3 12v3M21 12v3"],
  };

  /** The icon a custom card without an emoji shows. */
  const FALLBACK = "spark";

  // The icon and short key label of each card of the built-in packs, by deck and card id; null for a card whose
  // words are its face, such as a poker value.
  const CARDS = {
    meeting: {
      "wrap-up": ["timer", "Wrap"],
      "off-track": ["off-track", "Off track"],
      question: ["question", "Question"],
      agree: ["thumbs-up", "Agree"],
      disagree: ["thumbs-down", "Disagree"],
      "cant-hear": ["volume-off", "Can't hear"],
      offline: ["chat", "Offline"],
      "next-topic": ["skip", "Next topic"],
    },
    poker: Object.assign(
      Object.fromEntries(["p0", "p1", "p2", "p3", "p5", "p8", "p13", "p21", "unsure"].map((id) => [id, null])),
      { break: ["coffee", "Break"] },
    ),
    classroom: Object.assign(
      {
        hand: ["hand", "Hand up"],
        question: ["question", "Question"],
        repeat: ["repeat", "Repeat"],
        slower: ["gauge", "Slower"],
        "got-it": ["check-circle", "Got it"],
      },
      Object.fromEntries(["answer-a", "answer-b", "answer-c", "answer-d"].map((id) => [id, null])),
    ),
    status: {
      muted: ["mic-off", "Mute"],
      brb: ["coffee", "Back soon"],
      lunch: ["utensils", "Lunch"],
      focus: ["headphones", "Focus"],
      presenting: ["monitor", "Presenting"],
      "on-call": ["phone", "Other call"],
    },
  };

  /** The icon of starter elements that are not cards, by element id. */
  const ELEMENTS = { brb: "clock", "name-label": "person", "now-showing": "box", "pet-alert": "paw" };

  /** Words this short, such as "13" or "A", are a card's face without an icon. */
  const SHORT = 3;

  const graphemes = new Intl.Segmenter("en", { granularity: "grapheme" });
  const PICTOGRAPH = /\p{Extended_Pictographic}|\p{Regional_Indicator}/u;

  /** One shape of the short form as an SVG tag name and its attributes. */
  function shape(spec) {
    const [kind, ...rest] = spec.split(" ");
    if (kind === "p") {
      return ["path", { d: rest.join(" ") }];
    }
    const numbers = rest.map(Number);
    if (kind === "c") {
      return ["circle", { cx: numbers[0], cy: numbers[1], r: numbers[2] }];
    }
    return ["rect", { x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3], rx: numbers[4] }];
  }

  const ICONS = Object.freeze(
    Object.fromEntries(Object.entries(DRAWINGS).map(([name, specs]) => [name, Object.freeze(specs.map(shape))])),
  );

  /** The shapes of icon `name` as SVG markup, without stroke attributes; "" for an unknown name. */
  function iconMarkup(name) {
    const shapes = Object.prototype.hasOwnProperty.call(ICONS, name) ? ICONS[name] : [];
    return shapes
      .map(([tag, attributes]) => {
        const pairs = Object.entries(attributes).map(([key, value]) => `${key}="${value}"`);
        return `<${tag} ${pairs.join(" ")}/>`;
      })
      .join("");
  }

  /** The emoji `text` starts with and the words after it, or null when it starts with none. */
  function leadingEmoji(text) {
    const value = String(text).trimStart();
    const first = graphemes.segment(value)[Symbol.iterator]().next().value;
    if (!first || !PICTOGRAPH.test(first.segment)) {
      return null;
    }
    return { emoji: first.segment, rest: value.slice(first.segment.length).trim() };
  }

  /**
   * How a card of deck `deck` with id `card` and words `text` shows as an icon: `icon`, a line icon's name, or
   * `emoji`, its own leading emoji; `text`, its words without that emoji; and `label`, the short label a key gives
   * it. A built-in card shows its own icon and label, or just its words; a custom card its leading emoji, its words
   * alone when they are as short as a number, or else the fallback icon.
   */
  function cardIcon(deck, card, text) {
    const words = String(text);
    const known = Object.prototype.hasOwnProperty.call(CARDS, deck) ? CARDS[deck] : null;
    if (known && Object.prototype.hasOwnProperty.call(known, card)) {
      const entry = known[card];
      if (!entry) {
        return { icon: null, emoji: null, text: words, label: words.trim() };
      }
      const lead = leadingEmoji(words);
      return { icon: entry[0], emoji: null, text: lead ? lead.rest : words.trim(), label: entry[1] };
    }
    const lead = leadingEmoji(words);
    if (lead) {
      return { icon: null, emoji: lead.emoji, text: lead.rest, label: lead.rest };
    }
    const short = Array.from(graphemes.segment(words.trim())).length <= SHORT;
    return { icon: short ? null : FALLBACK, emoji: null, text: words, label: words.trim() };
  }

  /** How a text element that is not a card shows as an icon: a starter element's icon, else its leading emoji. */
  function elementIcon(id, text) {
    const lead = leadingEmoji(text);
    if (Object.prototype.hasOwnProperty.call(ELEMENTS, id)) {
      return { icon: ELEMENTS[id], emoji: null, text: lead ? lead.rest : String(text) };
    }
    return lead ? { icon: null, emoji: lead.emoji, text: lead.rest } : { icon: null, emoji: null, text: String(text) };
  }

  const api = { CARDS, ELEMENTS, FALLBACK, ICONS, STROKE_WIDTH, cardIcon, elementIcon, iconMarkup, leadingEmoji };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronIcons = api;
  }
})();
