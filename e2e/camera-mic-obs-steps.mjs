// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Camera & mic checks of the end-to-end proof in a real OBS (run.mjs): the editor lists the proof's camera clip among
// the sources that carry audio, and OBS's global audio devices first, once each; choosing the clip, or the first global
// device when this OBS has one, through the API moves the Sceneuron Voice filter to it; and once camera analysis runs,
// the preview route returns a JPEG of the clip, never cached, the analysis reports the clip's face, and the live
// description counts its person.

const VOICE_FILTER = "sceneuron_voice_filter";

/** The width and height in a JPEG's start-of-frame segment, or null when it has none. */
export function jpegSize(bytes) {
  if (bytes[0] !== 0xff || bytes[1] !== 0xd8) return null;
  let offset = 2;
  while (offset + 8 < bytes.length) {
    if (bytes[offset] !== 0xff) return null;
    const marker = bytes[offset + 1];
    if (marker >= 0xc0 && marker <= 0xc3) {
      return { height: (bytes[offset + 5] << 8) | bytes[offset + 6], width: (bytes[offset + 7] << 8) | bytes[offset + 8] };
    }
    offset += 2 + ((bytes[offset + 2] << 8) | bytes[offset + 3]);
  }
  return null;
}

/** Whether `bytes` is a whole JPEG of at most 640 pixels a side with the 16:9 shape of the proof's 1280x720 clip. */
export function isClipPreview(bytes) {
  const size = jpegSize(bytes);
  const whole = bytes.length > 4 && bytes.at(-2) === 0xff && bytes.at(-1) === 0xd9;
  return Boolean(whole && size && size.width <= 640 && Math.abs(size.width / size.height - 16 / 9) < 0.02);
}

/**
 * OBS's global audio devices from obs-websocket's GetSpecialInputs, in the order the editor lists them: Mic/Aux to
 * Mic/Aux 4, then Desktop Audio and Desktop Audio 2.
 */
export function globalDeviceNames(special) {
  return ["mic1", "mic2", "mic3", "mic4", "desktop1", "desktop2"].map((key) => special[key]).filter(Boolean);
}

/** Whether the editor's audio `sources` start with exactly `devices`, in order and marked global, and none follow. */
export function listsGlobalDevicesFirst(sources, devices) {
  const head = sources.slice(0, devices.length);
  const rest = sources.slice(devices.length);
  return head.length === devices.length &&
    head.every((source, index) => source.global === true && source.name === devices[index]) &&
    rest.every((source) => source.global !== true && !devices.includes(source.name));
}

async function hasVoiceFilter(ws, sourceName) {
  const { filters } = await ws.request("GetSourceFilterList", { sourceName });
  return filters.some((filter) => filter.filterKind === VOICE_FILTER);
}

/**
 * Runs the checks once camera analysis runs on `camera`, the clip's source name; `editorPort` and `token` reach the
 * preview, which answers a picture instead of JSON.
 */
export async function runCameraMicScenario({ api, ws, check, waitFor, camera, editorPort, token }) {
  const audio = await api("GET", "/api/audio-sources");
  const clip = audio.sources.find((source) => source.name === camera);
  const names = audio.sources.map((source) => source.name).join(", ");
  check("Mic", "the editor lists the camera clip among the sources that carry audio", audio.available && clip, names);
  const devices = globalDeviceNames(await ws.request("GetSpecialInputs"));
  check("Mic", "the editor lists OBS's global audio devices first, Mic/Aux before Desktop Audio, once each",
    listsGlobalDevicesFirst(audio.sources, devices), devices.join(", ") || "this OBS has no global audio devices");
  if (clip) {
    await api("POST", "/api/audio-sources/attach", { uuid: clip.uuid });
    const listed = (await api("GET", "/api/audio-sources")).sources.find((source) => source.uuid === clip.uuid);
    check("Mic", "choosing it through the editor API adds the Sceneuron Voice filter",
      (await hasVoiceFilter(ws, camera)) && listed?.voice);
  }
  // The first global device is Mic/Aux, where a microphone usually is, when the computer has an audio input.
  const device = audio.sources.find((source) => source.global && source.name === devices[0]);
  if (clip && device) {
    await api("POST", "/api/audio-sources/attach", { uuid: device.uuid });
    const moved = (await hasVoiceFilter(ws, device.name)) && !(await hasVoiceFilter(ws, camera));
    await api("POST", "/api/audio-sources/attach", { uuid: clip.uuid });
    const back = (await hasVoiceFilter(ws, camera)) && !(await hasVoiceFilter(ws, device.name));
    check("Mic", "choosing a global audio device through the editor API moves the Sceneuron Voice filter to it, and " +
      "choosing the clip moves it back", moved && back, device.name);
  }

  // The first requests start the preview and answer why there is no picture yet.
  const preview = await waitFor("a preview JPEG", async () => {
    const response = await fetch(`http://127.0.0.1:${editorPort}/api/camera/preview`, {
      headers: { "X-Sceneuron-Token": token },
    });
    if (!(response.headers.get("Content-Type") ?? "").startsWith("image/jpeg")) {
      await response.text();
      return null;
    }
    return { bytes: new Uint8Array(await response.arrayBuffer()), cache: response.headers.get("Cache-Control") };
  }, 15_000).catch(() => null);
  const size = preview && jpegSize(preview.bytes);
  check("Preview", "the preview route returns a JPEG of the test clip once camera analysis runs",
    preview && isClipPreview(preview.bytes) && preview.cache === "no-store",
    size ? `${size.width}x${size.height}, ${preview.bytes.length} bytes` : "no JPEG");

  const analysis = await waitFor("the clip's face in the analysis", async () => {
    const found = await api("GET", "/api/camera/analysis");
    return found.seen.some((fact) => /\bfaces? in view\b/.test(fact)) && found.faces.length > 0 ? found : null;
  }, 30_000).catch(() => null);
  check("Preview", "on-device analysis reports the clip's face, with its box, beside the preview", analysis,
    analysis ? analysis.seen.join("; ") : "no face reported");

  const described = await waitFor("the clip's person in the live description", async () => {
    const { seen } = await api("GET", "/api/state");
    return describesPerson(seen) ? seen : null;
  }, 30_000).catch(() => null);
  check("Preview", "the live description counts the clip's person", described, described ?? "no person described");
}

/** Whether a live description starts by counting at least one person, as "1 person; mouth open" does. */
export function describesPerson(line) {
  return typeof line === "string" && /^(1 person|[1-9]\d* people)(;|$)/.test(line);
}
