// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// End-to-end proof in a real OBS: load the plugin, which starts from the starter setup, analyze,
// preview, and listen to a camera clip, drive the overlay automatically and manually, let a scripted
// vision model judge AI conditions, show a simulated subtitle line, set up OBS in one click, pause
// automation, and restore everything after a restart.
//
// Usage: node e2e/run.mjs --plugin dist/macos/sceneuron.plugin [--work-dir DIR] [--keep]
// A directory given with --work-dir is never deleted; only a directory the test created is.

import { randomBytes } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { runAiScenario } from "./ai-steps.mjs";
import { runCameraMicScenario } from "./camera-mic-obs-steps.mjs";
import { checkCardsRestored, runCardScenario } from "./card-steps.mjs";
import { createChecks, editorApi, element, elementBox, overlayAlpha, waitFor } from "./harness.mjs";
import { ObsInstance, makeClip, obsConfigDir, prepareHome } from "./obs-instance.mjs";
import { ObsWebSocket } from "./obs-ws.mjs";
import { checkStarter, runSetupScenario } from "./starter-obs-steps.mjs";
import { runSubtitleScenario } from "./subtitle-obs-steps.mjs";

const { values: options } = parseArgs({
  options: { plugin: { type: "string" }, "work-dir": { type: "string" }, keep: { type: "boolean" } },
});
const repo = resolve(import.meta.dirname, "..");
const pluginBundle = resolve(options.plugin ?? join(repo, "dist", "macos", "sceneuron.plugin"));
const work = options["work-dir"] ? resolve(options["work-dir"]) : mkdtempSync(join(tmpdir(), "sceneuron-e2e-"));
const home = join(work, "home");
const ports = { websocket: 4460, editor: 4471 };
const token = randomBytes(16).toString("hex");
const SCENE = "Sceneuron E2E";
const CAMERA = "Camera clip";
const BRB_BOX = { x0: 0.33, y0: 0.44, x1: 0.67, y1: 0.56 };

const { results, check } = createChecks();
const api = editorApi(ports.editor, token);
const presence = async (wanted) => {
  const state = await api("GET", "/api/state");
  return state.presence.state === wanted ? state : null;
};

let active = null;

async function launch() {
  const obs = new ObsInstance(home);
  active = obs;
  obs.start();
  const ws = await waitFor("obs-websocket", () => ObsWebSocket.connect(`ws://127.0.0.1:${ports.websocket}`), 60_000);
  // obs-websocket answers "not ready" until the OBS window finishes loading the scene collection.
  await waitFor("OBS ready", () => ws.request("GetSceneList").then(() => true), 60_000);
  await waitFor("Sceneuron editor server", () => api("GET", "/api/state"), 30_000);
  return { obs, ws };
}

