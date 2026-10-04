// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Theme steps of the browser regression test (editor-browser.mjs): the Settings tab's theme picker shows a theme in
// the preview before it is saved; saving shows it on the overlay page, which restyles the name label but keeps its
// panel in its box and its words; Windows 95 gives the overlay's elements title bars, frames a title bar above the
// picture, and card slots dialog icons, and the property form says what the theme draws and shows on stream; every
// theme fits held subtitles and a raised card in their panels; live subtitles keep one size while their lines
// arrive; an unknown theme is refused; and None brings back each element's own look. They run last, against the
// project the earlier steps saved, which has the Subtitles pack, with subtitles agreed to and off.

import { createRequire } from "node:module";

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const require = createRequire(import.meta.url);
const { THEMES } = require("../web/render-themes.js");

const js = JSON.stringify;
const LABEL = '[data-element-id="name-label"]';
const SPOKEN = "subtitles-spoken";
const HELD = {
  [SPOKEN]: "Good morning everyone, and welcome to the stream.\nToday we are planning the next sprint together.",
  "subtitles-translated":
    "Buenos días a todos y bienvenidos a la transmisión.\nHoy planificamos juntos el próximo sprint.",
};
// The subtitle bar's width, and one narrow enough that the simulated speaker's lines wrap after about 24
// characters, so each line shows as one line of words and then as two.
const BAR_WIDTH = 1160;
const NARROW = 560;
// A line takes about two seconds to say, stays four, and the next starts five seconds later.
const LINE_WAIT_MS = 15_000;
// Classic Mac draws its windows 5 units in from the box's right and bottom, for its drop shadow.
const MAC_SHADOW = 5;

/** In a page, the panel a theme paints for the element `selector` finds, in canvas pixels inside its box. */
const panel = (selector) => `(() => {
  const body = document.querySelector(${js(`${selector} .sn-body`)});
  return [body.offsetLeft, body.offsetTop, body.offsetWidth, body.offsetHeight];
})()`;
/** In the overlay page, each shown text element's line count and font size, and whether its words fit its panel. */
const MEASURES = `[...document.querySelectorAll("#overlay-root .sn-kind-text")]
  .filter((node) => getComputedStyle(node).display !== "none")
  .map((node) => {
    const [body, text] = [node.querySelector(".sn-body"), node.querySelector(".sn-text")];
    const padding = getComputedStyle(body);
    const room = body.clientHeight - parseFloat(padding.paddingTop) - parseFloat(padding.paddingBottom);
    const fits = text.offsetHeight <= room + 0.5 && text.scrollWidth <= text.clientWidth + 0.5;
    const lines = text.textContent.split("\\n").length;
    return { id: node.dataset.elementId, lines, size: getComputedStyle(text).fontSize, fits };
  })`;
/** In the overlay page, how far each frame's title bar reaches below the top of the frame's box. */
const FRAME_TITLES = `[...document.querySelectorAll("#overlay-root .sn-kind-frame")].map((node) =>
  node.querySelector(".sn-title").getBoundingClientRect().bottom - node.getBoundingClientRect().top)`;

