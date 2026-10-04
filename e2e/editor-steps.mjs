// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Editor steps of the browser regression test (editor-browser.mjs) that edit the draft: loading,
// selecting, adding and deleting elements, property edits, saving, the preview's pointer and
// keyboard moves, and discarding. They run against the default project.

import { sleep, waitFor } from "./cdp.mjs";

const js = JSON.stringify;

/** Shorthands for reading and driving the editor page. */
export function editorHelpers(page, api) {
  const q = (expression) => page.eval(expression);
  const helpers = {
    q,
    check(condition, message) {
      if (!condition) throw new Error(message);
    },
    count: (selector) => q(`document.querySelectorAll(${js(selector)}).length`),
    item: (id) => `#element-list [data-element-id=${js(id)}]`,
    node: (id) => `#preview-root [data-element-id=${js(id)}]`,
    previewText: (id) => page.text(`#preview-root [data-element-id=${js(id)}] .sn-text`),
    dirty: () => page.text("#dirty-indicator"),
    toasts: (kind = "toast") => q(`[...document.querySelectorAll(".${kind} p")].map((p) => p.textContent)`),
    position: async () => [Number(await page.value("prop-x")), Number(await page.value("prop-y"))],
    project: () => api("GET", "/api/project"),
    state: () => api("GET", "/api/state"),
    selected: (name) => waitFor(`${name} in the form`, async () => (await page.value("prop-name")) === name),
    clean: () => waitFor("all changes saved", async () => (await helpers.dirty()) === "All changes saved"),
    ctrlS: () => page.key("s", { code: "KeyS", modifiers: 2 }),
    attribute: (selector, name) => q(`document.querySelector(${js(selector)}).getAttribute(${js(name)})`),
  };
  return helpers;
}

/** Presses 20 px inside a preview element, moves through `path` offsets, and releases. */
async function dragInPreview(page, selector, path) {
  const rect = await page.eval(`document.querySelector(${js(selector)}).getBoundingClientRect().toJSON()`);
  const [x, y] = [rect.x + 20, rect.y + 20];
  await page.mouse("mouseMoved", x, y);
  await page.mouse("mousePressed", x, y, 1);
  for (const [dx, dy] of path) {
    await page.mouse("mouseMoved", x + dx, y + dy, 1);
    await sleep(20);
  }
  const [dx, dy] = path.at(-1) ?? [0, 0];
  await page.mouse("mouseReleased", x + dx, y + dy);
  await sleep(100);
}

