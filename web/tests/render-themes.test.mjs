// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-themes.js and how render() draws a theme on a fake DOM: the theme registry, the styles a theme
// takes over from an element, the root's theme and decoration unit, decorations titled with the element's or its
// deck's name, the card, wide, and second text color classes, unknown themes drawn as none, and None restoring
// each element's own look.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const T = require("../render-themes.js");
const R = require("../render.js");

const CANVAS = { width: 1920, height: 1080 };
const named = (name, item) => ({ ...item, name });
const LABEL = named("Name label", element("name-label", "text", { x: 60, y: 900, width: 640, height: 110,
  text: "Sceneuron", background: "#9b5cffdd", border_width: 4, border_color: "#ffffffff", corner_radius: 18 }));
const SLOT = named("Poker card", element("poker-card", "text", { x: 1560, y: 600, width: 300, height: 420,
  text: "8", font_size: 130 }));
const FRAME = named("Camera frame", element("cam-frame", "frame", { width: 620, height: 360, border_width: 8,
  border_color: "#9b5cffff" }));

function snapshot(theme, ...elements) {
  const decks = [{ id: "poker", name: "Planning poker", element: "poker-card", raised: "p8" }];
  return { revision: 1, canvas: CANVAS, theme, decks, elements };
}

function newRoot() {
  const doc = createDocument({ constructable: true });
  return { doc, root: doc.body.appendChild(doc.createElement("div")) };
}

/** The outer node, the body, the text, and the decorations of an element in `root`. */
function parts(root, id) {
  const node = root.children.find((child) => child.getAttribute("data-element-id") === id);
  const body = node.children[0].children[0].children[0].children[0];
  const [text, title, ok] = body.children;
  return { node, body, text, title, ok };
}

describe("registry", () => {
  test("lists every theme in the editor's order, with what the picker and the stylesheet need", () => {
    assert.deepEqual(T.THEME_IDS, ["none", "graphite", "hacker", "eighties", "windows-95", "classic-mac", "vaporwave"]);
    assert.deepEqual(T.THEMES.map((theme) => theme.id), T.THEME_IDS);
    assert.deepEqual(T.THEMES.map((theme) => theme.name), [
      "None",
      "Graphite",
      "Hacker",
      "80s computer",
      "Windows 95",
      "Classic Mac",
      "Vaporwave",
    ]);
    for (const theme of T.THEMES) {
      assert.ok(theme.description.endsWith("."), theme.id);
      assert.ok(T.STYLESHEET.includes(theme.css), theme.id);
    }
    assert.equal(T.THEMES[0].css, "", "None draws nothing of its own");
    assert.ok(T.STYLESHEET.includes(".sn-deco{display:none}"), "decorations hide without a theme");
  });

  test("only a known id names a theme; anything else draws as none", () => {
    for (const id of T.THEME_IDS) {
      assert.equal(T.themeId(id), id);
      assert.equal(T.themeById(id).id, id);
    }
    for (const value of ["Hacker", "y2k", "", " hacker", undefined, null, 3, {}]) {
      assert.equal(T.themeId(value), "none", String(value));
    }
    assert.equal(T.themeById("y2k").id, "none");
  });
});

describe("themeStyle", () => {
  test("None keeps every style the element's properties give", () => {
    const style = R.computeStyle("text", LABEL.props);
    assert.equal(T.themeStyle(style, "none"), style);
  });

  test("a theme takes over colors, borders, and corners and keeps place, size, type size, and opacity", () => {
    const style = R.computeStyle("text", { ...LABEL.props, opacity: 0.5 });
    const themed = T.themeStyle(style, "windows-95");
    for (const key of ["background-color", "border", "border-radius", "color", "--sn-accent"]) {
      assert.ok(key in style && !(key in themed), key);
    }
    for (const key of ["left", "top", "width", "height", "display", "opacity", "font-size", "font-weight"]) {
      assert.equal(themed[key], style[key], key);
    }
    assert.equal(themed["text-align"], "center");
    assert.equal(themed["--sn-h"], "110px", "decorations size to the element's height");
    assert.ok("color" in style, "the computed style is not changed");
  });
});

describe("applyTheme and deckNames", () => {
  test("the root carries the theme and a decoration unit that follows the canvas height", () => {
    const { root } = newRoot();
    assert.equal(T.applyTheme(root, "hacker", CANVAS), "1px");
    assert.equal(root.getAttribute("data-theme"), "hacker");
    assert.equal(root.style.getPropertyValue("--sn-u"), "1px");
    assert.equal(T.applyTheme(root, "none", { width: 1280, height: 720 }), "0.6667px");
    assert.equal(root.getAttribute("data-theme"), "none");
    assert.equal(root.style.getPropertyValue("--sn-u"), "0.6667px");
    assert.equal(T.applyTheme(root, "none", null), "1px");
    assert.equal(root.style.getPropertyValue("--sn-u"), "1px");
  });

  test("each card slot is named after the first deck it shows", () => {
    const decks = [
      { id: "poker", name: "Planning poker", element: "slot" },
      { id: "other", name: "Other", element: "slot" },
      { id: "status", element: "banner" },
      null,
    ];
    assert.deepEqual([...T.deckNames({ decks })], [["slot", "Planning poker"], ["banner", ""]]);
    assert.equal(T.deckNames({}).size, 0);
    assert.equal(T.deckNames(null).size, 0);
  });
});

