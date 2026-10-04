// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises render-style.js: color parsing, style computation, animation class mapping and planning,
// and canvas fitting.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

import { BRB, element } from "./render-fixtures.mjs";

const require = createRequire(import.meta.url);
const R = require("../render-style.js");

describe("parseColor and cssColor", () => {
  test("parse every hex length", () => {
    assert.deepEqual(R.parseColor("#fff"), { r: 255, g: 255, b: 255, a: 1 });
    assert.deepEqual(R.parseColor("#0008"), { r: 0, g: 0, b: 0, a: 136 / 255 });
    assert.deepEqual(R.parseColor("#9b5cff"), { r: 155, g: 92, b: 255, a: 1 });
    assert.deepEqual(R.parseColor("#9B5CFFCC"), { r: 155, g: 92, b: 255, a: 0.8 });
  });

  test("reject anything that is not a hex color", () => {
    for (const value of ["9b5cff", "#12345", "#ggg", "#", "", "red", null, undefined, 42, "#9b5cff00ff"]) {
      assert.equal(R.parseColor(value), null, String(value));
    }
  });

  test("css colors carry alpha as rgba", () => {
    assert.equal(R.cssColor("#141428cc"), "rgba(20, 20, 40, 0.8)");
    assert.equal(R.cssColor("#00000000"), "rgba(0, 0, 0, 0)");
    assert.equal(R.cssColor("#fff8"), "rgba(255, 255, 255, 0.533)");
    assert.equal(R.cssColor("#9b5cff"), "rgba(155, 92, 255, 1)");
  });

  test("invalid colors fall back", () => {
    assert.equal(R.cssColor("nope"), "transparent");
    assert.equal(R.cssColor("nope", "rgba(1, 2, 3, 1)"), "rgba(1, 2, 3, 1)");
  });
});

describe("computeStyle", () => {
  test("text element maps every drawn property", () => {
    assert.deepEqual(R.computeStyle("text", BRB), {
      left: "560px",
      top: "440px",
      width: "800px",
      height: "200px",
      display: "none",
      "--sn-accent": "rgba(155, 92, 255, 1)",
      opacity: "1",
      "background-color": "rgba(20, 20, 40, 0.8)",
      border: "6px solid rgba(155, 92, 255, 1)",
      "border-radius": "32px",
      color: "rgba(255, 255, 255, 1)",
      "font-size": "96px",
      "font-weight": "700",
      "text-align": "center",
    });
  });

  test("box is a filled rectangle without text styles", () => {
    const style = R.computeStyle("box", { background: "#ff000080", border_width: 0, color: "#00ff00" });
    assert.equal(style["background-color"], "rgba(255, 0, 0, 0.502)");
    assert.equal(style.border, "none");
    assert.equal(style["--sn-accent"], "rgba(255, 0, 0, 1)");
    for (const key of ["color", "font-size", "font-weight", "text-align"]) {
      assert.ok(!(key in style), key);
    }
  });

  test("frame is transparent inside and draws its border", () => {
    const style = R.computeStyle("frame", {
      background: "#ff0000ff",
      border_color: "#9b5cff",
      border_width: 8,
      corner_radius: 24,
    });
    assert.equal(style["background-color"], "transparent");
    assert.equal(style.border, "8px solid rgba(155, 92, 255, 1)");
    assert.equal(style["border-radius"], "24px");
    assert.equal(style["--sn-accent"], "rgba(155, 92, 255, 1)");
    assert.ok(!("color" in style));
  });

  test("visibility and opacity", () => {
    assert.equal(R.computeStyle("box", { visible: true }).display, "block");
    assert.equal(R.computeStyle("box", { visible: false }).display, "none");
    assert.equal(R.computeStyle("box", { opacity: 0.456 }).opacity, "0.456");
    assert.equal(R.computeStyle("box", { opacity: 2 }).opacity, "1");
    assert.equal(R.computeStyle("box", { opacity: -1 }).opacity, "0");
    assert.equal(R.computeStyle("box", { opacity: Number.NaN }).opacity, "1");
    assert.equal(R.computeStyle("box", { opacity: "0.5" }).opacity, "1");
  });

  test("alignment and weight", () => {
    assert.equal(R.computeStyle("text", { align: "left" })["text-align"], "left");
    assert.equal(R.computeStyle("text", { align: "right" })["text-align"], "right");
    assert.equal(R.computeStyle("text", { align: "center" })["text-align"], "center");
    assert.equal(R.computeStyle("text", { align: "justify" })["text-align"], "center");
    assert.equal(R.computeStyle("text", { bold: false })["font-weight"], "400");
    assert.equal(R.computeStyle("text", { bold: true })["font-weight"], "700");
  });

  test("missing or bad numbers fall back to defaults", () => {
    const style = R.computeStyle("text", {});
    assert.equal(style.left, "0px");
    assert.equal(style.width, "480px");
    assert.equal(style.height, "120px");
    assert.equal(style["font-size"], "48px");
    assert.equal(style.display, "block");
    assert.equal(R.computeStyle("text", { x: 10.12345, width: -5 }).left, "10.12px");
    assert.equal(R.computeStyle("text", { width: -5 }).width, "0px");
  });

  test("glow accent prefers a visible border, then the fill, then the text color", () => {
    assert.equal(
      R.computeStyle("text", { background: "#00000000", color: "#ffcc00" })["--sn-accent"],
      "rgba(255, 204, 0, 1)",
    );
    const hiddenBorder = { border_width: 4, border_color: "#ff000000", background: "#0000ff80" };
    assert.equal(R.computeStyle("box", hiddenBorder)["--sn-accent"], "rgba(0, 0, 255, 1)");
  });
});