async function firstRun() {
  const clip = makeClip({
    workDir: work,
    fixturesDir: join(repo, "crates", "sceneuron-vision", "tests", "fixtures"),
    faceSeconds: 8,
    emptySeconds: 12,
  });
  const { obs, ws } = await launch();
  check("R1", "OBS log shows the module loaded", obs.latestLog().includes("[sceneuron] loaded version"));
  const inputs = await ws.request("GetInputKindList", { unversioned: true });
  check("R1", "overlay source type registered", inputs.inputKinds.includes("sceneuron_overlay"));
  const filters = await ws.request("GetSourceFilterKindList");
  check("R1", "camera filter type registered", filters.sourceFilterKinds.includes("sceneuron_camera_filter"));
  check("Voice", "voice filter type registered", filters.sourceFilterKinds.includes("sceneuron_voice_filter"));
  const { hotkeys } = await ws.request("GetHotkeyList");
  check("Voice", "push-to-talk hotkey registered", hotkeys.includes("sceneuron.voice.talk"));
  const vendorState = await ws.vendor("GetState");
  check("R1", "obs-websocket vendor answers", typeof vendorState.state?.revision === "number");
  await checkStarter({ api, check, aiFile: join(obsConfigDir(home), "plugin_config", "sceneuron", "ai.json") });

  await ws.request("CreateScene", { sceneName: SCENE });
  await ws.request("SetCurrentProgramScene", { sceneName: SCENE });
  await ws.request("CreateInput", {
    sceneName: SCENE,
    inputName: CAMERA,
    inputKind: "ffmpeg_source",
    inputSettings: { local_file: clip, is_local_file: true, looping: true, restart_on_activate: false },
  });
  const sources = await waitFor("camera in source list", async () =>
    (await api("GET", "/api/sources")).find((s) => s.name === CAMERA));
  await api("POST", "/api/sources/attach", { uuid: sources.uuid });
  const attached = await ws.request("GetSourceFilterList", { sourceName: CAMERA });
  check("R4", "editor attached the analysis filter to an existing source",
    attached.filters.some((f) => f.filterKind === "sceneuron_camera_filter"));
  await ws.request("CreateInput", {
    sceneName: SCENE,
    inputName: "Second camera",
    inputKind: "ffmpeg_source",
    inputSettings: { local_file: clip, is_local_file: true, looping: true, restart_on_activate: false },
  });
  const second = await waitFor("second camera in source list", async () =>
    (await api("GET", "/api/sources")).find((s) => s.name === "Second camera"));
  await api("POST", "/api/sources/attach", { uuid: second.uuid });
  const moved = await api("GET", "/api/sources");
  await api("POST", "/api/sources/attach", { uuid: sources.uuid });
  const back = await api("GET", "/api/sources");
  const analyzed = (list, name) => list.find((s) => s.name === name)?.analyzed;
  check("R4", "analysis follows one camera at a time",
    analyzed(moved, "Second camera") && !analyzed(moved, CAMERA) && analyzed(back, CAMERA) && !analyzed(back, "Second camera"));
  await ws.request("CreateSourceFilter", {
    sourceName: "Second camera",
    filterName: "Voice",
    filterKind: "sceneuron_voice_filter",
  });
  const voiced = await ws.request("GetSourceFilterList", { sourceName: "Second camera" });
  check("Voice", "a voice filter attaches to a source with audio",
    voiced.filters.some((f) => f.filterKind === "sceneuron_voice_filter"));
  await ws.request("RemoveInput", { inputName: "Second camera" });
  const { name: overlayName } = await api("POST", "/api/overlay");
  const items = await ws.request("GetSceneItemList", { sceneName: SCENE });
  check("R2", "editor added an overlay to the current scene", items.sceneItems.some((i) => i.sourceName === overlayName));

  const running = await waitFor("vision running", async () => {
    const state = await api("GET", "/api/state");
    return state.vision.state === "running" && state.vision.latency_ms > 0 ? state : null;
  });
  check("R5", "face detector is running on camera frames", true, `latency ${running.vision.latency_ms} ms`);
  await runCameraMicScenario({ api, ws, check, waitFor, camera: CAMERA, editorPort: ports.editor, token });
  const present = await waitFor("presence present", () => presence("present"));
  check("R5", "face in view -> present, BRB hidden", !element(present, "brb").props.visible);
  const away = await waitFor("presence away", () => presence("away"));
  check("R5", "nobody in view -> away, BRB shown by rule", element(away, "brb").props.visible);
  // obs-browser only paints visible pages, so make sure the test scene is the one on air.
  await ws.request("SetCurrentProgramScene", { sceneName: SCENE });
  // The overlay's browser starts asynchronously (slowly on a fresh profile), so wait until the
  // always-visible name label is drawn before judging the presence-driven card.
  const labelBox = elementBox(await api("GET", "/api/project"), "name-label");
  const labelAlpha = await waitFor("overlay page rendering", async () => {
    const alpha = await overlayAlpha(ws, overlayName, labelBox);
    return alpha > 80 ? alpha : null;
  }, 60_000).catch(() => 0);
  check("R2", "overlay page renders inside OBS", labelAlpha > 80, `label alpha ${labelAlpha.toFixed(1)}`);
  const shownAlpha = await waitFor("away with the BRB card on screen", async () => {
    if (!(await presence("away"))) return null;
    const alpha = await overlayAlpha(ws, overlayName, BRB_BOX);
    return alpha > 80 ? alpha : null;
  }, 45_000).catch(() => 0);
  check("R2", "overlay renders the BRB card", shownAlpha > 80, `mean alpha ${shownAlpha.toFixed(1)}`);
  const hiddenAlpha = await waitFor("present with the BRB card gone", async () => {
    if (!(await presence("present"))) return null;
    const alpha = await overlayAlpha(ws, overlayName, BRB_BOX);
    return alpha < 10 ? { alpha } : null;
  }, 45_000).then((r) => r.alpha).catch(() => 255);
  check("R2", "overlay hides the BRB card again", hiddenAlpha < 10, `mean alpha ${hiddenAlpha.toFixed(1)}`);

  // One clip segment from a fresh "present": exactly one change (to away) may happen, no flicker.
  const since = (await api("GET", "/api/state")).presence.since_ms;
  const before = await ws.request("GetStats");
  await waitFor("one clip segment", async () => {
    const state = await api("GET", "/api/state");
    return state.now_ms >= since + 19_000 ? state : null;
  }, 30_000);
  const window = (await api("GET", "/api/state")).events.filter((e) => e.source === "camera" && e.at_ms > since);
  check("False triggers", "no repeated or flickering presence changes", window.length === 1,
    window.map((e) => e.detail).join("; ") || "none");
  // Measure only the steady window, not OBS startup (which includes the browser engine starting).
  const stats = await ws.request("GetStats");
  const frames = stats.renderTotalFrames - before.renderTotalFrames;
  const skipped = (stats.renderSkippedFrames - before.renderSkippedFrames) / Math.max(1, frames);
  check("Resource use", "OBS keeps rendering on time while analyzing", stats.averageFrameRenderTime < 33.3 && skipped < 0.01,
    `render ${stats.averageFrameRenderTime.toFixed(2)} ms/frame, skipped ${(skipped * 100).toFixed(2)}%, ` +
    `OBS CPU ${stats.cpuUsage.toFixed(1)}%, ${frames} frames at ${stats.activeFps.toFixed(1)} fps`);

  const manual = await api("POST", "/api/override", { element: "name-label", props: { text: "Manual E2E" } });
  check("R3", "editor manual change applies immediately", element(manual, "name-label").props.text === "Manual E2E");
  const vendorHeld = await ws.vendor("SetOverride", { element: "brb", props: { visible: true } });
  const exact = JSON.parse(vendorHeld.state_json ?? "{}");
  check("R3", "vendor responses carry the exact state, override lists included",
    exact.elements?.find((e) => e.id === "brb")?.overridden?.includes("visible") && vendorHeld.state?.elements?.some(
      (e) => e.id === "brb" && e.overridden === "visible"));
  await waitFor("away while BRB is held", () => presence("away"));
  const held = await waitFor("present while BRB is held", () => presence("present"));
  check("R6", "held manual override survives automatic rules", element(held, "brb").props.visible);
  await ws.vendor("ReleaseOverrides", {});
  const released = await api("GET", "/api/state");
  check("R6", "release returns control to automation", !element(released, "brb").props.visible);

  await ws.request("TriggerHotkeyByName", { hotkeyName: "sceneuron.element.name-label" });
  const hidden = await api("GET", "/api/state");
  check("R3", "an element hotkey shows or hides it immediately",
    !element(hidden, "name-label").props.visible && element(hidden, "name-label").overridden.includes("visible"));
  await ws.vendor("ReleaseOverrides", { element: "name-label" });

  await ws.request("TriggerHotkeyByName", { hotkeyName: "sceneuron.automation" });
  const paused = await api("GET", "/api/state");
  check("R3", "hotkey pauses automation", paused.automation_enabled === false);
  const awayPaused = await waitFor("away while paused", () => presence("away"));
  check("R6", "paused automation leaves the BRB hidden", !element(awayPaused, "brb").props.visible);
  const resumed = (await ws.vendor("SetAutomation", { enabled: true })).state;
  check("R6", "resuming applies rules for the current presence", resumed.automation_enabled && element(resumed, "brb").props.visible);
  check("R1", "vendor PresenceChanged events reach obs-websocket clients",
    ws.events.some((e) => e.eventType === "VendorEvent" && e.eventData?.eventType === "PresenceChanged"));

  await runCardScenario({ api, ws, check, waitFor, element });
  await runSubtitleScenario({ api, ws, check, waitFor, element, overlayAlpha, overlayName });
  await runAiScenario({ api, ws, check, waitFor, overlayAlpha, overlayName, element, token, editorPort: ports.editor });
  await runSetupScenario({ api, ws, check, scene: SCENE, camera: CAMERA, overlayName });

  const project = await api("GET", "/api/project");
  project.elements.find((e) => e.id === "name-label").props.text = "Persisted E2E";
  project.rules.push({
    id: "e2e-away-label",
    name: "E2E away label",
    enabled: true,
    when: "away",
    actions: [{ type: "set_props", element: "name-label", props: { text: "Away E2E" } }],
  });
  await api("PUT", "/api/project", project);
  ws.close();
  const stopped = await obs.stop();
  check("R7", "OBS quits cleanly with the plugin loaded", stopped.clean, `exit ${stopped.code ?? stopped.signal}`);
  check("R1", "no recovered internal errors were logged", !obs.latestLog().includes("recovered from an internal error"));
  const saved = readFileSync(join(obsConfigDir(home), "plugin_config", "sceneuron", "project.json"), "utf8");
  check("R7", "project saved to disk", saved.includes("Persisted E2E"));
}

