// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the overlay draws the live description bar, as the server sends it with its placeholders filled in:
// its two lines, left-aligned, updated in place without an enter animation as the line changes, hidden while it is
// empty, and under every theme a wide window titled with its name, drawn in the theme's own colors and first text
// color, which themes.test.mjs checks for contrast.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument } from "./fake-dom.mjs";
import { element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");
const T = require("../render-themes.js");

/** The bar as the live description pack (crates/sceneuron-core/src/packs/live_description.rs) authors it. */
const BAR_PROPS = {
  x: 380,
  y: 196,
  width: 1160,
  height: 180,
  font_size: 32,
  corner_radius: 16,
  align: "left",
  enter: "none",
  color: "#ffffffff",
  background: "#111827ff",
};

function bar(text, visible = true) {
  return { ...element("live-description", "text", { ...BAR_PROPS, text, visible }), name: "Live description" };
}

function snapshot(theme, ...elements) {
  return { revision: 1, canvas: { width: 1920, height: 1080 }, theme, decks: [], elements };
}

function draw() {
  const doc = createDocument({ constructable: true });
  const root = doc.body.appendChild(doc.createElement("div"));
  const parts = () => {
    const node = root.children[0];
    const enter = node.children[0];
    const body = enter.children[0].children[0].children[0];
    const [text, title] = body.children;
    return { node, enter, body, text, title };
  };
  return { root, parts };
}

describe("the live description bar", () => {
  test("shows the line and the caption on two lines, left-aligned, in its own colors", () => {
    const { root, parts } = draw();
    R.render(root, snapshot("none", bar("1 person; right hand raised, peace sign\nA person waving (just now)")));
    const { node, body, text } = parts();
    assert.equal(text.textContent, "1 person; right hand raised, peace sign\nA person waving (just now)");
    assert.equal(node.style.getPropertyValue("display"), "block");
    assert.equal(body.style.getPropertyValue("text-align"), "left");
    assert.equal(body.style.getPropertyValue("background-color"), "rgba(17, 24, 39, 1)");
    assert.equal(body.style.getPropertyValue("color"), "rgba(255, 255, 255, 1)");
  });

  test("follows the line in place, plays no enter animation, and hides while it is empty", () => {
    const { root, parts } = draw();
    R.render(root, snapshot("none", bar("", false)));
    const { node, enter, text } = parts();
    assert.equal(node.style.getPropertyValue("display"), "none");
    for (const line of ["1 person", "Nobody in view", "2 people; mouth open"]) {
      R.render(root, snapshot("none", bar(line)));
      assert.equal(parts().node, node, "the same node");
      assert.equal(text.textContent, line);
      assert.equal(node.style.getPropertyValue("display"), "block");
      assert.deepEqual([...enter.classList.names], ["sn-layer", "sn-enter"], "no enter animation");
    }
    R.render(root, snapshot("none", bar("", false)));
    assert.equal(node.style.getPropertyValue("display"), "none");
  });

  test("every theme draws it as a wide window titled with its name, in the theme's colors", () => {
    for (const theme of T.THEME_IDS.filter((id) => id !== "none")) {
      const { root, parts } = draw();
      R.render(root, snapshot(theme, bar("1 person\nA desk (10 s ago)")));
      const { node, body, text, title } = parts();
      assert.equal(root.getAttribute("data-theme"), theme);
      assert.equal(text.textContent, "1 person\nA desk (10 s ago)", theme);
      assert.equal(title.children[0].textContent, "Live description", theme);
      const classes = node.classList;
      assert.ok(classes.contains("sn-wide") && !classes.contains("sn-card"), `${theme}: a wide window`);
      assert.ok(!classes.contains("sn-alt-text"), `${theme}: white text takes the first text color`);
      for (const property of ["background-color", "color"]) {
        assert.equal(body.style.getPropertyValue(property), "", `${theme} draws its own ${property}`);
      }
    }
  });
});
