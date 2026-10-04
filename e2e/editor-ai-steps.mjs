// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// AI steps of the browser regression test (editor-browser.mjs), against the dev server's simulated AI
// (--demo-ai): the AI tab and its consent gate, a new condition wired to a rule, a text placeholder,
// scene understanding turning a condition on and changing the overlay, and the assistant's camera choice
// before and once there is consent. They run after the overlay steps.

import { sleep, waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
// What the consent says Google may do with what the simulated AI's Gemini provider would be sent.
const FREE_TIER =
  "Google: With a free-tier key (a Google project without billing), Google may use what is sent, camera images " +
  "included, to improve its products, and people may review it. A key from a project with billing is not used " +
  "that way.";

/** Runs the AI steps; `overlay` is the overlay page the overlay steps opened. */
export async function runAiSteps({ page, overlay, step, same, api }) {
  const helpers = editorHelpers(page, api);
  const { q, check, previewText, dirty, project, clean } = helpers;
  const hidden = (id) => q(`document.getElementById(${js(id)}).hidden`);
  const selectedLabel = (id) => q(`document.getElementById(${js(id)}).selectedOptions[0].textContent`);
  const overlayText = (id) => overlay.text(`[data-element-id=${js(id)}] .sn-text`);
  let conditionIndex = 0;

  await step("the AI tab shows the simulated AI, which needs consent before it watches", async () => {
    // The overlay steps opened the overlay in a newer tab; a hidden tab gets no frames to handle input.
    await page.send("Page.bringToFront");
    await page.click("#tab-ai");
    await waitFor("the setup chip", async () => (await page.text("#ai-chip")) === "AI needs setup");
    same(await hidden("ai-simulated"), false, "simulated AI note hidden");
    same(await q(`document.getElementById("ai-enabled").disabled`), true, "watch switch before consent");
    same(await page.text("#ai-key-status"), "Key saved (...demo)", "key status");
    same(await page.value("ai-api-key"), "", "key field");
    same(await selectedLabel("ai-provider"), "Google Gemini", "provider");
    const where = "Where: generativelanguage.googleapis.com (Google Gemini).";
    same(await page.text("#ai-consent-text li:nth-child(2)"), where, "consent");
    same(await page.text("#ai-consent-text li:nth-child(3)"), FREE_TIER, "free-tier disclosure");
    same(await q(`document.getElementById("assistant-camera").disabled`), true, "assistant camera before consent");
    // Plans go to the provider without camera consent, so the assistant says what Google may do with them.
    const before = "Needs your consent in the AI tab. Plans work without it. With a free-tier key (a Google project " +
      "without billing), Google may use your request and project to improve its products, and people may review it. " +
      "A key from a project with billing is not used that way.";
    same(await page.text("#assistant-camera-note"), before, "assistant camera note before consent");
  });

  await step("a new condition gets its id from its name", async () => {
    conditionIndex = (await project()).conditions.length;
    const base = `condition-${conditionIndex}`;
    await page.click("#add-condition");
    await waitFor("the new name focused", async () => (await page.focused()) === `${base}-name`);
    await page.type("Arms up");
    const token = `#conditions-list article:nth-child(${conditionIndex + 1}) p.hint code`;
    await waitFor("the id following the name", async () => (await page.text(token)) === "{{arms-up}}");
    same(await page.focused(), `${base}-name`, "focus after the id changed");
    same(await page.text(`#conditions-list article:nth-child(${conditionIndex + 1}) [data-live="badge"]`),
      "Not saved yet", "badge");
    await page.fill(`${base}-prompt`, "The person raises both arms above the head.");
    same(await helpers.attribute(`#${base}-prompt`, "aria-invalid"), null, "prompt flag");
    same(await dirty(), "Unsaved changes", "save state");
  });

  await step("a text placeholder is inserted and previewed", async () => {
    await page.click('[data-add-kind="text"]');
    await page.click("#tab-properties");
    await helpers.selected("Text 1");
    await page.fill("prop-text", "Hi {{arm-up}}");
    await waitFor("the unknown placeholder warning", async () => !(await hidden("prop-text-unknown")));
    const warning = "{{arm-up}} is not a condition, so it shows nothing on stream.";
    same(await page.text("#prop-text-unknown"), warning, "warning");
    await page.fill("prop-text", "AI sees: ");
    same(await hidden("prop-text-unknown"), true, "warning after fixing");
    check((await helpers.count('#text-placeholders [data-token="{{arms-up}}"]')) === 1, "no chip for the condition");
    await page.click('#text-placeholders [data-token="{{scene}}"]');
    same(await page.value("prop-text"), "AI sees: {{scene}}", "text after inserting");
    same(await page.focused(), "prop-text", "focus after inserting");
    await waitFor("the stand-in", async () => (await previewText("text-1")) === "AI sees: [Scene description]");
  });

  await step("a rule starts when the condition turns on", async () => {
    await page.click("#tab-rules");
    const rule = (await project()).rules.length;
    await page.click("#add-rule");
    await waitFor("the new rule", async () => (await page.focused()) === `rule-${rule}-name`);
    await page.type("Hype on arms up");
    await page.choose(`rule-${rule}-when`, "on:arms-up");
    same(await selectedLabel(`rule-${rule}-when`), "When Arms up turns on", "trigger label");
    await page.choose(`rule-${rule}-action-0-element`, "text-1");
    await page.click("#save-button");
    await clean();
    const saved = await project();
    const condition = saved.conditions.find((candidate) => candidate.id === "arms-up");
    same(condition?.prompt, "The person raises both arms above the head.", "saved prompt");
    same(saved.rules[rule].when, { condition: "arms-up", state: "on" }, "saved trigger");
    same(saved.rules[rule].actions[0].element, "text-1", "rule target");
    same(saved.elements.find((element) => element.id === "text-1").props.text, "AI sees: {{scene}}", "saved text");
  });

  await step("watching turns a condition on and the overlay follows", async () => {
    await page.click("#tab-ai");
    await page.click("#ai-consent");
    await waitFor("the switch", async () => !(await q(`document.getElementById("ai-enabled").disabled`)));
    await page.click("#ai-enabled");
    await waitFor("watching", async () => /^AI (watching|thinking)$/.test(await page.text("#ai-chip")));
    const badge = '#conditions-list article:first-child [data-live="badge"]';
    await waitFor("Showing something on", async () => {
      if ((await page.text(badge)) === "On") return true;
      await page.click("#ai-look-now");
      await sleep(500);
      return false;
    }, 15_000);
    const shown = "Now showing: Mechanical keyboard";
    await waitFor("the item on the overlay", async () => (await overlayText("now-showing")) === shown);
    check(/keyboard/.test(await overlayText("text-1")), `caption: ${await overlayText("text-1")}`);
    check(/keyboard/.test(await page.text('#conditions-list article:first-child [data-live="reason"]')), "no reason");
    const logged = await q(`[...document.querySelectorAll('#event-log li[data-source="ai"] .event-what')]
      .map((node) => node.textContent)`);
    check(logged.includes("Showing something turned on"), `AI events: ${js(logged)}`);
    // Usage counters arrive as throttled telemetry, a moment after the snapshot that turned the condition on.
    await waitFor("the looks counted", async () => Number(await page.text("#ai-calls")) >= 2);
    return await overlayText("text-1");
  });

  await step("with consent, the assistant's camera choice says what a free-tier key lets Google do", async () => {
    const note = "Sends one still image with your request. With a free-tier key (a Google project without billing), " +
      "Google may use what is sent to improve its products, and people may review it. A key from a project with " +
      "billing is not used that way.";
    same(await page.text("#assistant-camera-note"), note, "assistant camera note");
  });
}
