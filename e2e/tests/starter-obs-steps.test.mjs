// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the proof's starter and setup steps against fakes: telling settings that hold a key, a token, or a
// consent from the starter's, which holds none; the starter checks against the bundled starter AI settings; and the
// setup scenario, which takes away what earlier steps set up, sets OBS up, and checks what it did, with and without
// Mic/Aux, and records a broken setup, a setup that analyzes the clip, one that sends a Stream Deck pairing link from
// the scratch home, or a throw as failed checks.

import assert from "node:assert/strict";
import { join } from "node:path";
import { test } from "node:test";

import { checkStarter, holdsNoSecretOrConsent, runSetupScenario } from "../starter-obs-steps.mjs";

const STARTER_AI = join(import.meta.dirname, "..", "..", "crates", "sceneuron-ai", "starter", "ai.json");
const CAMERA = "Camera clip";
const OVERLAY = "Sceneuron Overlay";

function recorder() {
  const results = [];
  const check = (requirement, name, ok, detail = "") => results.push({ requirement, name, ok: Boolean(ok), detail });
  return { results, check };
}

test("settings with a key, a token, or a consent anywhere are told apart from ones without", () => {
  const clean = { api_key: "", voice: { consent: false, tokens: "" }, list: [{ elevenlabs_api_key: "" }] };
  assert.equal(holdsNoSecretOrConsent(clean), true);
  assert.equal(holdsNoSecretOrConsent({ api_key: "AQ.secret" }), false);
  assert.equal(holdsNoSecretOrConsent({ voice: { Control_Token: "abc" } }), false);
  assert.equal(holdsNoSecretOrConsent({ subtitles: { consent: true } }), false);
  assert.equal(holdsNoSecretOrConsent({ list: [{ jev_api_key: null }] }), false, "a key must be an empty string");
});

/** An editor API that answers the project and AI settings given. */
function starterApi(project, ai) {
  return async (method, path) => (path === "/api/project" ? project : ai);
}

const STARTER_PROJECT = {
  decks: ["poker", "meeting", "classroom", "status"].map((id) => ({ id })),
  elements: [{ id: "brb" }, { id: "subtitles-spoken" }],
  rules: Array.from({ length: 6 }, (_, index) => ({ id: `rule-${index}` })),
};
const STARTER_VIEW = {
  provider: "gemini",
  consent: false,
  voice: { mode: "always_on", consent: false },
  subtitles: { targets: ["ja"], consent: false },
};

test("the bundled starter passes the starter checks", async () => {
  const { results, check } = recorder();
  await checkStarter({ api: starterApi(STARTER_PROJECT, STARTER_VIEW), check, aiFile: STARTER_AI });
  assert.deepEqual(results.map((r) => r.ok), [true, true, true], JSON.stringify(results));
});

test("a default project, consent given, or a missing settings file fail the starter checks", async () => {
  const { results, check } = recorder();
  const consented = { ...STARTER_VIEW, voice: { mode: "always_on", consent: true } };
  await checkStarter({ api: starterApi({ ...STARTER_PROJECT, decks: [] }, consented), check, aiFile: STARTER_AI });
  assert.deepEqual(results.map((r) => r.ok), [false, true, false]);
  const missing = recorder();
  await checkStarter({ api: starterApi(STARTER_PROJECT, STARTER_VIEW), check: missing.check, aiFile: "/no/ai.json" });
  assert.deepEqual(missing.results.map((r) => r.ok), [true, false]);
  assert.equal(missing.results[1].name, "the starter checks ran to the end");
});

/**
 * A fake OBS after the earlier proof steps: the clip has camera analysis and the voice filter, the overlay is in the
 * scene, and `mic` names Mic/Aux when there is one. Its POST /api/setup adds what is missing, as the plugin does,
 * except camera analysis on the clip, a media file, which it starts only with `analyzesClip`, and a Stream Deck
 * pairing link, which the scratch home's missing plugin stops unless `pairsStreamDeck`; a `broken` one adds nothing.
 * With `prunesLate`, a removed input keeps its scene item, as OBS does until its next video frame, until the item list
 * is read; a setup that `countsRemoved` takes that item for the overlay in place.
 */
