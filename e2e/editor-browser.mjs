// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Browser regression test for the editor and overlay pages: starts sceneuron-dev with simulated
// presence and the simulated AI on a free port, drives the editor in headless Chrome through every
// panel, the Camera & mic tab's preview and microphone included, sets up a second server's simulated
// OBS with "Set up OBS for me" on a new project, which starts from the starter setup, checks that
// the overlay page is transparent and follows live changes, drives the AI tab, the assistant, the
// Cards tab with its cards' hand signals and its Stream Deck pairing link, and its Room section,
// with a room on a stand-in SpacetimeDB, voice, subtitles, overlay themes, the Graphite theme's icons
// and its on-air pill and timer for a simulated OBS, fun reactions with their emoji floods, the live
// description bar, and typed voice commands with what can be said, and fails on any console error
// or Content Security Policy violation.
// Prints PASS or FAIL per step and exits nonzero on a failure.
//
// Usage: node e2e/editor-browser.mjs [--dev-binary PATH] [--chrome PATH] [--work-dir DIR] [--keep]

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { isDeepStrictEqual, parseArgs } from "node:util";

import { runAiSteps } from "./editor-ai-steps.mjs";
import { runAssistantSteps } from "./assistant-steps.mjs";
import { runCameraMicSteps } from "./camera-mic-steps.mjs";
import { runCardEditorSteps } from "./card-editor-steps.mjs";
import { waitFor } from "./cdp.mjs";
import { Browser, findChrome } from "./chrome.mjs";
import { DevServer, devBinary } from "./dev-server.mjs";
import { runGraphiteSteps } from "./graphite-steps.mjs";
import { runDraftSteps } from "./editor-steps.mjs";
import { runHandSignalSteps } from "./hand-signal-steps.mjs";
import { runLiveDescriptionSteps } from "./live-description-steps.mjs";
import { expectedEditorError, runLiveSteps } from "./live-steps.mjs";
import { decodePng, meanAlpha } from "./png.mjs";
import { runReactionSteps } from "./reaction-steps.mjs";
import { runRoomEditorSteps } from "./room-editor-steps.mjs";
import { RoomStandIn } from "./room-stand-in.mjs";
import { runSetupSteps } from "./setup-steps.mjs";
import { runStreamDeckEditorSteps } from "./streamdeck-editor-steps.mjs";
import { runSubtitleSteps } from "./subtitle-steps.mjs";
import { runThemeSteps } from "./theme-steps.mjs";
import { runVoiceCommandSteps } from "./voice-command-steps.mjs";
import { runVoiceSteps } from "./voice-steps.mjs";

const { values: options } = parseArgs({
  options: {
    "dev-binary": { type: "string" },
    chrome: { type: "string" },
    "work-dir": { type: "string" },
    keep: { type: "boolean" },
  },
});
const binary = devBinary(options["dev-binary"]);
// Only a directory this run created is deleted afterwards; a given one may hold other files.
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-browser-")) : resolve(options["work-dir"]);
mkdirSync(work, { recursive: true });
const token = "editor-browser-token-0123456789abcdef";
const js = JSON.stringify;

let failures = 0;
async function step(name, body) {
  try {
    const detail = await body();
    console.log(`PASS ${name}${detail ? ` - ${detail}` : ""}`);
  } catch (error) {
    failures += 1;
    console.log(`FAIL ${name} - ${error.message}`);
  }
}

function same(actual, expected, what) {
  if (!isDeepStrictEqual(actual, expected)) throw new Error(`${what}: expected ${js(expected)}, got ${js(actual)}`);
}

const context = { step, same, token };
let server;
let browser;
let overlay;
const roomStandIn = new RoomStandIn();

async function finish(code) {
  clearTimeout(deadline);
  const failed = code !== 0;
  for (const [name, page] of [["editor", context.page], ["overlay", overlay]]) {
    if (failed && page) {
      await page.screenshot().then((png) => writeFileSync(join(work, `${name}.png`), png), () => {});
    }
  }
  await browser?.close().catch((error) => console.log(`cannot stop Chrome: ${error.message}`));
  await server?.stop();
  await roomStandIn.stop().catch((error) => console.log(`cannot stop the room stand-in: ${error.message}`));
  if (failed || options.keep) console.log(`Work directory: ${work}`);
  else if (ownWork) rmSync(work, { recursive: true, force: true });
  process.exit(code);
}

const deadline = setTimeout(() => {
  console.log("FAIL the run took longer than three minutes");
  finish(1);
}, 180_000);
process.on("SIGINT", () => finish(130));