/** Runs the theme steps; `overlay` is the overlay page the overlay steps opened. */
export async function runThemeSteps({ page, overlay, step, same, api }) {
  const { q, check, clean, ctrlS, dirty, item, node, project, selected } = editorHelpers(page, api);
  const rootTheme = (target, root) => target.eval(`document.getElementById(${js(root)}).getAttribute("data-theme")`);
  const labelStyle = (target, property) =>
    target.eval(`getComputedStyle(document.querySelector(${js(`${LABEL} .sn-body`)})).${property}`);
  const labelText = () => overlay.text(`${LABEL} .sn-text`);
  const choose = async (theme) => {
    await page.click("#tab-settings");
    await page.click(`#theme-${theme}`);
    await waitFor(`the ${theme} preview`, async () => (await rootTheme(page, "preview-root")) === theme);
  };
  const save = async (theme) => {
    await ctrlS();
    await clean();
    await waitFor(`the ${theme} overlay`, async () => (await rootTheme(overlay, "overlay-root")) === theme);
  };
  const framesTitledAbove = async () => {
    const below = await overlay.eval(FRAME_TITLES);
    check(below.length > 0 && below.every((reach) => reach <= 0.5), `frame title bars reach ${js(below)} in`);
  };
  const note = (id) => q(`(() => { const note = document.getElementById(${js(id)});
    return note.hidden ? null : note.textContent; })()`);
  const setWidth = async (width) => {
    await page.click("#tab-properties");
    await page.click(item(SPOKEN));
    await selected("Subtitles");
    await page.fill("prop-width", String(width));
    await waitFor("the width in the preview", async () =>
      (await q(`document.querySelector(${js(node(SPOKEN))}).style.width`)) === `${width}px`);
    await ctrlS();
    await clean();
  };
  const own = {};

  await step("the theme picker shows a theme in the preview before it is saved", async () => {
    await page.send("Page.bringToFront");
    await page.click("#tab-settings");
    same(await q(`document.querySelector('input[name="overlay-theme"]:checked').value`), "none", "the checked theme");
    same(await rootTheme(page, "preview-root"), "none", "the preview's theme");
    Object.assign(own, { font: await labelStyle(overlay, "fontFamily"), color: await labelStyle(overlay, "color") });
    Object.assign(own, { panel: await overlay.eval(panel(LABEL)), text: await labelText() });
    const before = await labelStyle(page, "fontFamily");
    await choose("hacker");
    const after = await labelStyle(page, "fontFamily");
    check(after.startsWith("Menlo") && after !== before, `the preview's font went from ${before} to ${after}`);
    same(await dirty(), "Unsaved changes", "save state");
    same(await rootTheme(overlay, "overlay-root"), "none", "the overlay's theme before saving");
    return after;
  });

  await step("saving shows the theme on the overlay, which keeps the label's panel and words", async () => {
    await save("hacker");
    same((await project()).theme, "hacker", "the saved theme");
    same(await labelStyle(overlay, "color"), "rgb(61, 255, 138)", "the label's text color");
    check((await labelStyle(overlay, "fontFamily")).startsWith("Menlo"), "the overlay's label is not in Menlo");
    same(await overlay.eval(panel(LABEL)), own.panel, "the label's panel");
    same(await labelText(), own.text, "the label's words");
    const prompt = await overlay.eval(`getComputedStyle(document.querySelector(${js(`${LABEL} .sn-text`)}),
      "::before").content`);
    same(prompt, '"$\u00a0"', "the prompt, kept with the first word");
  });

  await step("Windows 95 titles windows, puts frame titles above the picture, and draws dialogs", async () => {
    await choose("windows-95");
    await save("windows-95");
    const { elements, decks } = await project();
    const label = elements.find((element) => element.id === "name-label");
    same(await overlay.text(`${LABEL} .sn-title-text`), label.name, "the label's title");
    same(await overlay.eval(`getComputedStyle(document.querySelector(${js(`${LABEL} .sn-title`)})).display`), "flex",
      "the label's title bar");
    same(await labelStyle(overlay, "backgroundColor"), "rgb(192, 192, 192)", "the label's window");
    await framesTitledAbove();
    check(decks.length > 0, "the earlier steps saved no deck");
    same(await overlay.eval(`document.querySelectorAll("#overlay-root .sn-card").length`), decks.length, "dialogs");
    const slot = `[data-element-id=${js(decks[0].element)}]`;
    same(await overlay.text(`${slot} .sn-title-text`), decks[0].name, "a dialog's title");
    const icon = `getComputedStyle(document.querySelector(${js(`${slot} .sn-body`)}), "::before").content`;
    same(await overlay.eval(icon), '"i"', "a dialog's icon");
    return `${decks.length} dialogs`;
  });

  await step("the property form says what Windows 95 draws its own way and shows on stream", async () => {
    await page.click("#tab-properties");
    await page.click(item(SPOKEN));
    await selected("Subtitles");
    same(await note("theme-name-note"), "The Windows 95 theme shows this name on stream, in a title bar.", "name");
    check((await note("theme-appearance-note"))?.startsWith("The Windows 95 theme draws its own backgrounds"),
      "no note on the colors, borders, and corners");
    check((await note("theme-color-note"))?.startsWith("The Windows 95 theme draws text in its own color"),
      "no note on the text color");
    await choose("none");
    same([await note("theme-name-note"), await note("theme-appearance-note")], [null, null], "notes under None");
    await choose("windows-95");
    same(await dirty(), "All changes saved", "save state, back on the saved theme");
  });

  await step("every theme keeps the label's panel in its box and fits held subtitles and a raised card", async () => {
    for (const [element, text] of Object.entries(HELD)) {
      await api("POST", "/api/override", { element, props: { text, visible: true } });
    }
    const [deck] = (await project()).decks;
    const unit = Number.parseFloat(await overlay.eval(`document.getElementById("overlay-root").style
      .getPropertyValue("--sn-u")`));
    const inset = Math.round(MAC_SHADOW * unit);
    try {
      for (const theme of THEMES) {
        await choose(theme.id);
        await save(theme.id);
        await api("POST", "/api/card", { deck: deck.id, card: deck.cards[0].id, action: "raise" });
        const measures = await waitFor(`the raised card in ${theme.name}`, async () => {
          const shown = await overlay.eval(MEASURES);
          return shown.some((measure) => measure.id === deck.element) && shown;
        });
        const fitted = measures.filter((measure) => measure.fits).map((measure) => measure.id);
        for (const id of ["name-label", ...Object.keys(HELD), deck.element]) {
          check(fitted.includes(id), `${theme.name}: ${id} does not fit its panel: ${js(measures)}`);
        }
        const [x, y, width, height] = own.panel;
        const expected = theme.id === "classic-mac" ? [x, y, width - inset, height - inset] : own.panel;
        same(await overlay.eval(panel(LABEL)), expected, `the label's panel in ${theme.name}`);
        if (theme.titles) {
          await framesTitledAbove();
        }
      }
    } finally {
      await api("POST", "/api/release", { element: null });
      await api("POST", "/api/card", { deck: deck.id, action: "lower" });
    }
    return `${THEMES.length} themes`;
  });

  await step("Windows 95 keeps live subtitles at one size while their lines arrive", async () => {
    await choose("windows-95");
    await save("windows-95");
    await setWidth(NARROW);
    const seen = { lines: new Set(), sizes: new Set(), unfit: [] };
    try {
      await api("PUT", "/api/ai", { subtitles: { enabled: true } });
      await waitFor("one line of live words, and then two", async () => {
        const bar = (await overlay.eval(MEASURES)).find((measure) => measure.id === SPOKEN);
        if (bar) {
          seen.lines.add(bar.lines);
          seen.sizes.add(bar.size);
          if (!bar.fits) seen.unfit.push(bar);
        }
        return seen.lines.has(1) && seen.lines.has(2);
      }, LINE_WAIT_MS);
    } finally {
      await api("PUT", "/api/ai", { subtitles: { enabled: false } });
      await setWidth(BAR_WIDTH);
    }
    same(seen.unfit, [], "live lines that did not fit");
    same(seen.sizes.size, 1, `font sizes ${js([...seen.sizes])}`);
    return `${[...seen.sizes][0]} for one line and two`;
  });

  await step("an unknown theme is refused, and None brings back each element's own look", async () => {
    const refused = await api("PUT", "/api/project", { ...(await project()), theme: "y2k" }).catch((error) => error);
    check(refused instanceof Error && /400 .*unknown variant `y2k`/.test(refused.message), `y2k: ${js(refused)}`);
    same((await project()).theme, "windows-95", "the saved theme after the refusal");
    await choose("none");
    await save("none");
    same(await overlay.eval(`document.querySelectorAll("#overlay-root .sn-deco, #overlay-root .sn-card").length`), 0,
      "decorations left");
    same([await labelStyle(overlay, "fontFamily"), await labelStyle(overlay, "color")], [own.font, own.color],
      "the label's own font and color");
    same(await overlay.eval(panel(LABEL)), own.panel, "the label's panel");
  });
}
