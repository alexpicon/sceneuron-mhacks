// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Fun reaction steps of the browser regression test (editor-browser.mjs): the Settings tab's fun reactions section,
// on by default, with every preset, its Try it button, and the camera switches of the tongue, unavailable with the
// simulated AI, and the facepalm, unavailable with a simulated camera that sees no hands; a rule whose action floods
// the overlay; the flood drawn on the overlay page above every element, covering the whole canvas and then removing
// itself; the tongue's Try it button, which plays at once on the overlay and in the preview; and the email preset,
// whose banner shows large in the middle of the overlay above its emojis and every element. They run last, and save
// overlay screenshots of the flood as flood-1.png to flood-4.png and of the email's banner, at the canvas's full
// 1920x1080, as email-1.png to email-4.png in the work directory.

import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { sleep, waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";
import { decodePng } from "./png.mjs";

const SHOTS_MS = [250, 900, 1600, 2300];
const EMAIL_SHOTS_MS = [250, 900, 1800, 2800];
const BACKDROP = { r: 28, g: 30, b: 44, a: 1 };
const TRANSPARENT = { r: 0, g: 0, b: 0, a: 0 };
const PRESETS = [
  "tongue", "email", "mind-blown", "ship-it", "coffee", "absolutely-right", "more-tokens",
  "works-on-my-gpu", "meatproxy", "clanker",
];
const EMAIL_LINE = "📧 This meeting could have been an email 🤦";

/** Which cells of a `columns` by `rows` grid over the image hold a pixel that differs from `backdrop`. */
function coveredCells(image, columns, rows, backdrop) {
  const covered = new Set();
  for (let y = 0; y < image.height; y += 2) {
    for (let x = 0; x < image.width; x += 2) {
      const at = (y * image.width + x) * 4;
      const [r, g, b] = image.pixels.subarray(at, at + 3);
      if (Math.abs(r - backdrop.r) + Math.abs(g - backdrop.g) + Math.abs(b - backdrop.b) > 60) {
        covered.add(Math.floor((y / image.height) * rows) * columns + Math.floor((x / image.width) * columns));
      }
    }
  }
  return covered;
}

/** Runs the fun reaction steps; `overlay` is the overlay page and `work` the directory for screenshots. */
export async function runReactionSteps({ page, overlay, step, same, api, work }) {
  const { q, check, count, project, state, clean } = editorHelpers(page, api);
  const o = (expression) => overlay.eval(expression);
  const layer = `document.querySelector("#overlay-root > .sn-flood")`;
  const banner = `document.querySelector("#overlay-root > .sn-flood > .sn-flood-banner")`;
  const floodNonce = async () => (await state()).flood?.nonce ?? null;
  const cameraStatus = (id) => page.text(`#reaction-camera-status-${id}`);

  const offered = "the Settings tab offers every reaction, on by default, with the tongue's and the facepalm's cameras";
  await step(offered, async () => {
    await page.send("Page.bringToFront");
    await page.click("#tab-settings");
    same(await q(`document.getElementById("reactions-enabled").checked`), true, "the switch");
    same(await page.value("reactions-cooldown_ms"), "3", "the cooldown in seconds");
    const tries = await q(`[...document.querySelectorAll("[data-reaction-try]")].map((b) => b.dataset.reactionTry)`);
    same(tries, PRESETS, "the Try it buttons");
    const switches = `[...document.querySelectorAll("[data-reaction-camera]")]`;
    const cameras = await q(`${switches}.map((input) => [input.dataset.reactionCamera, input.checked])`);
    same(cameras, [["tongue", true], ["email", true]], "the camera switches");
    const simulated = "Unavailable: the simulated AI cannot see a tongue; it needs a real vision model.";
    await waitFor("the simulated AI's status", async () => (await cameraStatus("tongue")) === simulated);
    const handless = "Unavailable: this camera analysis cannot see hands.";
    await waitFor("the simulated camera's status", async () => (await cameraStatus("email")) === handless);
    await page.click("#reaction-camera-email");
    await waitFor("the email's camera off", async () => (await cameraStatus("email")).startsWith("Off: the camera"));
    await page.click("#reactions-enabled");
    await waitFor("the off status", async () => (await page.text("#reactions-status")).startsWith("Off:"));
    same(await q(`document.getElementById("reaction-camera-tongue").disabled`), true, "a camera switch while off");
    await page.fill("reactions-cooldown_ms", "45");
    await page.click("#save-button");
    await clean();
    const off = { enabled: false, cooldown_ms: 45000, camera_off: ["email"], voice_off: [] };
    same((await project()).reactions, off, "saved reactions");
    await page.click("#reactions-enabled");
    await page.click("#reaction-camera-email");
    await page.fill("reactions-cooldown_ms", "3");
    await page.click("#save-button");
    await clean();
    const restored = { enabled: true, cooldown_ms: 3000, camera_off: [], voice_off: [] };
    same((await project()).reactions, restored, "restored reactions");
  });

  let ruleIndex = -1;
  await step("a rule's flood covers the overlay above every element and then removes itself", async () => {
    const presence = (await state()).presence.state;
    const trigger = presence === "present" ? "away" : "present";
    ruleIndex = (await project()).rules.length;
    await page.click("#tab-rules");
    await page.click("#add-rule");
    await waitFor("the new rule", async () => (await page.focused()) === `rule-${ruleIndex}-name`);
    await page.choose(`rule-${ruleIndex}-action-0-type`, "flood");
    const choice = `document.getElementById("rule-${ruleIndex}-action-0-flood")`;
    await waitFor("the reaction choice", () => q(`${choice} !== null`));
    same(await count(`#rule-${ruleIndex}-action-0-element`), 0, "a flood names no element");
    same(await count(`#rule-${ruleIndex}-action-0-flood option`), PRESETS.length, "every preset to choose from");
    await page.choose(`rule-${ruleIndex}-when`, trigger);
    // Read before the rule exists: simulated presence may also change by itself and play the rule's flood.
    const before = await floodNonce();
    await page.click("#save-button");
    await clean();
    same((await project()).rules[ruleIndex].actions, [{ type: "flood", flood: "tongue" }], "the saved action");

    await overlay.send("Page.bringToFront");
    await overlay.send("Emulation.setDefaultBackgroundColorOverride", { color: BACKDROP });
    await api("POST", "/api/simulate", { presence: trigger });
    await waitFor("the flood on the overlay", () => o(`${layer} !== null`), 3000);
    const started = Date.now();
    check((await floodNonce()) !== before, "the rule played no new flood");
    const box = await o(`${layer}.getBoundingClientRect().toJSON()`);
    same([box.x, box.y, box.width, box.height], [0, 0, 960, 540], "the flood's box on the 960x540 page");
    same(await o(`${layer} === document.getElementById("overlay-root").lastElementChild`), true, "drawn last");
    same(await o(`getComputedStyle(${layer}).zIndex`), "2147483647", "above every element");
    same(await o(`${layer}.querySelectorAll("span").length`), 94, "emojis");
    same(await o(`${banner}`), null, "the tongue has no banner");
    const covered = new Set();
    for (const [index, at] of SHOTS_MS.entries()) {
      await sleep(Math.max(0, at - (Date.now() - started)));
      const png = await overlay.screenshot();
      writeFileSync(join(work, `flood-${index + 1}.png`), png);
      for (const cell of coveredCells(decodePng(png), 8, 4, BACKDROP)) covered.add(cell);
    }
    await overlay.send("Emulation.setDefaultBackgroundColorOverride", { color: TRANSPARENT });
    await waitFor("the flood to remove itself", () => o(`${layer} === null`), 3000);
    const elapsed = Date.now() - started;
    check(covered.size >= 28, `the flood reached only ${covered.size} of 32 parts of the canvas`);
    return `${covered.size} of 32 parts of the canvas covered, gone after ${elapsed} ms`;
  });

  const previewFlood = `document.querySelector("#preview-root > .sn-flood")`;
  const testFlood = "the tongue's Try it plays at once on the overlay and in the preview, whatever the cooldown";
  await step(testFlood, async () => {
    await page.send("Page.bringToFront");
    await page.click(`[data-rule-command="delete-rule"][data-rule-index="${ruleIndex}"]`);
    await page.click("#save-button");
    await clean();
    await page.click("#tab-settings");
    const before = await floodNonce();
    await page.click("#reaction-try-tongue");
    await waitFor("the flood in the preview", () => q(`${previewFlood} !== null`));
    await sleep(1000);
    writeFileSync(join(work, "editor-flood.png"), await page.screenshot());
    check((await floodNonce()) > before, "no new flood within the cooldown");
    await waitFor("the flood on the overlay", () => o(`${layer} !== null`), 3000);
    const log = await q(`[...document.querySelectorAll("#event-log li")].map((li) => li.textContent)`);
    const logged = log.some((line) => line.includes("flooded the overlay with 😜 😛 🤪 😝"));
    check(logged, `no flood in ${JSON.stringify(log)}`);
    await waitFor("the preview's flood to end", () => q(`${previewFlood} === null`));
    await waitFor("the overlay's flood to end", () => o(`${layer} === null`), 3000);
  });

  await step("the email's Try it shows its banner large in the middle of the overlay, above everything", async () => {
    const before = await floodNonce();
    // The overlay comes to the front first, so the screenshots follow the banner from its start; the editor's
    // button is clicked from its script, as the tongue's button was clicked by the mouse. The editor's preview,
    // which draws only while its page is in front, played the tongue's flood above.
    await overlay.send("Page.bringToFront");
    await overlay.send("Emulation.setDefaultBackgroundColorOverride", { color: BACKDROP });
    await q(`document.getElementById("reaction-try-email").click()`);
    await waitFor("the banner on the overlay", () => o(`${banner} !== null`), 3000);
    const started = Date.now();
    check((await floodNonce()) > before, "no new flood");
    same((await state()).flood.flood, "email", "the preset played");
    same(await o(`${banner}.textContent`), EMAIL_LINE, "the banner's line");
    same(await o(`${layer}.lastElementChild === ${banner}`), true, "the banner drawn above the emojis");
    same(await o(`${layer} === document.getElementById("overlay-root").lastElementChild`), true, "above every element");
    const emojis = await o(`[...${layer}.querySelectorAll("span")].map((span) => span.textContent)`);
    same(emojis.length, 94, "emojis");
    const mail = ["📧", "✉️", "📨", "📬", "🤦"];
    check(emojis.every((emoji) => mail.includes(emoji)), `emojis ${JSON.stringify(emojis)}`);
    for (const [index, at] of EMAIL_SHOTS_MS.entries()) {
      await sleep(Math.max(0, at - (Date.now() - started)));
      const clip = { x: 0, y: 0, width: 960, height: 540, scale: 2 };
      const { data } = await overlay.send("Page.captureScreenshot", { format: "png", clip });
      writeFileSync(join(work, `email-${index + 1}.png`), Buffer.from(data, "base64"));
      if (index === 1 || index === 2) {
        same(await o(`getComputedStyle(${banner}).opacity`), "1", `the banner's opacity after ${at} ms`);
      }
      if (index === 1) {
        const box = await o(`${banner}.getBoundingClientRect().toJSON()`);
        const [middleX, middleY] = [box.x + box.width / 2, box.y + box.height / 2];
        const centered = Math.abs(middleX - 480) < 3 && Math.abs(middleY - 270) < 3;
        check(centered, `the banner's middle at ${middleX}, ${middleY}`);
        check(box.width > 960 * 0.6 && box.x > 0 && box.x + box.width < 960, `the banner's box ${JSON.stringify(box)}`);
      }
    }
    await overlay.send("Emulation.setDefaultBackgroundColorOverride", { color: TRANSPARENT });
    await waitFor("the banner to remove itself", () => o(`${layer} === null`), 3000);
    const elapsed = Date.now() - started;
    const log = await q(`[...document.querySelectorAll("#event-log li")].map((li) => li.textContent)`);
    const line = 'flooded the overlay with 📧 ✉️ 📨 📬 🤦 and "This meeting could have been an email"';
    check(log.some((entry) => entry.includes(line)), `no email in ${JSON.stringify(log)}`);
    return `gone after ${elapsed} ms; email-1.png to email-4.png saved`;
  });
}