async function api(method, path, body) {
  const response = await fetch(`${server.base}${path}`, {
    method,
    headers: { "X-Sceneuron-Token": token, "Content-Type": "application/json" },
    body: body === undefined ? undefined : js(body),
  });
  const json = await response.json();
  if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
  return json;
}

async function runOverlaySteps() {
  overlay = await browser.newPage({ width: 960, height: 540 });
  await overlay.send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });
  const q = (expression) => overlay.eval(expression);
  const labelText = () => overlay.text('[data-element-id="name-label"] .sn-text');

  await step("the overlay page is transparent and draws the saved layout", async () => {
    await overlay.goto(`${server.base}/overlay?token=${token}`);
    await waitFor("five overlay nodes", async () => (await q(`document.querySelectorAll(".sn-el").length`)) === 5);
    const styles = await q(`[document.documentElement, document.body].map((node) => getComputedStyle(node))
      .map((style) => [style.backgroundColor, style.overflow])`);
    same(styles, [["rgba(0, 0, 0, 0)", "hidden"], ["rgba(0, 0, 0, 0)", "hidden"]], "page background and overflow");
    const scroll = await q(`[document.documentElement.scrollWidth, document.documentElement.scrollHeight]`);
    same(scroll, [960, 540], "scroll size");
    const root = await q(`document.getElementById("overlay-root").style.transform`);
    same(root, "translate(0px, 0px) scale(0.5)", "fit transform");
    const rect = await q(`document.querySelector('[data-element-id="name-label"]').getBoundingClientRect().toJSON()`);
    same([rect.x, rect.y, rect.width, rect.height], [30, 450, 320, 55], "name label box");
    same(await labelText(), "Hello stream", "saved text");
    await new Promise((resolve) => setTimeout(resolve, 800));
    const image = decodePng(await overlay.screenshot());
    const empty = meanAlpha(image, { x0: 0, y0: 0, x1: 0.25, y1: 0.25 });
    const label = meanAlpha(image, { x0: 0.05, y0: 0.85, x1: 0.3, y1: 0.92 });
    if (empty !== 0 || label < 150) throw new Error(`alpha ${empty} where empty, ${label} on the name label`);
    return `alpha ${empty} where empty, ${Math.round(label)} on the name label`;
  });

  await step("the overlay follows live overrides and effects", async () => {
    await q(`document.addEventListener("animationstart", (event) =>
      (window.seenAnimations = (window.seenAnimations || []).concat(event.animationName)))`);
    await api("POST", "/api/override", { element: "name-label", props: { text: "Overlay live" } });
    await waitFor("the held text", async () => (await labelText()) === "Overlay live");
    await api("POST", "/api/effect", { element: "name-label", effect: "pop" });
    await waitFor("the pop animation", () => q(`(window.seenAnimations || []).includes("sn-fx-pop")`));
    await api("POST", "/api/release", { element: null });
    await waitFor("the saved text", async () => (await labelText()) === "Hello stream");
  });
}

try {
  server = new DevServer(binary, ["--port", "0", "--token", token, "--demo-presence", "--demo-ai"]);
  await server.ready();
  browser = await Browser.launch({ chrome: findChrome(options.chrome), workDir: work });
  Object.assign(context, { page: await browser.newPage({ width: 1280, height: 800 }), api, base: server.base });
  await runDraftSteps(context);
  await runLiveSteps(context);
  await runCameraMicSteps(context);
  await runSetupSteps({ ...context, browser, binary, work });
  await runOverlaySteps();
  await runAiSteps({ ...context, overlay });
  await runAssistantSteps(context);
  await runCardEditorSteps(context);
  await runHandSignalSteps(context);
  await runStreamDeckEditorSteps({ ...context, browser });
  await runRoomEditorSteps({ ...context, roomServer: await roomStandIn.start() });
  await runVoiceSteps({ ...context, overlay });
  await runSubtitleSteps({ ...context, overlay });
  await runThemeSteps({ ...context, overlay });
  await runGraphiteSteps({ ...context, overlay, server });
  await runReactionSteps({ ...context, overlay, work });
  await runLiveDescriptionSteps({ ...context, overlay });
  await runVoiceCommandSteps(context);
  await step("no console errors or Content Security Policy violations", () => {
    const pages = [context.page, overlay].filter(Boolean);
    const errors = pages.flatMap((page) => page.errors(expectedEditorError));
    const violations = pages.flatMap((page) => page.entries).filter((entry) => /CSP|Content Security/.test(entry.text));
    if (errors.length || violations.length) throw new Error(js([...errors, ...violations]));
  });
} catch (error) {
  failures += 1;
  console.log(`FAIL the run stopped - ${error.message}`);
}
console.log(failures ? `${failures} step(s) failed` : "All steps passed");
await finish(failures ? 1 : 0);
