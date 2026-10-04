// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Draws docs/images/streamdeck-keys.png: the four pages of the plugin's ready-made layout for the
// 15-key Stream Deck, two by two. Every key is drawn by the plugin's own code (faces.ts,
// reaction-face.ts, and svg.ts) from a real control view: sceneuron-dev runs with the meeting,
// planning poker, and status packs, a card up in each deck, and the email reaction playing, and
// the picture shows push to talk held on the first page. Stream Deck draws its own page keys, Go
// to Page and Page Indicator; the picture draws them the plugin's way, with the page each goes to
// and the page number. Headless Chrome renders the page, as the browser tests do.
//
// Build the dev server first:
//   cargo build --locked -p sceneuron-dev
// Usage: node e2e/streamdeck-keys.mjs [--dev-binary PATH] [--chrome PATH] [--out FILE] [--work-dir DIR]

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { automationFace, cardFace, lowerFace, roomFace, talkFace } from "../streamdeck/src/lib/faces.ts";
import { reactionFace } from "../streamdeck/src/lib/reaction-face.ts";
import { keySvg, svgDataUrl } from "../streamdeck/src/lib/svg.ts";
import { parseControlView } from "../streamdeck/src/lib/view.ts";
import { PAGES } from "../streamdeck/src/profiles/layout.ts";
import { withPack } from "./card-steps.mjs";
import { Browser, findChrome } from "./chrome.mjs";
import { DevServer, devBinary } from "./dev-server.mjs";

const { values: options } = parseArgs({
  options: {
    "dev-binary": { type: "string" },
    chrome: { type: "string" },
    out: { type: "string" },
    "work-dir": { type: "string" },
  },
});
const repo = resolve(import.meta.dirname, "..");
const out = resolve(options.out ?? join(repo, "docs", "images", "streamdeck-keys.png"));
const token = "streamdeck-keys-editor-token-0123456789";
const controlToken = "streamdeck-keys-control-token-0123456789";
const PACKS = ["meeting", "poker", "status"];
const RAISED = [
  ["meeting", "wrap-up"],
  ["status", "muted"],
  ["poker", "p8"],
];
/** Pixel sizes of the picture: keys, the gaps between them, the device's border, and the margins. */
const KEY = 112;
const GAP = 18;
const BORDER = 30;
const MARGIN = 32;
const LABEL = 42;
/** Pages side by side in each row of the picture. */
const ACROSS = 2;

/** A plain key the way the plugin draws one, for Stream Deck's own page keys. */
function pageKey(title, tag) {
  return { title, tag, lit: false, dim: false };
}

/** What key `key` on page number `page` (from 1) shows for control view `view`. */
function face(key, page, view) {
  switch (key.kind) {
    case "card":
      return cardFace(view, "live", { deck: key.deck, card: key.card });
    case "lower":
      return lowerFace(view, "live", { deck: key.deck });
    case "automation":
      return automationFace(view, "live").face;
    case "talk": {
      // The key takes Sceneuron's word on push to talk when the view has voice control's state, so
      // the view says held exactly where the picture holds the key.
      const held = page === 1;
      return talkFace(view.voice ? { ...view, voice: { ...view.voice, talking: held } } : view, "live", held);
    }
    case "reveal":
    case "next-round":
      return roomFace(key.kind, view, "live", false);
    case "reaction":
      return reactionFace("live", { reaction: key.reaction }, view.flood?.flood ?? null);
    case "go-to-page":
      return pageKey(key.title, `Go to page ${key.page}`);
    case "page-indicator":
      return pageKey(String(page), "Page");
  }
  throw new Error(`no face for a ${key.kind} key`);
}

/** The HTML page with every page of keys drawn for `view`; an empty key is drawn dark, as on a device. */
function picture(view) {
  const deviceWidth = 5 * KEY + 4 * GAP + 2 * BORDER;
  const devices = PAGES.map((page, index) => {
    const keys = page.rows.flatMap((row) => row.map((key) => (key ? svgDataUrl(keySvg(face(key, index + 1, view))) : null)));
    const images = keys
      .map((url) => (url ? `<img src="${url}" width="${KEY}" height="${KEY}" alt="">` : `<div class="empty"></div>`))
      .join("");
    return `<section><h2>Page ${index + 1}: ${page.name}</h2><div class="deck">${images}</div></section>`;
  });
  const style = `
    html, body { margin: 0; background: #0b0c10; }
    main {
      display: grid; grid-template-columns: repeat(${ACROSS}, max-content); gap: ${MARGIN}px ${MARGIN + 8}px;
      padding: ${MARGIN}px;
    }
    h2 { margin: 0; height: ${LABEL}px; color: #c9cdd6; font: 600 22px/30px "Helvetica Neue", sans-serif; }
    .deck {
      display: grid; grid-template-columns: repeat(5, ${KEY}px); gap: ${GAP}px; padding: ${BORDER}px;
      width: ${deviceWidth - 2 * BORDER}px; background: #1c1d22; border-radius: 36px;
    }
    img { display: block; border-radius: 14px; }
    .empty { width: ${KEY}px; height: ${KEY}px; border-radius: 14px; background: #000000; }`;
  const head = `<meta charset="utf-8"><style>${style}</style>`;
  return `<!doctype html><html><head>${head}</head><body><main>${devices.join("")}</main></body></html>`;
}

const args = ["--port", "0", "--token", token, "--control-token", controlToken];
const server = new DevServer(devBinary(options["dev-binary"]), args);
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-keys-")) : resolve(options["work-dir"]);
let browser = null;
try {
  const base = await server.ready();
  const api = async (method, path, body, key = token) => {
    const headers = { "X-Sceneuron-Token": key, "Content-Type": "application/json" };
    const text = body === undefined ? undefined : JSON.stringify(body);
    const response = await fetch(`${base}${path}`, { method, headers, body: text });
    const json = await response.json();
    if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
    return json;
  };
  const packs = await api("GET", "/api/packs");
  let project = await api("GET", "/api/project");
  for (const id of PACKS) {
    project = withPack(project, packs.find((pack) => pack.id === id));
  }
  await api("PUT", "/api/project", project);
  for (const [deck, card] of RAISED) {
    await api("POST", "/api/card", { deck, card, action: "raise" });
  }
  await api("POST", "/api/flood", { flood: "email" });
  const view = parseControlView(await api("GET", "/api/control/state", undefined, controlToken));
  if (!view) throw new Error("the control state is not a control view");

  mkdirSync(work, { recursive: true });
  browser = await Browser.launch({ chrome: findChrome(options.chrome), workDir: work });
  const down = Math.ceil(PAGES.length / ACROSS);
  const width = ACROSS * (5 * KEY + 4 * GAP + 2 * BORDER) + (ACROSS + 1) * MARGIN + (ACROSS - 1) * 8;
  const height = down * (LABEL + 3 * KEY + 2 * GAP + 2 * BORDER) + (down + 1) * MARGIN;
  const page = await browser.newPage({ width, height });
  await page.goto(`data:text/html;base64,${Buffer.from(picture(view), "utf8").toString("base64")}`);
  await page.eval("Promise.all([document.fonts.ready, ...[...document.images].map((image) => image.decode())])");
  mkdirSync(resolve(out, ".."), { recursive: true });
  writeFileSync(out, await page.screenshot());
  console.log(`wrote ${out} (${width}x${height})`);
} finally {
  await browser?.close();
  await server.stop();
  if (ownWork) rmSync(work, { recursive: true, force: true });
}