/** Runs the draft steps; `step(name, body)` reports each one, and `same` compares values. */
export async function runDraftSteps({ page, step, same, api, base, token }) {
  const helpers = editorHelpers(page, api);
  const { q, check, count, item, node, previewText, dirty, toasts, position, project } = helpers;
  const { selected, clean, ctrlS, attribute } = helpers;
  const savedElement = async (id) => (await project()).elements.find((element) => element.id === id);
  const presence = /^(Present|Away), score 0\.\d\d$/;

  await step("the editor loads with live status and styles", async () => {
    await page.goto(`${base}/editor?token=${token}`);
    await waitFor("the live connection", async () => (await page.text("#connection-chip")) === "Live");
    same(await q("window.location.search + window.location.hash"), `#token=${token}`, "token moved to the fragment");
    await waitFor("five listed elements", async () => (await count("#element-list li")) === 5);
    await waitFor("five preview nodes", async () => (await count("#preview-root > .sn-el")) === 5);
    await waitFor("simulated presence", async () => presence.test(await page.text("#presence-text")));
    same(await page.text("#vision-chip"), "Camera running, 12 ms", "vision chip");
    same(await page.text("#automation-chip"), "Automation on", "automation chip");
    same(await dirty(), "All changes saved", "save state");
    const sheets = await q(`[...document.styleSheets].map((sheet) => sheet.cssRules.length > 0)`);
    same(sheets, Array(await count('link[rel="stylesheet"]')).fill(true), "every stylesheet loaded");
    same(await q("getComputedStyle(document.body).backgroundColor"), "rgb(28, 28, 30)", "page background");
    return await page.text("#presence-text");
  });

  await step("the element list selects with clicks and arrow keys", async () => {
    await page.click(item("name-label"));
    await selected("Name label");
    same(await attribute(item("name-label"), "aria-current"), "true", "current item");
    await page.key("ArrowDown");
    await selected("Camera frame");
    same(await q(`document.activeElement.getAttribute("data-element-id")`), "cam-frame", "focused item");
    await page.key("Home");
    await selected("Be right back");
  });

  await step("elements are added and deleted", async () => {
    await page.click('[data-add-kind="box"]');
    await selected("Box 1");
    same(await count("#element-list li"), 6, "listed elements");
    same(await page.text(`${item("box-1")} .element-meta`), "Box, visible, not saved", "new element state");
    same(await dirty(), "Unsaved changes", "save state");
    await waitFor("the new box in the preview", async () => (await count(node("box-1"))) === 1);
    await page.click("#delete-element");
    await waitFor("the box removed", async () => (await count("#element-list li")) === 5);
    await clean();
  });

  await step("property edits update the draft and the preview", async () => {
    await page.click(item("name-label"));
    await selected("Name label");
    await page.fill("prop-text", "Hello stream");
    await waitFor("the draft text in the preview", async () => (await previewText("name-label")) === "Hello stream");
    same(await dirty(), "Unsaved changes", "save state");
    same(await q("document.title"), "* Sceneuron editor", "page title");
    await q(`document.getElementById("prop-background-alpha").focus()`);
    await page.key("ArrowLeft");
    same(await page.text("#prop-background-output"), "86%", "alpha output");
    const fill = () => q(`document.querySelector(${js(`${node("name-label")} .sn-body`)}).style.backgroundColor`);
    await waitFor("the new alpha in the preview", async () => {
      const color = await fill();
      // Browsers round the alpha when they serialize it, so 0.859 may read back as 0.86.
      const alpha = Number(/^rgba\(155, 92, 255, ([\d.]+)\)$/.exec(color)?.[1]);
      if (Math.abs(alpha - 0xdb / 255) < 0.002) return true;
      throw new Error(color);
    });
    await page.fill("prop-x", "20000");
    same(await attribute("#prop-x", "aria-invalid"), "true", "invalid flag");
    same(await page.text("#prop-x-error"), "Enter a number from -10000 to 10000.", "field error");
    await ctrlS();
    const blocked = (texts) => texts.some((text) => text.startsWith("Fix the highlighted field"));
    await waitFor("the blocked save alert", async () => blocked(await toasts("toast-error")));
    same((await savedElement("name-label")).props.text, "Sceneuron", "the server's text");
    await q(`document.querySelector(".toast-error button").focus()`);
    await page.key("Escape");
    await waitFor("the alert dismissed", async () => (await count(".toast-error")) === 0);
    same(await page.focused(), "prop-x", "focus after dismissing");
    await page.fill("prop-x", "60");
    same(await attribute("#prop-x", "aria-invalid"), null, "invalid flag after fixing");
  });

  await step("Ctrl+S saves the draft", async () => {
    await ctrlS();
    await clean();
    check((await toasts()).includes("Saved."), "no Saved. notification");
    const { props } = await savedElement("name-label");
    same([props.text, props.background, props.x], ["Hello stream", "#9b5cffdb", 60], "saved props");
    same(await q("document.title"), "Sceneuron editor", "page title");
  });

  await step("a click in the preview selects without moving or marking changes", async () => {
    await page.click(item("brb"));
    await selected("Be right back");
    await dragInPreview(page, node("cam-frame"), []);
    await selected("Camera frame");
    await dragInPreview(page, node("cam-frame"), [[1, 1], [2, 2]]);
    same(await position(), [1260, 660], "position");
    same(await dirty(), "All changes saved", "save state");
  });

  await step("dragging in the preview moves the element", async () => {
    const [x, y] = await position();
    await dragInPreview(page, node("cam-frame"), [[10, 6], [20, 12], [30, 18], [40, 24], [50, 30]]);
    const scale = await q(`parseFloat(document.getElementById("preview").style.getPropertyValue("--preview-scale"))`);
    const expected = [Math.round(x + 50 / scale), Math.round(y + 30 / scale)];
    same(await position(), expected, `position at scale ${scale.toFixed(4)}`);
    same(await dirty(), "Unsaved changes", "save state");
  });

  await step("arrow keys in the preview nudge the selected element", async () => {
    const [x, y] = await position();
    await q(`document.getElementById("preview").focus()`);
    await page.key("ArrowLeft", { modifiers: 8 });
    await page.key("ArrowUp");
    same(await position(), [x - 10, y - 1], "position");
  });

  await step("discard restores the saved draft", async () => {
    await page.click("#discard-button");
    await clean();
    same(await position(), [1260, 660], "position");
    same(await page.focused(), "save-button", "focus");
  });
}
