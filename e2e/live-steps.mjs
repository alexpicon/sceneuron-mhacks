// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Editor steps of the browser regression test (editor-browser.mjs) that drive live state and the
// side panel: manual holds, effects, automation, simulated presence, tabs, rules, settings, a
// failed save, and the camera tab. They continue from the state the draft steps leave.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
const FAILED_SAVE = "simulated save failure";

/** Console errors caused on purpose: the failed-save step (400) and the assistant steps' refused saves (409). */
export const expectedEditorError = (entry) =>
  entry.source === "network" &&
  ((entry.url.endsWith("/api/project") && /status of (400|409)/.test(entry.text)) ||
    (entry.url.endsWith("/api/room") && /status of 502/.test(entry.text)));

/** Runs the live and side panel steps; `step(name, body)` reports each one. */
export async function runLiveSteps({ page, step, same, api, base, token }) {
  const helpers = editorHelpers(page, api);
  const { q, check, count, item, node, previewText, dirty, toasts, project, state, selected, clean, attribute } =
    helpers;
  const isGhost = (id) => q(`document.querySelector(${js(node(id))}).classList.contains("sn-ghost")`);
  const held = () => page.text("#live-held");
  const loggedEvents = () =>
    q(`[...document.querySelectorAll("#event-log li")].map((li) => [...li.children].map((span) => span.textContent))`);

  await step("manual holds show in the editor until released", async () => {
    await page.click(item("name-label"));
    await selected("Name label");
    await page.click('[data-hold-visible="false"]');
    await waitFor("the hide hold", async () => (await held()) === "Held: visible");
    same(await page.text('[data-badge="visible"]'), "held", "visible badge");
    check((await page.text(`${item("name-label")} .element-meta`)).includes("held"), "the list shows no hold");
    await waitFor("a ghost in the preview", () => isGhost("name-label"));
    await page.fill("hold-text", "On air");
    await page.click("#hold-text-button");
    await waitFor("the held text in the preview", async () => (await previewText("name-label")) === "On air");
    same(await page.text('[data-badge="text"]'), "held", "text badge");
    await page.click("#release-element");
    await waitFor("the release", async () => (await held()) === "Nothing held");
    await waitFor("the draft text in the preview", async () => (await previewText("name-label")) === "Hello stream");
    check(!(await isGhost("name-label")), "still a ghost after the release");
  });

  await step("effects play in the preview", async () => {
    await q(`document.getElementById("preview-root").addEventListener("animationstart", (event) =>
      (window.seenAnimations = (window.seenAnimations || []).concat(event.animationName)))`);
    await page.click('[data-effect="shake"]');
    await waitFor("the shake animation", () => q(`(window.seenAnimations || []).includes("sn-fx-shake")`));
    const live = (await state()).elements.find((element) => element.id === "name-label");
    same(live.effect?.effect, "shake", "live effect");
  });

  await step("automation pauses, stays paused through a save, and resumes", async () => {
    await page.click("#automation-toggle");
    await waitFor("the paused chip", async () => (await page.text("#automation-chip")) === "Automation paused");
    same(await page.text("#automation-toggle"), "Resume automation", "toggle label");
    const before = await state();
    same(before.automation_enabled, false, "server automation");
    await page.click("#save-button");
    const saved = (events) => events.some((event) => event.source === "editor" && event.at_ms >= before.now_ms);
    await waitFor("the save", async () => saved((await state()).events));
    same((await state()).automation_enabled, false, "server automation after saving");
    await page.click("#automation-toggle");
    await waitFor("the running chip", async () => (await page.text("#automation-chip")) === "Automation on");
    same((await state()).automation_enabled, true, "server automation after resuming");
  });

  await step("simulated presence runs the rules", async () => {
    const target = (await state()).presence.state === "away" ? "present" : "away";
    const rule = target === "away" ? 'ran "Show BRB when I leave"' : 'ran "Hide BRB when I return"';
    await page.click(`[data-simulate="${target}"]`);
    await waitFor("the simulated change and its rule in the log", async () => {
      const events = await loggedEvents();
      const index = events.findIndex(([, source, detail]) =>
        source === "test" && detail === `presence is now ${target}`
      );
      if (index > 0 && js(events[index - 1].slice(1)) === js(["rule", rule])) return true;
      throw new Error(js(events.slice(0, 4)));
    });
    check((await page.text(`${item("brb")} .element-meta`)).includes("rule"), "the list shows no rule values");
    return `simulated ${target}`;
  });

  await step("tabs switch with arrow keys", async () => {
    await q(`document.getElementById("tab-properties").focus()`);
    await page.key("ArrowRight");
    const tabs = await q(
      `["tab-rules", "panel-rules", "panel-properties"].map((id) => document.getElementById(id))` +
        `.map((node) => node.getAttribute("aria-selected") ?? node.hidden)`,
    );
    same(tabs, ["true", false, true], "rules tab, rules panel hidden, properties panel hidden");
    same(await page.focused(), "tab-rules", "focus");
    await page.key("End");
    same(await page.focused(), "tab-camera", "focus after End");
    await page.key("Home");
    await page.key("ArrowLeft");
    same(await page.focused(), "tab-camera", "focus after wrapping");
  });

  await step("rules are added, edited, saved, and deleted", async () => {
    const defaults = (await project()).rules.map((rule) => rule.id);
    const last = defaults.length;
    await page.click("#tab-rules");
    await page.click("#add-rule");
    await waitFor("the new rule's name focused", async () => (await page.focused()) === `rule-${last}-name`);
    await page.choose(`rule-${last}-action-0-type`, "effect");
    const effectChosen = `document.getElementById("rule-${last}-action-0-effect")?.value === "flash"`;
    await waitFor("the effect choice", () => q(effectChosen));
    await page.choose(`rule-${last}-when`, "present");
    await page.click("#rule-0-action-0-set-text");
    await page.fill("rule-0-action-0-text", "Back soon");
    await page.click("#save-button");
    await clean();
    const { rules } = await project();
    const effect = { type: "effect", element: "brb", effect: "flash" };
    const added = { id: "rule-1", name: "New rule", enabled: true, when: "present", actions: [effect] };
    same(rules[last], added, "new rule");
    same(rules[0].actions[0].props, { visible: true, text: "Back soon" }, "rule text");
    await page.click(`[data-rule-command="add-action"][data-rule-index="${last}"]`);
    await waitFor("the new action focused", async () => (await page.focused()) === `rule-${last}-action-1-type`);
    await page.click(`[data-rule-command="delete-action"][data-rule-index="${last}"][data-action-index="1"]`);
    const actions = `#rules-list article:nth-child(${last + 1}) .rule-action`;
    await waitFor("one action left", async () => (await count(actions)) === 1);
    same(await q(`document.activeElement.getAttribute("data-focus-key")`), "rule:rule-1:add-action", "focus");
    await page.click(`[data-rule-command="delete-rule"][data-rule-index="${last}"]`);
    await waitFor("the default rules", async () => (await count("#rules-list article")) === last);
    same(await page.focused(), "add-rule", "focus after deleting");
    await page.click("#save-button");
    await clean();
    same((await project()).rules.map((rule) => rule.id), defaults, "saved rules");
  });

  await step("settings are validated and saved", async () => {
    await page.click("#tab-settings");
    await page.fill("det-min_confidence", "2");
    const field = () =>
      Promise.all([attribute("#det-min_confidence", "aria-invalid"), page.text("#det-min_confidence-error")]);
    same(await field(), ["true", "Enter a number from 0.05 to 0.99."], "invalid entry");
    await page.fill("det-min_confidence", "0.8");
    same(await field(), [null, null], "valid entry");
    same(await dirty(), "Unsaved changes", "save state");
    await page.click("#save-button");
    await clean();
    same((await project()).detection.min_confidence, 0.8, "saved confidence");
  });

  await step("a failed save shows an alert that the keyboard dismisses", async () => {
    const body = Buffer.from(js({ error: FAILED_SAVE })).toString("base64");
    const responseHeaders = [{ name: "Content-Type", value: "application/json" }];
    const failPuts = ({ requestId, request }) => {
      const reply = request.method === "PUT"
        ? page.send("Fetch.fulfillRequest", { requestId, responseCode: 400, responseHeaders, body })
        : page.send("Fetch.continueRequest", { requestId });
      reply.catch((error) => console.log(`cannot answer the intercepted request: ${error.message}`));
    };
    page.on("Fetch.requestPaused", failPuts);
    await page.send("Fetch.enable", { patterns: [{ urlPattern: "*/api/project", requestStage: "Request" }] });
    try {
      await page.fill("canvas-width", "1280");
      await page.click("#save-button");
      await waitFor("the error alert", async () => (await toasts("toast-error")).includes(FAILED_SAVE));
      same(await attribute(".toast-error", "role"), "alert", "alert role");
      same([await dirty(), await page.text("#save-button")], ["Unsaved changes", "Save"], "save state");
      await q(`document.querySelector(".toast-error button").focus()`);
      await page.key("Enter", { text: "\r" });
      await waitFor("the alert dismissed", async () => (await count(".toast-error")) === 0);
      same(await page.focused(), "save-button", "focus after dismissing");
    } finally {
      await page.send("Fetch.disable");
      page.off("Fetch.requestPaused", failPuts);
    }
    await page.click("#discard-button");
    await clean();
    same(await page.value("canvas-width"), "1920", "canvas width after discarding");
  });

  await step("the camera tab lists sources, starts analysis, and adds the overlay", async () => {
    await page.click("#tab-camera");
    const noSource = "No camera is analyzed yet.";
    await waitFor("the preview's reason", async () => (await page.text("#camera-status")).startsWith(noSource));
    const names =
      `[...document.querySelectorAll("#source-list li > div > span:first-child")].map((span) => span.textContent)`;
    await waitFor("two sources", async () => (await q(`${names}.length`)) === 2);
    same(await q(names), ["Webcam (simulated)", "Capture card (simulated)"], "sources");
    await page.click('[data-source-name="Webcam (simulated)"]');
    const analyzing = "Sceneuron now analyzes Webcam (simulated).";
    await waitFor("the analysis notification", async () => (await toasts()).includes(analyzing));
    await waitFor(
      "the analyzing chip",
      async () => (await page.text("#source-list li:first-child .chip")) === "Analyzing",
    );
    same(await page.focused(), "refresh-sources", "focus");
    await page.click("#add-overlay");
    const added = "Added Sceneuron overlay to the current scene.";
    await waitFor("the overlay notification", async () => (await toasts()).includes(added));
    const overlayUrl = await page.value("overlay-url");
    check(overlayUrl.startsWith(`${base}/overlay#token=`) && !overlayUrl.includes(token), `overlay URL ${overlayUrl}`);
  });
}
