// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-subtitles.js: the stylesheet that puts the script fonts first for subtitle text, the room a
// label takes as the core reserves it, reading a snapshot's subtitle bar, and drawing one into a fake DOM through
// render(): the label beside the words, the script's class, Latin lines under each line, how each theme shows the
// label, and what render-fit.js is told.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const S = require("../render-subtitles.js");
const R = require("../render.js");
const T = require("../render-themes.js");

const QUENYA = "\u{E022}\u{E046}\u{E010}";

function snapshot(...elements) {
  return { revision: 1, now_ms: 0, canvas: { width: 1920, height: 1080 }, elements };
}

/** A subtitle element showing `text` with the subtitle bar `bar`. */
function subtitled(id, text, bar) {
  return { ...element(id, "text", { text, visible: true }), subtitles: true, subtitle_bar: bar };
}

/** An element's node, its body, and the body's children. */
function parts(root, id) {
  const node = root.children.find((child) => child.getAttribute("data-element-id") === id);
  const body = node.children[0].children[0].children[0].children[0];
  return { node, body, children: body.children };
}

function drawn(...elements) {
  const doc = createDocument();
  const root = doc.body.appendChild(doc.createElement("div"));
  R.render(root, snapshot(...elements), {});
  return root;
}

describe("stylesheet", () => {
  test("styles the label and the Latin lines in the type the core sizes them for", () => {
    assert.ok(S.STYLESHEET.includes(".sn-label{flex:none;"));
    assert.ok(S.STYLESHEET.includes(`font-size:${S.LABEL_SCALE}em;`), "the label's type");
    assert.ok(S.STYLESHEET.includes(`.sn-latin{font-size:${S.LATIN_SCALE}em;`), "a Latin line's type");
    assert.ok(S.STYLESHEET.includes("margin-right:0.875em;"), "the gap after a label, 0.35 of the words' type");
  });
});

describe("label room", () => {
  test("a label is as wide as the core reserves it, and nothing takes no room", () => {
    assert.equal(S.labelWidthEm(""), 0);
    assert.equal(S.labelRoomEm(""), 0);
    assert.ok(Math.abs(S.labelWidthEm("English") - (7 * 0.64 + 1)) < 1e-9);
    assert.ok(Math.abs(S.labelRoomEm("English") - ((7 * 0.64 + 1) * 0.4 + 0.35)) < 1e-9);
    assert.equal(S.labelWidthEm("日本語"), 3 * 0.64 + 1, "by characters, not code units");
  });
});

describe("barOf", () => {
  test("reads a snapshot's bar and checks each field", () => {
    assert.equal(S.barOf({}), null);
    assert.equal(S.barOf({ subtitle_bar: null }), null);
    assert.deepEqual(S.barOf({ subtitle_bar: { label: "Japanese", script: "", latin: [] } }), {
      label: "Japanese",
      script: "",
      latin: [],
    });
    assert.deepEqual(S.barOf({ subtitle_bar: { label: 3, script: "runes", latin: "no" } }), {
      label: "",
      script: "",
      latin: [],
    });
    assert.deepEqual(S.SCRIPTS, ["tengwar", "piqad", "sitelen-pona"]);
  });
});

describe("render", () => {
  test("a label goes beside the words and other text has none", () => {
    const root = drawn(
      subtitled("spoken", "Good morning", { label: "English (detected)", script: "", latin: [] }),
      element("plain", "text", { text: "Hello", visible: true }),
    );
    const spoken = parts(root, "spoken");
    const [label, text] = spoken.children;
    assert.ok(label.classList.contains("sn-label") && text.classList.contains("sn-text"));
    assert.equal(label.textContent, "English (detected)");
    assert.equal(label.hidden, false);
    const widest = Math.round(S.labelWidthEm("English (detected)") * 1000) / 1000;
    assert.equal(label.style.getPropertyValue("max-width"), `${widest}em`);
    assert.equal(text.textContent, "Good morning");
    assert.ok(spoken.node.classList.contains("sn-subtitle"));
    const plain = parts(root, "plain");
    assert.ok(plain.children[0].classList.contains("sn-text"), "no label");
    assert.ok(!plain.node.classList.contains("sn-subtitle"));
  });

  test("a label turned off hides, and comes back with its language", () => {
    const doc = createDocument();
    const root = doc.body.appendChild(doc.createElement("div"));
    const bar = (label) => ({ label, script: "", latin: [] });
    R.render(root, snapshot(subtitled("ja", "おはよう", bar("Japanese"))), {});
    R.render(root, snapshot(subtitled("ja", "おはよう", bar(""))), {});
    const [label] = parts(root, "ja").children;
    assert.equal(label.hidden, true);
    R.render(root, snapshot(subtitled("ja", "おはよう", bar("Japanese"))), {});
    assert.equal(label.hidden, false);
    assert.equal(parts(root, "ja").children.filter((child) => child.classList.contains("sn-label")).length, 1);
  });

  test("a script of its own marks the node, and Latin lines go under each line", () => {
    const doc = createDocument();
    const root = doc.body.appendChild(doc.createElement("div"));
    const bar = { label: "Quenya (Elvish)", script: "tengwar", latin: ["Elen", "síla"] };
    R.render(root, snapshot(subtitled("quenya", `${QUENYA}\n${QUENYA}`, bar)), {});
    const quenya = parts(root, "quenya");
    assert.ok(quenya.node.classList.contains("sn-script-tengwar") && quenya.node.classList.contains("sn-glossed"));
    const [, text] = quenya.children;
    const lines = text.children.map((child) => [[...child.classList.names][0], child.textContent]);
    assert.deepEqual(lines, [["sn-line", QUENYA], ["sn-latin", "Elen"], ["sn-line", QUENYA], ["sn-latin", "síla"]]);

    R.render(root, snapshot(subtitled("quenya", QUENYA, { ...bar, latin: [] })), {});
    assert.equal(text.textContent, QUENYA, "without Latin lines the words are plain text");
    assert.equal(text.children.length, 0);
    assert.ok(!quenya.node.classList.contains("sn-glossed") && quenya.node.classList.contains("sn-script-tengwar"));
    R.render(root, snapshot(subtitled("quenya", "Elen", { ...bar, script: "", latin: [] })), {});
    assert.ok(!S.SCRIPTS.some((script) => quenya.node.classList.contains(`sn-script-${script}`)));
  });
});

