// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/panels/deck-cards.js: a card editor's Hand signal list, labeled and showing the card's own signal,
// and the warnings and notes on what that signal shares with other cards, which the list's description reads.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const web = new URL("../", import.meta.url);
const SCRIPTS = [
  "render-style.js",
  "themes/graphite.js",
  "themes/hacker.js",
  "themes/eighties.js",
  "themes/windows-95.js",
  "themes/classic-mac.js",
  "themes/vaporwave.js",
  "render-themes.js",
  "render-fit.js",
  "icons.js",
  "render-icons.js",
  "render-results.js",
  "render-cards.js",
  "render-clock.js",
  "render-subtitles.js",
  "render.js",
  "editor/constants.js",
  "editor/values.js",
  "editor/project.js",
  "editor/deck-edits.js",
  "editor/cards-text.js",
  "editor/signals-context.js",
  "editor/signals-text.js",
  "editor/voice-text.js",
];
const PANELS = ["editor/panels/deck-voice.js", "editor/panels/deck-cards.js"];

/** Loads the scripts the deck markup uses, with markup helpers that keep the tree as plain objects. */
function loadDeckCards() {
  // Like dom.js, an attribute that is null, undefined, or false is left out.
  const kept = ([, value]) => ![null, undefined, false].includes(value);
  const node = (tag, attrs, children) => ({
    tag,
    attrs: Object.fromEntries(Object.entries(attrs || {}).filter(kept)),
    children: children.flat().filter((child) => child !== null && child !== undefined && child !== false),
    style: { setProperty() {} },
    setAttribute(name, value) {
      this.attrs[name] = value;
    },
  });
  const dom = {
    h: (tag, attrs, ...children) => node(tag, attrs, children),
    hidden: (text) => node("span", { class: "visually-hidden" }, [text]),
    option: (value, label) => node("option", { value }, [label]),
    field: (id, label, control) => node("div", { class: "field" }, [node("label", { for: id }, [label]), control]),
    check: (id, label, input) => node("div", { class: "check" }, [input, node("label", { for: id }, [label])]),
  };
  const context = vm.createContext({});
  context.window = context;
  const run = (script) => vm.runInContext(readFileSync(new URL(script, web), "utf8"), context, { filename: script });
  SCRIPTS.forEach(run);
  context.SceneuronEditor.dom = dom;
  PANELS.forEach(run);
  return context.SceneuronEditor.deckCards;
}

function find(nodes, test) {
  for (const node of nodes) {
    if (node && typeof node === "object") {
      const found = test(node) ? node : find(node.children, test);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

const text = (node) => node.children.map((child) => (typeof child === "string" ? child : text(child))).join("");

/** A draft with a meeting-like deck and a poker-like deck, whose hand signals are on. */
function draft() {
  const slot = (id) => ({ id, name: id, kind: "text", props: { visible: false, text: "" } });
  return {
    elements: [slot("meeting-card"), slot("poker-card")],
    decks: [
      {
        id: "meeting",
        name: "Meeting signals",
        element: "meeting-card",
        camera: false,
        cards: [
          { id: "wrap-up", name: "Wrap it up", props: { text: "Wrap it up" }, signal: { sign: "fist" } },
          { id: "cant-hear", name: "Can't hear you", props: { text: "Can't hear you" } },
        ],
      },
      {
        id: "poker",
        name: "Planning poker",
        element: "poker-card",
        camera: true,
        cards: [{ id: "p0", name: "0", props: { text: "0" }, signal: { fingers: 0 } }],
      },
    ],
  };
}

function cardEditor(project, editing) {
  const { deckCard } = loadDeckCards();
  const article = deckCard(project, project.decks[0], 0, { editing, open: true });
  return find([article], (node) => node.tag === "details");
}

test("each card has a labeled Hand signal list that shows its own signal", () => {
  const editor = cardEditor(draft(), "wrap-up");
  const select = find([editor], (node) => node.attrs.id === "deck-0-card-signal");
  assert.equal(select.tag, "select");
  assert.equal(select.attrs.value, "sign:fist");
  assert.equal(select.attrs["data-card-field"], "signal");
  assert.equal(select.attrs["data-focus-key"], "deck:meeting:card-signal");
  const label = find([editor], (node) => node.tag === "label" && node.attrs.for === "deck-0-card-signal");
  assert.equal(text(label), "Hand signal");
  const options = select.children.map((option) => [option.attrs.value, text(option)]);
  assert.equal(options.length, 19);
  assert.deepEqual(options[0], ["", "None"]);
  assert.deepEqual(options.at(-1), ["sign:fist", "Fist"]);
  assert.equal(find([editor], (node) => /Saving keeps it/.test(text(node))), null, "the old read-only line is gone");
  const none = find([cardEditor(draft(), "cant-hear")], (node) => node.attrs.id === "deck-0-card-signal");
  assert.equal(none.attrs.value, "");
});

test("what the signal shares with a listening deck is listed under the list and describes it", () => {
  const editor = cardEditor(draft(), "wrap-up");
  const select = find([editor], (node) => node.attrs.id === "deck-0-card-signal");
  assert.equal(select.attrs["aria-describedby"], "deck-0-card-signal-notes");
  const notes = find([editor], (node) => node.attrs.id === "deck-0-card-signal-notes");
  assert.equal(notes.tag, "ul");
  assert.deepEqual(
    notes.children.map((item) => [item.attrs.class, text(item)]),
    [[
      "hint",
      "Planning poker: 0 has 0 fingers up. A fist also counts as 0 fingers, so once this deck's hand signals are on " +
        "too, a vote in Planning poker decides: 0 is raised while one is open, and this card while none is.",
    ]],
  );
});

test("a warning comes before the notes, and a signal shared with nothing has no description", () => {
  const project = draft();
  project.decks[1].cards.push({ id: "fist", name: "Fist vote", props: {}, signal: { sign: "fist" } });
  const notes = find([cardEditor(project, "wrap-up")], (node) => node.attrs.id === "deck-0-card-signal-notes");
  assert.deepEqual(notes.children.map((item) => item.attrs.class), ["field-warning", "hint"]);
  assert.match(text(notes.children[0]), /^Planning poker: Fist vote also uses a fist, and its deck's hand signals/);
  project.decks[1].camera = false;
  const quiet = cardEditor(project, "wrap-up");
  const select = find([quiet], (node) => node.attrs.id === "deck-0-card-signal");
  assert.equal(select.attrs["aria-describedby"], undefined);
  assert.equal(find([quiet], (node) => node.attrs.id === "deck-0-card-signal-notes"), null);
});
