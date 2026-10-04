// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render.js: the animation stylesheet, fitting the root to the viewport, and render() driving a
// fake DOM.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { createDocument, descendants } from "./fake-dom.mjs";
import { BRB, element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render.js");

function snapshot(...elements) {
  return { revision: 1, now_ms: 0, canvas: { width: 1920, height: 1080 }, elements };
}

function layers(root, id) {
  const node = root.children.find((child) => child.getAttribute("data-element-id") === id);
  assert.ok(node, `no node for ${id}`);
  const [enter] = node.children;
  const [idle] = enter.children;
  const [fx] = idle.children;
  const [body] = fx.children;
  const [text] = body.children;
  return { node, enter, idle, fx, body, text };
}

function newRoot(options) {
  const doc = createDocument(options);
  const root = doc.body.appendChild(doc.createElement("div"));
  return { doc, root };
}

describe("module", () => {
  test("exports the public renderer API and nothing else", () => {
    const api = ["DEFAULT_PROPS", "STYLESHEET", "applyFit", "computeStyle", "cssColor", "effectClass", "enterClass"];
    api.push("fitTransform", "idleClass", "parseColor", "planElement", "render", "withDefaults");
    assert.deepEqual(Object.keys(R).sort(), api);
  });
});

describe("animation stylesheet", () => {
  test("the stylesheet defines a rule and keyframes for every class", () => {
    const classes = [
      ...["fade", "slide-up", "pop"].map(R.enterClass),
      ...["pulse", "glow", "float"].map(R.idleClass),
      ...["shake", "pop", "flash", "bounce"].map(R.effectClass),
    ];
    for (const cls of classes) {
      assert.ok(R.STYLESHEET.includes(`.${cls}{animation:${cls} `), cls);
      assert.ok(R.STYLESHEET.includes(`@keyframes ${cls}{`), cls);
    }
  });
});

describe("applyFit", () => {
  test("applyFit sizes the root to the canvas and scales it", () => {
    const { root } = newRoot();
    R.applyFit(root, { width: 1280, height: 720 }, 640, 720);
    assert.equal(root.style.getPropertyValue("width"), "1280px");
    assert.equal(root.style.getPropertyValue("height"), "720px");
    assert.equal(root.style.getPropertyValue("transform"), "translate(0px, 180px) scale(0.5)");
  });
});

describe("render", () => {
  test("creates one positioned node per element with text set as text", () => {
    const { root } = newRoot();
    const label = element("label", "text", { text: "<b>Hi</b>", x: 60, background: "#9b5cffdd" });
    R.render(root, snapshot(label, element("frame", "frame", { border_width: 8 })));
    assert.deepEqual(
      root.children.map((node) => node.getAttribute("data-element-id")),
      ["label", "frame"],
    );
    const { node, body, text } = layers(root, "label");
    assert.ok(root.classList.contains("sn-root"));
    assert.ok(node.classList.contains("sn-el") && node.classList.contains("sn-kind-text"));
    assert.equal(node.style.getPropertyValue("left"), "60px");
    assert.equal(node.style.getPropertyValue("background-color"), "");
    assert.equal(body.style.getPropertyValue("background-color"), "rgba(155, 92, 255, 0.867)");
    assert.equal(text.textContent, "<b>Hi</b>");
    assert.equal(text.children.length, 0);
    assert.equal(layers(root, "frame").text.textContent, "");
  });

  test("injects the stylesheet once per document", () => {
    const { doc, root } = newRoot();
    const second = doc.body.appendChild(doc.createElement("div"));
    R.render(root, snapshot(element("a", "box", {})));
    R.render(root, snapshot(element("a", "box", {})));
    R.render(second, snapshot());
    const styles = descendants(doc.head).filter((node) => node.tagName === "STYLE");
    assert.equal(styles.length, 1);
    assert.ok(styles[0].textContent.includes("@keyframes sn-enter-fade"));
  });

  test("prefers a constructed stylesheet, which the page CSP allows", () => {
    const { doc, root } = newRoot({ constructable: true });
    R.render(root, snapshot());
    R.render(root, snapshot());
    assert.equal(doc.adoptedStyleSheets.length, 1);
    assert.ok(doc.adoptedStyleSheets[0].text.includes("@keyframes sn-idle-pulse"));
    assert.equal(doc.head.children.length, 0);
  });

  test("updates nodes in place, reorders them, and removes missing ones", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("a", "box", { x: 1 }), element("b", "box", {}), element("c", "box", {})));
    const a = layers(root, "a").node;
    R.render(root, snapshot(element("c", "box", {}), element("a", "box", { x: 2 })));
    assert.deepEqual(
      root.children.map((node) => node.getAttribute("data-element-id")),
      ["c", "a"],
    );
    assert.equal(layers(root, "a").node, a);
    assert.equal(a.style.getPropertyValue("left"), "2px");
  });

  test("plays enter when an element becomes visible and clears it after the animation", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("brb", "text", BRB)));
    const { node, enter } = layers(root, "brb");
    assert.equal(node.style.getPropertyValue("display"), "none");
    assert.ok(!enter.classList.contains("sn-enter-pop"));

    R.render(root, snapshot(element("brb", "text", { ...BRB, visible: true })));
    assert.equal(node.style.getPropertyValue("display"), "block");
    assert.ok(enter.classList.contains("sn-enter-pop"));
    enter.dispatch("animationend", { animationName: "sn-enter-pop" });
    assert.ok(!enter.classList.contains("sn-enter-pop"));

    R.render(root, snapshot(element("brb", "text", { ...BRB, visible: true, text: "Back soon" })));
    assert.ok(!enter.classList.contains("sn-enter-pop"), "no replay while staying visible");
    assert.equal(layers(root, "brb").text.textContent, "Back soon");

    R.render(root, snapshot(element("brb", "text", BRB)));
    const reflows = enter.reflows;
    R.render(root, snapshot(element("brb", "text", { ...BRB, visible: true })));
    assert.ok(enter.classList.contains("sn-enter-pop"));
    assert.ok(enter.reflows > reflows, "restart forces a reflow");
  });

  test("ignores animation events that bubble from inner layers", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("a", "box", { enter: "fade" })));
    const { enter, idle } = layers(root, "a");
    for (const listener of enter.listeners.get("animationend")) {
      listener({ type: "animationend", target: idle });
    }
    assert.ok(enter.classList.contains("sn-enter-fade"));
  });

  test("loops idle animations only while visible", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("f", "frame", { idle: "glow" })));
    const { idle } = layers(root, "f");
    assert.ok(idle.classList.contains("sn-idle-glow"));
    R.render(root, snapshot(element("f", "frame", { idle: "float" })));
    assert.ok(idle.classList.contains("sn-idle-float") && !idle.classList.contains("sn-idle-glow"));
    R.render(root, snapshot(element("f", "frame", { idle: "float", visible: false })));
    assert.ok(!idle.classList.contains("sn-idle-float"));
  });

  test("replays a one-shot effect only when its nonce changes", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("f", "frame", {}, { effect: "flash", nonce: 1 })));
    const { fx } = layers(root, "f");
    assert.ok(!fx.classList.contains("sn-fx-flash"), "an effect from before the page loaded is not replayed");

    R.render(root, snapshot(element("f", "frame", {}, { effect: "flash", nonce: 2 })));
    assert.ok(fx.classList.contains("sn-fx-flash"));
    fx.dispatch("animationend", { animationName: "sn-fx-flash" });
    assert.ok(!fx.classList.contains("sn-fx-flash"));

    R.render(root, snapshot(element("f", "frame", {}, { effect: "flash", nonce: 2 })));
    assert.ok(!fx.classList.contains("sn-fx-flash"));

    R.render(root, snapshot(element("f", "frame", {}, { effect: "shake", nonce: 3 })));
    assert.ok(fx.classList.contains("sn-fx-shake"));
    R.render(root, snapshot(element("f", "frame", {}, { effect: "pop", nonce: 4 })));
    assert.ok(fx.classList.contains("sn-fx-pop") && !fx.classList.contains("sn-fx-shake"));
  });

  test("ghostHidden draws hidden elements faintly without animations", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("brb", "text", BRB)), { ghostHidden: true });
    const { node, enter, idle } = layers(root, "brb");
    assert.equal(node.style.getPropertyValue("display"), "block");
    assert.ok(node.classList.contains("sn-ghost"));
    assert.ok(!enter.classList.contains("sn-enter-pop") && !idle.classList.contains("sn-idle-pulse"));

    R.render(root, snapshot(element("brb", "text", { ...BRB, visible: true })), { ghostHidden: true });
    assert.ok(!node.classList.contains("sn-ghost"));
    assert.ok(enter.classList.contains("sn-enter-pop") && idle.classList.contains("sn-idle-pulse"));
  });

  test("skips malformed and duplicate elements", () => {
    const { root } = newRoot();
    R.render(root, { elements: [null, { kind: "box" }, element("a", "box", {}), element("a", "text", {})] });
    assert.equal(root.children.length, 1);
    assert.ok(layers(root, "a").node.classList.contains("sn-kind-box"));
    R.render(root, { elements: [element("odd", "not a kind", { text: "x" })] });
    assert.deepEqual([...layers(root, "odd").node.classList.names], ["sn-el", "sn-kind-box"]);
    assert.equal(layers(root, "odd").text.textContent, "");
    R.render(root, null);
    assert.equal(root.children.length, 0);
  });

  test("drops styles that no longer apply when the kind changes", () => {
    const { root } = newRoot();
    R.render(root, snapshot(element("a", "text", { color: "#ff0000" })));
    const { node, body } = layers(root, "a");
    assert.equal(body.style.getPropertyValue("color"), "rgba(255, 0, 0, 1)");
    R.render(root, snapshot(element("a", "box", { color: "#ff0000" })));
    assert.equal(body.style.getPropertyValue("color"), "");
    assert.ok(node.classList.contains("sn-kind-box") && !node.classList.contains("sn-kind-text"));
  });
});