describe("themes", () => {
  test("every theme styles the label, and window themes title the bar with it instead", () => {
    for (const theme of T.THEMES.filter((candidate) => candidate.id !== "none")) {
      const root = `.sn-root[data-theme="${theme.id}"]`;
      assert.ok(theme.css.includes(`${root} .sn-label{`), theme.id);
      assert.equal(theme.css.includes(`${root} .sn-label{display:none}`), theme.titles, theme.id);
    }
    const doc = createDocument();
    const root = doc.body.appendChild(doc.createElement("div"));
    const bar = { label: "English (detected)", script: "", latin: [] };
    const spoken = { ...subtitled("spoken", "Good morning", bar), name: "Subtitles" };
    const title = () => parts(root, "spoken").children.find((child) => child.classList.contains("sn-title"));
    R.render(root, { ...snapshot(spoken), theme: "windows-95" }, {});
    assert.equal(title().children[0].textContent, "English (detected)");
    R.render(root, { ...snapshot({ ...spoken, subtitle_bar: { ...bar, label: "" } }), theme: "windows-95" }, {});
    assert.equal(title().children[0].textContent, "Subtitles", "without a label, the element's name");
  });

  test("subtitle words take the script fonts and then the theme's own type, and other text keeps the theme's", () => {
    const doc = createDocument();
    const root = doc.body.appendChild(doc.createElement("div"));
    const fonts = '"Sceneuron Tengwar","Sceneuron pIqaD","Sceneuron sitelen pona"';
    const bar = { label: "Quenya (Elvish)", script: "tengwar", latin: [] };
    const elements = [subtitled("quenya", QUENYA, bar), element("plain", "text", { text: "Hello", visible: true })];
    const family = (id) => parts(root, id).children.find((child) => child.classList.contains("sn-text"))
      .style.getPropertyValue("font-family");
    for (const theme of T.THEMES) {
      R.render(root, { ...snapshot(...elements), theme: theme.id }, {});
      assert.equal(family("quenya"), `${fonts},${theme.fonts[0]}`, theme.id);
      assert.equal(family("plain"), "", theme.id);
    }
    R.render(root, snapshot({ ...elements[0], subtitle_bar: null }), {});
    assert.equal(family("quenya"), "", "a bar that turns into plain text drops them");
  });
});

describe("fill", () => {
  function state() {
    const doc = createDocument();
    const body = doc.createElement("div");
    const text = body.appendChild(doc.createElement("span"));
    return { node: doc.createElement("div"), body, text };
  }

  test("tells render-fit.js the label's room, the lines' height, and how to put the words back", () => {
    const glossed = state();
    S.fill(glossed, `${QUENYA}\n${QUENYA}`, { label: "Quenya", script: "tengwar", latin: ["a", "b"] });
    assert.deepEqual(glossed.subtitleFit, { roomEm: S.labelRoomEm("Quenya"), lineSize: 1.65 });
    assert.equal(glossed.lineUnits, 3.3, "two lines, each with a Latin line 0.65 of one");
    glossed.text.textContent = "a sample measured in place";
    glossed.refill();
    assert.equal(glossed.text.children.length, 4);
    assert.equal(glossed.content, `${QUENYA}\n${QUENYA}`);

    const plain = state();
    S.fill(plain, "Hello", null);
    assert.deepEqual(plain.subtitleFit, { roomEm: 0, lineSize: 1 });
    assert.equal(plain.lineUnits, null);
    assert.equal(plain.refill, null);
    assert.equal(plain.label, undefined, "no label is made for text that never had one");
  });

  test("Latin lines need a script: a bar in Latin letters writes its words as they are", () => {
    const latin = state();
    S.fill(latin, "Elen síla", { label: "Quenya", script: "", latin: ["Elen síla"] });
    assert.equal(latin.text.textContent, "Elen síla");
    assert.equal(latin.lineUnits, null);
  });
});