describe("render with a theme", () => {
  test("marks the root and draws the theme's panel instead of the element's own colors, in the same place", () => {
    const { root } = newRoot();
    R.render(root, snapshot("hacker", LABEL));
    const { node, body } = parts(root, "name-label");
    assert.equal(root.getAttribute("data-theme"), "hacker");
    assert.equal(node.style.getPropertyValue("left"), "60px");
    assert.equal(node.style.getPropertyValue("width"), "640px");
    assert.equal(node.style.getPropertyValue("--sn-h"), "110px");
    assert.equal(node.style.getPropertyValue("--sn-accent"), "", "the theme sets the accent");
    for (const property of ["background-color", "border", "border-radius", "color"]) {
      assert.equal(body.style.getPropertyValue(property), "", property);
    }
    assert.equal(body.style.getPropertyValue("font-size"), "48px");
  });

  test("titles windows with the element's name and dialogs with their deck's, hidden from screen readers", () => {
    const { root } = newRoot();
    R.render(root, snapshot("windows-95", LABEL, SLOT, FRAME));
    const label = parts(root, "name-label");
    assert.equal(label.text.textContent, "Sceneuron");
    assert.ok(label.title.classList.contains("sn-deco") && label.title.classList.contains("sn-title"));
    assert.equal(label.title.children[0].textContent, "Name label");
    assert.deepEqual(
      label.title.children.slice(1).map((button) => [...button.classList.names]),
      [["sn-button", "sn-button-min"], ["sn-button", "sn-button-max"], ["sn-button", "sn-button-close"]],
    );
    assert.equal(label.ok.textContent, "OK");
    for (const decoration of [label.title, label.ok]) {
      assert.equal(decoration.getAttribute("aria-hidden"), "true");
    }
    assert.ok(label.node.classList.contains("sn-wide") && !label.node.classList.contains("sn-card"));
    const slot = parts(root, "poker-card");
    assert.equal(slot.title.children[0].textContent, "Planning poker");
    assert.ok(slot.node.classList.contains("sn-card") && !slot.node.classList.contains("sn-wide"));
    assert.equal(parts(root, "cam-frame").title.children[0].textContent, "Camera frame");
  });

  test("text whose own color has a hue gets the theme's second text color; white, black, and grays do not", () => {
    for (const color of ["#fde047ff", "#000080", "#dc2626ff", "#4f46e5"]) {
      assert.ok(T.hasHue(color), color);
    }
    for (const color of ["#ffffffff", "#111827ff", "#1a1a1aff", "#808080", "#f8fafc", "nope", undefined]) {
      assert.ok(!T.hasHue(color), String(color));
    }
    const yellow = named("Translated subtitles", element("subtitles-translated", "text", { color: "#fde047ff" }));
    const frame = { ...FRAME, props: { ...FRAME.props, color: "#fde047ff" } };
    const { root } = newRoot();
    const alt = (id) => parts(root, id).node.classList.contains("sn-alt-text");
    R.render(root, snapshot("hacker", LABEL, yellow, frame));
    assert.deepEqual([alt("name-label"), alt("subtitles-translated"), alt("cam-frame")], [false, true, false]);
    R.render(root, snapshot("none", LABEL, yellow, frame));
    assert.equal(alt("subtitles-translated"), false, "None draws the element's own color");
  });

  test("decorations follow renames and keep their nodes across renders", () => {
    const { root } = newRoot();
    R.render(root, snapshot("classic-mac", LABEL));
    const { title } = parts(root, "name-label");
    R.render(root, snapshot("classic-mac", { ...LABEL, name: "Streamer" }));
    assert.equal(parts(root, "name-label").title, title);
    assert.equal(title.children[0].textContent, "Streamer");
  });

  test("an unknown or missing theme draws as none", () => {
    for (const theme of ["y2k", undefined, null]) {
      const { root } = newRoot();
      R.render(root, snapshot(theme, LABEL, SLOT));
      assert.equal(root.getAttribute("data-theme"), "none", String(theme));
      const label = parts(root, "name-label");
      assert.equal(label.title, undefined, "no decorations");
      assert.equal(label.body.style.getPropertyValue("background-color"), "rgba(155, 92, 255, 0.867)");
      assert.ok(!parts(root, "poker-card").node.classList.contains("sn-card"));
    }
  });

  test("None brings back each element's own look and removes the decorations", () => {
    const { root } = newRoot();
    R.render(root, snapshot("none", LABEL, SLOT));
    const before = parts(root, "name-label");
    const own = ["background-color", "border", "border-radius", "color"].map((key) =>
      before.body.style.getPropertyValue(key));
    R.render(root, snapshot("vaporwave", LABEL, SLOT));
    assert.equal(parts(root, "name-label").body.children.length, 3);
    R.render(root, snapshot("none", LABEL, SLOT));
    const after = parts(root, "name-label");
    assert.equal(after.body, before.body, "the same nodes");
    assert.deepEqual(after.body.children, [after.text]);
    assert.deepEqual(
      ["background-color", "border", "border-radius", "color"].map((key) => after.body.style.getPropertyValue(key)),
      own,
    );
    assert.equal(after.node.style.getPropertyValue("--sn-h"), "");
    assert.ok(!parts(root, "poker-card").node.classList.contains("sn-card"));
    assert.ok(!after.node.classList.contains("sn-wide"));
  });

  test("the stylesheet the page adopts carries every theme", () => {
    const { doc, root } = newRoot();
    R.render(root, snapshot("none"));
    assert.equal(doc.adoptedStyleSheets.length, 1);
    for (const theme of T.THEMES) {
      assert.ok(doc.adoptedStyleSheets[0].text.includes(theme.css), theme.id);
    }
  });
});
