// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The starter setup and "Set up OBS for me" in the end-to-end proof's fresh OBS (run.mjs): a fresh install starts from
// the starter project and the starter AI settings, which hold no key and no consent; and once earlier steps' overlay,
// camera analysis, and voice filters are taken away, POST /api/setup adds the overlay to the current scene and the
// Sceneuron Voice filter to Mic/Aux when this OBS has one, but leaves the camera clip alone, since a media file is no
// camera device, and says to pick the source; it sends no Stream Deck pairing link, since the scratch home has no
// Sceneuron plugin for Stream Deck, which also shows that the plugin looks in the scratch home and never pairs the
// user's own Stream Deck; once the clip is picked, a second call finds it all in place. The checks record failures as
// failed checks instead of throwing them, so the rest of the proof still runs.

import { readFileSync } from "node:fs";

const js = JSON.stringify;
const STARTER_DECKS = ["poker", "meeting", "classroom", "status"];
const CAMERA_FILTER = "sceneuron_camera_filter";
const VOICE_FILTER = "sceneuron_voice_filter";

/** Whether every field named like a key or token in `value`, at any depth, is empty, and every consent is false. */
export function holdsNoSecretOrConsent(value) {
  if (Array.isArray(value)) return value.every(holdsNoSecretOrConsent);
  if (value === null || typeof value !== "object") return true;
  return Object.entries(value).every(([name, item]) => {
    const lower = name.toLowerCase();
    if (lower.includes("key") || lower.includes("token")) return item === "";
    if (lower === "consent") return item === false;
    return holdsNoSecretOrConsent(item);
  });
}

/** Checks that the fresh OBS started from the starter setup; `aiFile` is its AI settings file. */
export async function checkStarter({ api, check, aiFile }) {
  try {
    const project = await api("GET", "/api/project");
    const decks = project.decks.map((deck) => deck.id);
    check("Starter", "a fresh install starts from the starter project, with its decks, subtitles bars, and rules",
      js(decks) === js(STARTER_DECKS) && project.elements.some((e) => e.id === "subtitles-spoken") &&
        project.rules.length === 6, `decks ${js(decks)}`);
    const saved = JSON.parse(readFileSync(aiFile, "utf8"));
    check("Starter", "the starter AI settings it writes hold no key, no token, and no consent",
      holdsNoSecretOrConsent(saved) && !saved.enabled && !saved.voice.enabled && !saved.subtitles.enabled);
    const ai = await api("GET", "/api/ai");
    check("Starter", "they load as Gemini, voice always on, and subtitles into Japanese, waiting for consent",
      ai.provider === "gemini" && ai.voice.mode === "always_on" && js(ai.subtitles.targets) === js(["ja"]) &&
        !ai.consent && !ai.voice.consent && !ai.subtitles.consent);
  } catch (error) {
    check("Starter", "the starter checks ran to the end", false, error.message);
  }
}

async function filterKinds(ws, sourceName) {
  const { filters } = await ws.request("GetSourceFilterList", { sourceName });
  return filters.map((filter) => [filter.filterKind, filter.filterName]);
}

/**
 * Takes away the overlay `overlayName`, camera analysis, and the voice filters, then sets OBS up through the editor
 * API and checks what it did; `scene` is the current scene and `camera` the camera clip's source, which is analyzed
 * again before the scenario ends.
 */
export async function runSetupScenario(t) {
  try {
    await setupSteps(t);
  } catch (error) {
    t.check("Setup", "the setup scenario ran to the end", false, error.message);
  }
}

async function setupSteps({ api, ws, check, scene, camera, overlayName }) {
  const special = await ws.request("GetSpecialInputs");
  const mic = ["mic1", "mic2", "mic3", "mic4"].map((key) => special[key]).find(Boolean) ?? null;
  await ws.request("RemoveInput", { inputName: overlayName });
  for (const sourceName of [camera, mic].filter(Boolean)) {
    for (const [kind, filterName] of await filterKinds(ws, sourceName)) {
      if (kind === CAMERA_FILTER || kind === VOICE_FILTER) {
        await ws.request("RemoveSourceFilter", { sourceName, filterName });
      }
    }
  }
  const analyzed = (await api("GET", "/api/sources")).filter((source) => source.analyzed);
  const voiced = (await api("GET", "/api/audio-sources")).sources.filter((source) => source.voice);
  check("Setup", "before Set up OBS for me, no source is analyzed and no microphone has the voice filter",
    analyzed.length === 0 && voiced.length === 0, js([...analyzed, ...voiced].map((source) => source.name)));

  const report = await api("POST", "/api/setup");
  const items = (await ws.request("GetSceneItemList", { sceneName: scene })).sceneItems;
  const has = async (sourceName, wanted) => (await filterKinds(ws, sourceName)).some(([kind]) => kind === wanted);
  check("Setup", "Set up OBS for me adds the Sceneuron Overlay to the current scene",
    report.overlay.outcome === "added" && items.some((item) => item.sourceName === report.overlay.source),
    js(report.overlay));
  check("Setup", "it analyzes no media file, even the only video source, and says to pick the source",
    report.camera.outcome === "skipped" && report.camera.reason.includes("Camera source") &&
      !(await has(camera, CAMERA_FILTER)), js(report.camera));
  if (mic) {
    check("Setup", "it gives Mic/Aux the Sceneuron Voice filter",
      report.microphone.outcome === "added" && report.microphone.source === mic && (await has(mic, VOICE_FILTER)),
      js(report.microphone));
  } else {
    check("Setup", "without Mic/Aux in this OBS, it skips the microphone and says why",
      report.microphone.outcome === "skipped" && report.microphone.reason.includes("Mic/Aux"), js(report.microphone));
  }
  check("Setup", "it pairs no Stream Deck from the scratch home, which lacks the plugin, and says how to install it",
    report.stream_deck?.outcome === "skipped" && report.stream_deck.reason.includes("double-click"),
    js(report.stream_deck));

  const clip = (await api("GET", "/api/sources")).find((source) => source.name === camera);
  await api("POST", "/api/sources/attach", { uuid: clip.uuid });
  const again = await api("POST", "/api/setup");
  // OBS lists a removed source's item until its next video frame prunes it, so the overlay taken away above may be
  // in `items` and gone from `after`: nothing added means no item in `after` that `items` lacked.
  const after = (await ws.request("GetSceneItemList", { sceneName: scene })).sceneItems;
  const known = new Set(items.map((item) => item.sceneItemId));
  const present = ["overlay", "camera", ...(mic ? ["microphone"] : [])].every((part) => again[part].outcome === "present");
  check("Setup", "once the clip is picked, a second click finds everything in place and adds nothing",
    present && again.camera.source === camera && again.stream_deck?.outcome === "skipped" &&
      after.every((item) => known.has(item.sceneItemId)), js(again));
}