describe("animation classes", () => {
  test("enter, idle, and effect names map to classes", () => {
    assert.equal(R.enterClass("fade"), "sn-enter-fade");
    assert.equal(R.enterClass("slide-up"), "sn-enter-slide-up");
    assert.equal(R.enterClass("pop"), "sn-enter-pop");
    assert.equal(R.idleClass("pulse"), "sn-idle-pulse");
    assert.equal(R.idleClass("glow"), "sn-idle-glow");
    assert.equal(R.idleClass("float"), "sn-idle-float");
    for (const effect of ["shake", "pop", "flash", "bounce"]) {
      assert.equal(R.effectClass(effect), `sn-fx-${effect}`);
    }
  });

  test("none and unknown values map to no class", () => {
    for (const value of ["none", "spin", "", undefined, "__proto__", "constructor", "toString"]) {
      assert.equal(R.enterClass(value), "", String(value));
      assert.equal(R.idleClass(value), "", String(value));
      assert.equal(R.effectClass(value), "", String(value));
    }
  });
});

describe("planElement", () => {
  const shown = element("a", "text", { enter: "fade", idle: "glow" }, { effect: "shake", nonce: 4 });
  const hidden = element("a", "text", { visible: false, enter: "fade", idle: "glow" }, { effect: "shake", nonce: 4 });

  test("first render of a visible element plays enter but not an old effect", () => {
    const plan = R.planElement(null, shown, {});
    assert.equal(plan.restartEnter, true);
    assert.equal(plan.enterClass, "sn-enter-fade");
    assert.equal(plan.idleClass, "sn-idle-glow");
    assert.equal(plan.playEffect, false);
    assert.equal(plan.nonce, 4);
  });

  test("enter plays again only when the element becomes visible", () => {
    assert.equal(R.planElement({ visible: true, nonce: 4 }, shown, {}).restartEnter, false);
    assert.equal(R.planElement({ visible: false, nonce: 4 }, shown, {}).restartEnter, true);
  });

  test("hidden elements run no animations", () => {
    const plan = R.planElement({ visible: true, nonce: 4 }, hidden, {});
    assert.deepEqual(
      [plan.visible, plan.ghost, plan.restartEnter, plan.enterClass, plan.idleClass],
      [false, false, false, "", ""],
    );
    assert.equal(R.planElement(null, hidden, { ghostHidden: true }).ghost, true);
  });

  test("effects replay when the nonce changes while visible", () => {
    const next = element("a", "box", {}, { effect: "bounce", nonce: 5 });
    const plan = R.planElement({ visible: true, nonce: 4 }, next, {});
    assert.equal(plan.playEffect, true);
    assert.equal(plan.effectClass, "sn-fx-bounce");
    assert.equal(R.planElement({ visible: true, nonce: 5 }, next, {}).playEffect, false);
    assert.equal(R.planElement({ visible: true, nonce: null }, next, {}).playEffect, true);
    const hiddenNext = element("a", "box", { visible: false }, { effect: "bounce", nonce: 6 });
    assert.equal(R.planElement({ visible: true, nonce: 5 }, hiddenNext, {}).playEffect, false);
    assert.equal(R.planElement({ visible: true, nonce: 5 }, element("a", "box", {}), {}).playEffect, false);
  });
});

describe("fitTransform", () => {
  test("scales a canvas down to fit and centers it", () => {
    assert.deepEqual(R.fitTransform({ width: 1920, height: 1080 }, 960, 540), {
      scale: 0.5,
      x: 0,
      y: 0,
      width: 1920,
      height: 1080,
    });
    const square = R.fitTransform({ width: 1920, height: 1080 }, 1000, 1000);
    assert.equal(square.scale, 1000 / 1920);
    assert.equal(square.x, 0);
    assert.ok(Math.abs(square.y - 218.75) < 1e-9, String(square.y));
  });

  test("handles an empty viewport and a missing canvas", () => {
    assert.equal(R.fitTransform({ width: 1920, height: 1080 }, 0, 0).scale, 0);
    const fallback = R.fitTransform(undefined, 1920, 1080);
    assert.deepEqual([fallback.width, fallback.height, fallback.scale], [1920, 1080, 1]);
  });
});
