// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// "Set up OBS for me" steps of the browser regression test (editor-browser.mjs), against a second sceneuron-dev of
// their own with simulated presence, whose simulated OBS has nothing set up yet, a new project file, and a simulated
// Stream Deck plugin whose pairing links go to a file: the project starts from the starter setup; one click adds the
// overlay, analyzes the simulated webcam, gives the simulated Mic/Aux the voice filter, and sends the Stream Deck the
// editor's own pairing link, says so in the tab and in a notification, and updates the camera and microphone lists;
// the link's code pairs as the plugin would trade it; and a second click finds it all in place and changes nothing.

import { readFileSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

import { waitFor } from "./cdp.mjs";
import { DevServer } from "./dev-server.mjs";
import { editorHelpers } from "./editor-steps.mjs";
import { expectedEditorError } from "./live-steps.mjs";

const { pairingLink } = createRequire(import.meta.url)("../web/editor/stream-deck-text.js");

const js = JSON.stringify;
const ADDED = [
  "Added Sceneuron overlay to the current scene.",
  "Camera analysis now watches Webcam (simulated).",
  "Voice control and subtitles now listen to Mic/Aux (simulated).",
  "Sent Sceneuron's plugin in the Stream Deck app a pairing link; your Stream Deck pairs in a moment.",
];
const PRESENT = [
  "Skipped the overlay: Sceneuron overlay is already in the current scene.",
  "Skipped the camera: Webcam (simulated) is already analyzed.",
  "Skipped the microphone: Mic/Aux (simulated) already has the Sceneuron Voice filter.",
  "Skipped the Stream Deck: it is already paired.",
];
const STARTER_DECKS = ["poker", "meeting", "classroom", "status"];

/** Runs the steps on a server and a tab of their own; `binary` is the sceneuron-dev to start and `work` a scratch folder. */
export async function runSetupSteps({ browser, binary, work, page: editorPage, step, same, token }) {
  const project = join(work, "setup-project.json");
  const links = join(work, "setup-stream-deck-links.txt");
  rmSync(links, { force: true });
  const server = new DevServer(binary, [
    "--port", "0", "--token", token, "--demo-presence", "--project", project, "--stream-deck-links", links,
  ]);
  let page = null;
  try {
    const base = await server.ready();
    const api = async (method, path) => {
      const response = await fetch(`${base}${path}`, { method, headers: { "X-Sceneuron-Token": token } });
      return response.json();
    };
    page = await browser.newPage({ width: 1280, height: 800 });
    await page.send("Page.bringToFront");
    await runSteps({ page, api, base, step, same, token, links });
    await step("the Set up OBS tab logged no console errors", () => same(page.errors(expectedEditorError), [], "errors"));
  } catch (error) {
    await step("the Set up OBS steps ran to the end", () => Promise.reject(error));
  } finally {
    await page?.send("Page.close").catch(() => {});
    page?.close();
    await server.stop();
    await editorPage.send("Page.bringToFront");
  }
}

async function runSteps({ page, api, base, step, same, token, links }) {
  const { q, check, toasts } = editorHelpers(page, api);
  const report = () => q(`[...document.querySelectorAll("#setup-obs-report li")].map((li) => li.textContent)`);
  const chip = (list) => page.text(`#${list} li:first-child .chip`);

  await step("a new project file starts from the starter setup", async () => {
    await page.goto(`${base}/editor?token=${token}`);
    await waitFor("the live connection", async () => (await page.text("#connection-chip")) === "Live");
    same((await api("GET", "/api/project")).decks.map((deck) => deck.id), STARTER_DECKS, "decks");
    const names = `[...document.querySelectorAll("#element-list [data-element-id]")].map((item) => item.dataset.elementId)`;
    await waitFor("the starter's elements", async () => (await q(`${names}.length`)) === 14);
    check((await q(names)).includes("subtitles-spoken"), "the subtitles bar is missing from the element list");
  });

  const sentLinks = () => readFileSync(links, "utf8").split("\n").filter(Boolean);

  await step("Set up OBS for me adds the overlay, camera analysis, and the voice filter, pairs the Stream Deck, and " +
    "says so", async () => {
    await page.click("#tab-camera");
    same(await q(`document.getElementById("setup-obs-report").hidden`), true, "the report before a click");
    await page.click("#setup-obs");
    await waitFor("the report", async () => (await report()).length === 4);
    same(await report(), ADDED, "the report");
    const summary =
      "Set up OBS: added the overlay, camera analysis, and the voice filter, and sent the Stream Deck a pairing link.";
    await waitFor("the notification", async () => (await toasts()).includes(summary));
    await waitFor("the analyzing chip", async () => (await chip("source-list")) === "Analyzing");
    await waitFor("the listening chip", async () => (await chip("mic-list")) === "Listening");
    same(await page.focused(), "setup-obs", "focus");
  });

  await step("the pairing link is the editor's own, and its code pairs as Sceneuron's plugin trades it", async () => {
    const sent = sentLinks();
    same(sent.length, 1, "pairing links");
    const link = new URL(sent[0]);
    const code = link.searchParams.get("code");
    same(sent[0], pairingLink(new URL(base).port, code), "the link");
    const before = (await api("GET", "/api/control/pairing")).pairings;
    const response = await fetch(`${base}/api/control/pair`, { method: "POST", body: js({ code }) });
    const reply = await response.json();
    check(response.ok && /^[0-9a-f]{32,}$/.test(reply.token), `pairing answered ${response.status}`);
    same((await api("GET", "/api/control/pairing")).pairings, before + 1, "pairings");
  });

  await step("a second click finds everything in place, the Stream Deck paired, and changes nothing", async () => {
    await page.click("#setup-obs");
    await waitFor("the report of what was in place", async () => js(await report()) === js(PRESENT));
    await waitFor("the notification", async () => (await toasts()).includes("OBS was already set up; nothing changed."));
    const sources = await api("GET", "/api/sources");
    same(sources.map((source) => source.analyzed), [true, false], "analyzed sources");
    const microphones = (await api("GET", "/api/audio-sources")).sources;
    same(microphones.map((microphone) => microphone.voice), [true, false], "microphones with the voice filter");
    same(sentLinks().length, 1, "pairing links");
  });
}
