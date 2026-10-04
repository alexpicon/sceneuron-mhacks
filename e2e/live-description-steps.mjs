// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Live description steps of the browser regression test (editor-browser.mjs), against the dev server's simulated
// presence (--demo-presence), whose live description says "1 person" while presence is present, about ten and a half
// seconds, and "Nobody in view" for the rest of each sixteen: Add live description puts the ready-made bar in the
// draft once, and once saved the overlay shows the on-device line, with the simulated AI's caption and its age below
// it when the AI has one, and follows the line as it changes, as the Camera & mic tab does. They run last, so the bar
// changes nothing earlier steps check.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const BAR = "live-description";
const LINES = ["1 person", "Nobody in view"];
// The longest the simulated line stays the same, and a little more, so a change of the line is always seen.
const CHANGE_WAIT_MS = 13_000;

/** What the bar shows for a snapshot: the line, then the caption with its age, leaving out an empty one. */
export function barText(state) {
  const caption = state.scene && state.scene.caption ? `${state.scene.caption} (${state.scene.age})` : "";
  return [state.seen, caption].filter(Boolean).join("\n");
}

/** Runs the live description steps; `overlay` is the overlay page the overlay steps opened. */
export async function runLiveDescriptionSteps({ page, overlay, step, same, api }) {
  const { q, check, count, item, clean, project, state, selected } = editorHelpers(page, api);
  const overlayText = () => overlay.text(`[data-element-id="${BAR}"] .sn-text`);

  await step("Add live description puts the ready-made bar in the draft once, with its help beside it", async () => {
    await page.send("Page.bringToFront");
    await page.click("#tab-properties");
    const help = await page.text("#live-description-help");
    const told = /free, and nothing is sent/.test(help) && /each caption costs a look/.test(help);
    check(told, `help ${JSON.stringify(help)}`);
    await page.click("#add-live-description");
    await selected("Live description");
    same(await page.value("prop-text"), "{{seen}}\n{{scene:age}}", "the bar's text");
    same(await count(item(BAR)), 1, "the bar in the element list");
    const before = (await q("document.querySelectorAll('#element-list li').length"));
    await page.click("#add-live-description");
    await waitFor("the note that it is there", async () =>
      (await page.text("#announcer")) === "Live description is already in this project, and is selected now.");
    same(await q("document.querySelectorAll('#element-list li').length"), before, "no second bar");
    await page.click("#save-button");
    await clean();
    const saved = (await project()).elements.find((element) => element.id === BAR);
    check(saved && saved.props.visible === true && saved.props.align === "left", `saved bar ${JSON.stringify(saved)}`);
  });

  await step("the overlay and the Camera & mic tab follow the simulated camera's live description", async () => {
    const shown = async () => {
      const expected = barText(await state());
      return LINES.some((line) => expected.startsWith(line)) && (await overlayText()) === expected ? expected : null;
    };
    const first = await waitFor("the line on the overlay", shown);
    const firstLine = first.split("\n")[0];
    const next = await waitFor("the next line on the overlay", async () => {
      const text = await shown();
      return text && text.split("\n")[0] !== firstLine ? text : null;
    }, CHANGE_WAIT_MS);
    await page.click("#tab-camera");
    const tab = async () => {
      const line = (await state()).seen;
      return line && (await page.text("#camera-live-description")) === `Live description: ${line}` ? line : null;
    };
    const inTab = await waitFor("the line in the Camera & mic tab", tab);
    await page.click("#tab-properties");
    return `${JSON.stringify(first)}, then ${JSON.stringify(next)}; the tab said ${JSON.stringify(inTab)}`;
  });
}
