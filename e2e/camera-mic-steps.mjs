// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Camera & mic steps of the browser regression test (editor-browser.mjs), against the dev server's simulated camera
// picture and microphones (--demo-presence): the camera preview showing and updating while its tab is shown, and
// otherwise stopping and dropping its picture, what on-device analysis finds, and choosing the microphone, with what
// the Microphone section and the AI tab's Voice and Subtitles sections say before and after. They run after the live
// steps, which analyze the first simulated camera.

import { sleep, waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
const MIC = "Mic/Aux (simulated)";
const NO_MIC = "No microphone has the Sceneuron Voice filter yet, so voice control and subtitles hear nothing.";
const LISTENING = `Sceneuron listens to ${MIC} for voice control and subtitles.`;

/**
 * Counts the editor's preview requests from here on, in window.previewAsks, and notes the picture URLs it revokes, in
 * window.revokedUrls.
 */
const INSTRUMENT_PREVIEW = `(() => {
  window.previewAsks = 0;
  const fetch = window.fetch;
  window.fetch = (input, init) => {
    if (String(input).includes("/api/camera/preview")) window.previewAsks += 1;
    return fetch(input, init);
  };
  window.revokedUrls = new Set();
  const revoke = URL.revokeObjectURL;
  URL.revokeObjectURL = (url) => {
    window.revokedUrls.add(String(url));
    revoke.call(URL, url);
  };
})()`;

/** Makes the page look hidden or visible to its scripts, as switching browser tabs would. */
const visibility = (state) => `(() => {
  Object.defineProperty(document, "visibilityState", { value: ${js(state)}, configurable: true });
  document.dispatchEvent(new Event("visibilitychange"));
})()`;

/** Runs the Camera & mic steps; `step(name, body)` reports each one. */
export async function runCameraMicSteps({ page, step, same, api }) {
  const { q, check, toasts } = editorHelpers(page, api);
  const hidden = (id) => q(`document.getElementById(${js(id)}).hidden`);
  const source = () => q(`document.getElementById("camera-image").getAttribute("src")`);
  const asks = () => q("window.previewAsks");
  const shown = async () => !(await hidden("camera-frame")) && Boolean(await source());
  /** Whether the preview holds no picture: no image source, no frame shown, and `previous`, a picture URL, revoked. */
  const dropped = (previous) => q(`window.revokedUrls.has(${js(previous)}) &&
    !document.getElementById("camera-image").hasAttribute("src") && document.getElementById("camera-frame").hidden`);

  await step("the camera preview shows the simulated camera and updates a few times a second", async () => {
    await q(INSTRUMENT_PREVIEW);
    await page.click("#tab-camera");
    await waitFor("the picture", () => q(`(() => {
      const image = document.getElementById("camera-image");
      return !document.getElementById("camera-frame").hidden && image.complete && image.naturalWidth === 640;
    })()`));
    same(await q(`document.getElementById("camera-image").naturalHeight`), 360, "picture height");
    same(await hidden("camera-status"), true, "the reason is hidden");
    same(await q(`document.getElementById("camera-frame").dataset.simulated`), "true", "marked simulated");
    check((await page.text("#camera-caption")).startsWith("Simulated camera: a drawn test picture"), "no caption");
    check((await source()).startsWith("blob:"), "the picture is not a blob URL");
    const seen = new Set();
    const started = Date.now();
    await waitFor("three pictures", async () => seen.add(await source()).size >= 3, 4000);
    const line = await page.text("#camera-seen");
    check(/^On-device analysis (sees: face in view \(0\.95\)\.|finds no face, hand, or sign right now\.)$/.test(line),
      `analysis line ${js(line)}`);
    const faces = await q(`document.querySelectorAll("#camera-faces .camera-face").length`);
    same(faces, line.includes("face in view") ? 1 : 0, "face boxes");
    return `${seen.size} pictures in ${Date.now() - started} ms, ${await asks()} requests`;
  });

  await step("the preview stops asking and drops its picture while its tab or the page is hidden", async () => {
    const left = await source();
    await page.click("#tab-properties");
    await waitFor("the picture dropped with the tab", () => dropped(left));
    same(await page.text("#camera-status"), "Starting the preview...", "what the tab shows when it comes back");
    await sleep(400);
    const before = await asks();
    await sleep(1000);
    same(await asks(), before, "requests with the Properties tab shown");
    await page.click("#tab-camera");
    await waitFor("asking again", async () => (await asks()) > before);
    await waitFor("a new picture", shown);
    const hiddenFrom = await source();
    await q(visibility("hidden"));
    await waitFor("the picture dropped with the page", () => dropped(hiddenFrom));
    await sleep(400);
    const hiddenAsks = await asks();
    await sleep(1000);
    same(await asks(), hiddenAsks, "requests while the page is hidden");
    await q(visibility("visible"));
    await waitFor("asking once the page shows", async () => (await asks()) > hiddenAsks);
    await waitFor("the picture once the page shows", shown);
  });

  await step("the AI tab says no microphone is chosen yet and leads to the list", async () => {
    await page.click("#tab-ai");
    const note = `${NO_MIC} Choose your microphone in the Camera & mic tab, or in OBS open the Audio Mixer, choose ` +
      "Filters on your microphone, and add the Sceneuron Voice filter.";
    await waitFor("the Voice note", async () => (await page.text("#voice-mic")) === note);
    same(await page.text("#subtitles-mic"), note, "the Subtitles note");
    same([await hidden("voice-mic-choose"), await page.text("#voice-mic-choose")], [false, "Choose microphone"], "button");
    await page.click("#voice-mic-choose");
    await waitFor("the Camera & mic tab", async () => !(await hidden("panel-camera")));
    same(await q(`document.activeElement.getAttribute("data-mic-name")`), MIC, "focus on the first microphone");
  });

  await step("the Microphone section lists the simulated microphones and listens to the one chosen", async () => {
    const names = `[...document.querySelectorAll("#mic-list li > div > span:first-child")].map((span) => span.textContent)`;
    same(await q(names), [MIC, "Desktop Audio (simulated)"], "microphones");
    const kind = await page.text("#mic-list li:first-child .source-kind");
    same(kind, "Global audio device", "kind");
    same(await page.text("#mic-status"), `${NO_MIC} Choose your microphone below.`, "status before");
    await page.click(`[data-mic-name=${js(MIC)}]`);
    await waitFor("the notification", async () => (await toasts()).includes(`Sceneuron now listens to ${MIC}.`));
    await waitFor("the listening chip", async () => (await page.text("#mic-list li:first-child .chip")) === "Listening");
    same(await page.text("#mic-status"), LISTENING, "status after");
    same(await page.focused(), "refresh-microphones", "focus");
    const { sources } = await api("GET", "/api/audio-sources");
    same(sources.map((mic) => [mic.name, mic.voice, mic.in_use]), [
      [MIC, true, true],
      ["Desktop Audio (simulated)", false, false],
    ], "the server's microphones");
  });

  await step("the Voice and Subtitles sections say which microphone they hear", async () => {
    await page.click("#tab-ai");
    await waitFor("the Voice note", async () => (await page.text("#voice-mic")) === LISTENING);
    same(await page.text("#subtitles-mic"), LISTENING, "the Subtitles note");
    same(await page.text("#voice-mic-choose"), "Change microphone", "button");
    await page.click("#tab-properties");
  });
}