async function secondRun() {
  const { obs, ws } = await launch();
  const project = await api("GET", "/api/project");
  check("R7", "layout restored after restart",
    project.elements.find((e) => e.id === "name-label").props.text === "Persisted E2E");
  check("R7", "rules restored after restart", project.rules.some((r) => r.id === "e2e-away-label"));
  await checkCardsRestored({ api, ws, check, waitFor });
  const ai = await api("GET", "/api/ai");
  check("AI", "AI settings survive a restart, and the key never leaves the plugin",
    ai.provider === "openai-compatible" && ai.consent === true && ai.enabled === false && ai.api_key === undefined);
  check("AI", "the assistant's condition is restored", project.conditions.some((c) => c.id === "waving"));
  const filters = await ws.request("GetSourceFilterList", { sourceName: CAMERA });
  check("R7", "camera analysis restored with the scene collection",
    filters.filters.some((f) => f.filterKind === "sceneuron_camera_filter"));
  const awayAgain = await waitFor("presence after restart", () => presence("away"), 60_000);
  check("R7", "camera rules work again after restart, the saved custom rule included",
    element(awayAgain, "brb").props.visible && element(awayAgain, "name-label").props.text === "Away E2E");
  ws.close();
  const stopped = await obs.stop();
  check("R7", "second OBS run quits cleanly", stopped.clean, `exit ${stopped.code ?? stopped.signal}`);
  check("R1", "module unloads at exit", obs.latestLog().includes("[sceneuron] unloaded"));
}

try {
  prepareHome({ home, pluginBundle, websocketPort: ports.websocket, editorPort: ports.editor, token });
  await firstRun();
  await secondRun();
} catch (error) {
  check("E2E", "scenario completed", false, error.message);
} finally {
  await active?.stop();
}
const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length}/${results.length} checks passed; work dir ${work}`);
if (!options.keep && !options["work-dir"] && failed.length === 0) rmSync(work, { recursive: true, force: true });
process.exit(failed.length ? 1 : 0);
