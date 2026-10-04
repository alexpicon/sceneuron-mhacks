// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Helpers the end-to-end proof and its step modules share: recorded checks, polling, the editor
// API, and reading elements, where they are on the overlay, and overlay pixels.

import { decodePng, meanAlpha } from "./png.mjs";

/** A list of check results and the `check` that records one and prints PASS or FAIL. */
export function createChecks(log = console.log) {
  const results = [];
  function check(requirement, name, ok, detail = "") {
    results.push({ requirement, name, ok: Boolean(ok), detail });
    log(`${ok ? "PASS" : "FAIL"} [${requirement}] ${name}${detail ? ` - ${detail}` : ""}`);
  }
  return { results, check };
}

export const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

/** Polls `probe` until it returns something truthy, which it returns; throws on timeout. */
export async function waitFor(what, probe, timeoutMs = 45_000, intervalMs = 250) {
  const deadline = Date.now() + timeoutMs;
  let last;
  while (Date.now() < deadline) {
    try {
      last = await probe();
      if (last) return last;
    } catch (error) {
      last = error;
    }
    await sleep(intervalMs);
  }
  throw new Error(`timed out waiting for ${what} (last: ${last instanceof Error ? last.message : JSON.stringify(last)})`);
}

/** Calls the editor API on 127.0.0.1:`port` with `token`; a reply that is not ok throws. */
export function editorApi(port, token) {
  return async function api(method, path, body) {
    const response = await fetch(`http://127.0.0.1:${port}${path}`, {
      method,
      headers: { "X-Sceneuron-Token": token, "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const json = await response.json();
    if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
    return json;
  };
}

export const element = (state, id) => state.elements.find((e) => e.id === id);

/**
 * The middle of an element's box in a state or project, as fractions of its canvas, which an overlay source fills:
 * the box without `inset` of its width and height on each side, so rounded corners stay out.
 */
export function elementBox(state, id, inset = 0.15) {
  const { x, y, width, height } = element(state, id).props;
  const canvas = state.canvas;
  return {
    x0: (x + width * inset) / canvas.width,
    y0: (y + height * inset) / canvas.height,
    x1: (x + width * (1 - inset)) / canvas.width,
    y1: (y + height * (1 - inset)) / canvas.height,
  };
}

/** The mean alpha of an overlay source's screenshot inside `box` (fractions of the image). */
export async function overlayAlpha(ws, overlayName, box) {
  const shot = await ws.request("GetSourceScreenshot", { sourceName: overlayName, imageFormat: "png", imageWidth: 960 });
  return meanAlpha(decodePng(Buffer.from(shot.imageData.split(",")[1], "base64")), box);
}
