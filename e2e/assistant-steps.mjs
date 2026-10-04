// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Assistant steps of the browser regression test (editor-browser.mjs), against the dev server's simulated
// AI: the setup wizard, stopping a proposal, previewing a plan and applying it over unsaved edits, a plan
// refused because the project changed, and a save refused for the same reason. They run after the AI steps,
// with consent given.

import { sleep, waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;

/** Saves a change to the project the way another editor would, at the server's current revision. */
async function saveElsewhere({ base, token }, change) {
  const headers = { "X-Sceneuron-Token": token, "Content-Type": "application/json" };
  const state = await (await fetch(`${base}/api/state`, { headers })).json();
  const project = await (await fetch(`${base}/api/project`, { headers })).json();
  change(project);
  const response = await fetch(`${base}/api/project`, {
    method: "PUT",
    headers: { ...headers, "If-Match": String(state.project_revision) },
    body: js(project),
  });
  if (!response.ok) throw new Error(`the other save failed with ${response.status}`);
}

/** Runs the assistant steps; `step(name, body)` reports each one. */
export async function runAssistantSteps(context) {
  const { page, step, same, api } = context;
  const helpers = editorHelpers(page, api);
  const { q, check, item, dirty, toasts, project, selected, clean } = helpers;
  const element = async (id) => (await project()).elements.find((candidate) => candidate.id === id);
  const planLines = (where) =>
    q(`[...document.querySelectorAll(${js(`${where} .plan-op`)})].map((li) => li.textContent)`);
  const showing = (selector) => q(`Boolean(document.querySelector(${js(selector)}))`);
  const propose = async (text) => {
    await page.fill("assistant-request", text);
    await page.key("Enter", { text: "\r" });
  };

  await step("the setup wizard proposes a setup and applies it", async () => {
    await page.click("#setup-open");
    await waitFor("the dialog", () => q(`document.getElementById("setup-dialog").open`));
    same(await page.focused(), "setup-request", "focus in the dialog");
    await page.click("[data-example]");
    await page.click('[data-setup="next"]');
    same(await q(`document.getElementById("setup-camera").disabled`), false, "camera choice with consent");
    await page.click("#setup-camera");
    await page.click('[data-setup="propose"]');
    await waitFor("the proposed setup", () => showing("#setup-result .plan"), 8000);
    const lines = await planLines("#setup-result");
    check(lines.some((line) => line.startsWith("Change condition Showing something")), js(lines));
    check(/^Simulated setup/.test(await page.text("#setup-result .plan-summary")), "no setup summary");
    await page.click('#setup-result [data-plan-action="apply"]');
    await waitFor("the dialog closed", async () => !(await q(`document.getElementById("setup-dialog").open`)));
    same(await page.focused(), "setup-open", "focus after closing");
    await waitFor("the new title", async () => (await element("name-label")).props.text === "Keyboard reviews, live");
    await clean();
    return `${lines.length} changes`;
  });

  await step("a proposal can be stopped, and a plan applied over unsaved edits after asking", async () => {
    await propose("Say hi when I wave");
    await waitFor("the stop button", () => showing('[data-plan-action="cancel"]'));
    await page.click('[data-plan-action="cancel"]');
    await waitFor("stopped", async () => {
      const text = await page.text("#assistant-result");
      if (/^Stopped waiting/.test(text)) return true;
      throw new Error(text);
    });
    await sleep(2000);
    same(await helpers.count("#assistant-result .plan"), 0, "plans shown after stopping");
    await page.click("#assistant-propose");
    await waitFor("the plan", () => showing("#assistant-result .plan"), 8000);
    const lines = await planLines("#assistant-result");
    check(lines.includes("Add condition Waving: The person waves a hand at the camera"), js(lines));
    const rule = "Add rule Say hi when I wave: when Waving turns on, show Wave hello and bounce it";
    check(lines.includes(rule), js(lines));
    await waitFor("the plan preview", () => q(`document.getElementById("preview").classList.contains("is-plan-preview")`));
    // The preview marks planned elements when it next draws, a frame after the plan arrives.
    await waitFor("an element marked as planned in the preview", async () => (await helpers.count("#preview-root .is-planned")) > 0);
    await page.click("#tab-properties");
    await page.click(item("brb"));
    await selected("Be right back");
    await page.fill("prop-name", "Edited here");
    same(await dirty(), "Unsaved changes", "save state");
    await page.click('#assistant-result [data-plan-action="apply"]');
    await waitFor("the confirmation", () => showing('[data-plan-action="confirm"]'));
    same(await q(`document.activeElement.getAttribute("data-plan-action")`), "confirm", "focus");
    await page.click('[data-plan-action="confirm"]');
    await waitFor("applied", async () => (await toasts()).includes("Plan applied: 4 changes."));
    await clean();
    check(!(await q(`document.getElementById("preview").classList.contains("is-plan-preview")`)), "plan preview left on");
    same((await element("brb")).name, "Be right back", "the unsaved edit was replaced");
    check((await project()).conditions.some((condition) => condition.id === "waving"), "no waving condition");
  });

  await step("a plan made before another save is refused and can be proposed again", async () => {
    await propose("Make the Camera frame blue");
    await waitFor("the plan", () => showing("#assistant-result .plan"), 8000);
    await saveElsewhere(context, (project) => {
      project.elements.find((candidate) => candidate.id === "cam-frame").props.corner_radius = 4;
    });
    await page.click('#assistant-result [data-plan-action="apply"]');
    await waitFor("the refusal", async () => /^Not applied: the project changed/.test(
      await page.text("#assistant-result .confirm-box p"),
    ));
    await page.click('#assistant-result [data-plan-action="again"]');
    await waitFor("a new plan", () => showing('#assistant-result [data-plan-action="apply"]'), 8000);
    await page.click('#assistant-result [data-plan-action="apply"]');
    await waitFor("applied", async () => (await toasts()).includes("Plan applied: 1 change."));
    await clean();
    same((await element("cam-frame")).props.background, "#3b82f6ee", "the planned color");
    same((await element("cam-frame")).props.corner_radius, 4, "the other save was kept");
  });

  await step("saving over a newer project shows a reload banner instead of overwriting", async () => {
    await page.click("#tab-properties");
    await page.click(item("cam-frame"));
    await selected("Camera frame");
    await page.fill("prop-name", "Renamed here");
    await saveElsewhere(context, (project) => {
      project.elements.find((candidate) => candidate.id === "name-label").name = "Renamed elsewhere";
    });
    const banner = () => page.text("#banner-text");
    await waitFor("the warning", async () => /^The project was changed in another editor/.test(await banner()));
    await page.click("#save-button");
    await waitFor("the refusal", async () => /^Not saved: /.test(await banner()));
    same((await element("cam-frame")).name, "Camera frame", "the server's name");
    same(await page.text("#banner-retry"), "Reload", "banner action");
    await page.click("#banner-retry");
    const label = `${item("name-label")} .element-name`;
    await waitFor("the reload", async () => (await page.text(label)) === "Renamed elsewhere");
    await clean();
    same(await q(`document.getElementById("banner").hidden`), true, "banner after reloading");
    same(await page.focused(), "save-button", "focus after reloading");
  });
}