function fakeObs({
  mic = "Mic/Aux",
  broken = false,
  analyzesClip = false,
  prunesLate = false,
  countsRemoved = false,
  pairsStreamDeck = false,
} = {}) {
  const filters = { [CAMERA]: [["sceneuron_camera_filter", "Analysis"], ["sceneuron_voice_filter", "Voice"]] };
  if (mic) filters[mic] = [];
  let lastId = 0;
  const item = (sourceName) => ({ sourceName, sceneItemId: ++lastId, removed: false });
  const items = [item(CAMERA), item(OVERLAY)];
  const calls = [];
  const has = (source, kind) => (filters[source] ?? []).some(([found]) => found === kind);
  const step = (outcome, source, reason = "") => ({ outcome, source, reason });
  const analyze = () => filters[CAMERA].push(["sceneuron_camera_filter", "Analysis"]);
  const addOverlay = () => {
    const name = items.some(({ sourceName }) => sourceName === OVERLAY) ? `${OVERLAY} 2` : OVERLAY;
    items.push(item(name));
    return step("added", name);
  };
  const setUp = () => {
    if (broken) return { overlay: step("skipped", "", "no scene"), camera: step("skipped", ""), microphone: {} };
    const counts = ({ sourceName, removed }) => sourceName.startsWith(OVERLAY) && (countsRemoved || !removed);
    const placed = items.find(counts);
    const overlay = placed ? step("present", placed.sourceName) : addOverlay();
    const camera = has(CAMERA, "sceneuron_camera_filter")
      ? step("present", CAMERA)
      : analyzesClip
        ? (analyze(), step("added", CAMERA))
        : step("skipped", "", "OBS has no camera device; add your camera as a Video Capture Device, or pick the " +
          "source to analyze under Camera source below");
    const voiced = Object.keys(filters).find((source) => has(source, "sceneuron_voice_filter"));
    const microphone = voiced
      ? step("present", voiced)
      : mic
        ? (filters[mic].push(["sceneuron_voice_filter", "Voice"]), step("added", mic))
        : step("skipped", "", "OBS has no Mic/Aux device; choose your microphone in Settings > Audio");
    const notInstalled = "Sceneuron's plugin for Stream Deck is not installed on this Mac; double-click " +
      "ai.chaski.sceneuron.streamDeckPlugin to install it in the Stream Deck app, then choose Set up OBS for me again";
    const stream_deck = pairsStreamDeck ? step("added", "Stream Deck") : step("skipped", "", notInstalled);
    return { overlay, camera, microphone, stream_deck };
  };
  const ws = {
    request: async (name, args = {}) => {
      calls.push([name, args]);
      if (name === "GetSpecialInputs") return { mic1: mic, mic2: null, desktop1: "Desktop Audio" };
      if (name === "RemoveInput") {
        const removed = items.findIndex(({ sourceName }) => sourceName === args.inputName);
        if (prunesLate) items[removed].removed = true;
        else items.splice(removed, 1);
      }
      if (name === "RemoveSourceFilter") {
        filters[args.sourceName] = filters[args.sourceName].filter(([, filterName]) => filterName !== args.filterName);
      }
      if (name === "GetSourceFilterList") {
        return { filters: filters[args.sourceName].map(([filterKind, filterName]) => ({ filterKind, filterName })) };
      }
      if (name === "GetSceneItemList") {
        const sceneItems = items.map(({ sourceName, sceneItemId }) => ({ sourceName, sceneItemId }));
        items.splice(0, items.length, ...items.filter(({ removed }) => !removed));
        return { sceneItems };
      }
      return {};
    },
  };
  const api = async (method, path, body) => {
    calls.push([method, path, body]);
    if (path === "/api/setup") return setUp();
    if (path === "/api/sources/attach") return analyze();
    const analyzed = has(CAMERA, "sceneuron_camera_filter");
    if (path === "/api/sources") return [{ uuid: "clip", name: CAMERA, analyzed }];
    const sources = Object.keys(filters).map((name) => ({ name, voice: has(name, "sceneuron_voice_filter") }));
    return { sources };
  };
  return { t: { api, ws, scene: "Sceneuron E2E", camera: CAMERA, overlayName: OVERLAY }, calls };
}

test("with Mic/Aux, the setup adds the overlay and voice filter, skips the clip, then finds all in place", async () => {
  const { t, calls } = fakeObs();
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.deepEqual(results.map((r) => [r.name, r.ok]), [
    ["before Set up OBS for me, no source is analyzed and no microphone has the voice filter", true],
    ["Set up OBS for me adds the Sceneuron Overlay to the current scene", true],
    ["it analyzes no media file, even the only video source, and says to pick the source", true],
    ["it gives Mic/Aux the Sceneuron Voice filter", true],
    ["it pairs no Stream Deck from the scratch home, which lacks the plugin, and says how to install it", true],
    ["once the clip is picked, a second click finds everything in place and adds nothing", true],
  ]);
  assert.deepEqual(calls.find(([name]) => name === "RemoveInput"), ["RemoveInput", { inputName: OVERLAY }]);
  const attached = calls.filter(([method, path]) => method === "POST" && path === "/api/sources/attach");
  assert.deepEqual(attached, [["POST", "/api/sources/attach", { uuid: "clip" }]], "the clip is analyzed again");
});

test("an overlay removed but not yet pruned does not count as in place, and the setup checks still pass", async () => {
  const { t } = fakeObs({ prunesLate: true });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.ok(results.every((r) => r.ok), JSON.stringify(results));
});

test("a setup that takes the removed, unpruned overlay for the one in place fails the overlay check", async () => {
  const { t } = fakeObs({ prunesLate: true, countsRemoved: true });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.deepEqual(results.map((r) => r.ok), [true, false, true, true, true, false]);
});

test("a setup that starts camera analysis on the clip fails the camera check", async () => {
  const { t } = fakeObs({ analyzesClip: true });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.deepEqual(results.map((r) => r.ok), [true, true, false, true, true, true]);
});

test("a setup that sends a Stream Deck pairing link from the scratch home fails the Stream Deck checks", async () => {
  const { t } = fakeObs({ pairsStreamDeck: true });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.deepEqual(results.map((r) => r.ok), [true, true, true, true, false, false]);
});

test("without Mic/Aux, the microphone is skipped with a reason", async () => {
  const { t } = fakeObs({ mic: null });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.ok(results.every((r) => r.ok), JSON.stringify(results));
  assert.ok(results.some((r) => r.name === "without Mic/Aux in this OBS, it skips the microphone and says why"));
});

test("a setup that adds nothing fails its checks, and a throw is recorded instead of ending the proof", async () => {
  const { t } = fakeObs({ broken: true });
  const { results, check } = recorder();
  await runSetupScenario({ ...t, check });
  assert.deepEqual(results.map((r) => r.ok), [true, false, false, false, false, false]);
  const failing = recorder();
  const ws = { request: async () => Promise.reject(new Error("obs-websocket closed")) };
  await runSetupScenario({ ...t, ws, check: failing.check });
  assert.deepEqual(failing.results.map((r) => [r.name, r.ok]), [["the setup scenario ran to the end", false]]);
});
