// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Draws the overlay in every theme: sceneuron-dev runs with the default project and the meeting and subtitles
// packs, the "Wrap it up" card raised, and a line and its translation held on the subtitle bars, and headless
// Chrome screenshots the real overlay page at 1920x1080 over a dark backdrop, once per theme. The default name
// label and camera frame move to the top, where the packs leave room, so nothing overlaps. A contact sheet shows
// every theme side by side. The project stays in memory, and the server runs on a free port.
//
// Build the dev server first:
//   cargo build --locked -p sceneuron-dev
// Usage: node e2e/theme-shots.mjs [--dev-binary PATH] [--chrome PATH] [--out DIR] [--work-dir DIR]

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

import { withPack } from "./card-steps.mjs";
import { sleep, waitFor } from "./cdp.mjs";
import { Browser, findChrome } from "./chrome.mjs";
import { DevServer, devBinary } from "./dev-server.mjs";

const require = createRequire(import.meta.url);
const { THEMES } = require("../web/render-themes.js");

const { values: options } = parseArgs({
  options: {
    "dev-binary": { type: "string" },
    chrome: { type: "string" },
    out: { type: "string" },
    "work-dir": { type: "string" },
  },
});
const repo = resolve(import.meta.dirname, "..");
const out = resolve(options.out ?? join(repo, "target", "theme-shots"));
const token = "theme-shots-editor-token-0123456789abcdef";
const js = JSON.stringify;
const WIDTH = 1920;
const HEIGHT = 1080;
// The subtitle lines as the core wraps them for the bars: at most 51 characters each.
const HELD = {
  "subtitles-spoken": [
    "Good morning everyone, and welcome to the stream.",
    "Today we are planning the next sprint together.",
  ],
  "subtitles-translated": [
    "Buenos días a todos y bienvenidos a la transmisión.",
    "Hoy planificamos juntos el próximo sprint.",
  ],
};
const BACKDROP = "linear-gradient(135deg, #334155 0%, #1e293b 50%, #0f172a 100%)";
const MOVED = { "name-label": { x: 60, y: 60 }, "cam-frame": { x: 1240, y: 60 } };
// The contact sheet: two columns of half-size shots with their names above them.
const SHEET_GAP = 24;
const SHEET_LABEL = 44;

/** The contact sheet's HTML, with the shots beside it as relative file names. */
function sheet(shots) {
  const cells = shots
    .map(({ theme, file }) => `<figure><figcaption>${theme.name}</figcaption><img src="${file}" alt=""></figure>`)
    .join("");
  const style = `
    html, body { margin: 0; background: #0b0c10; }
    main {
      display: grid; grid-template-columns: repeat(2, ${WIDTH / 2}px); gap: ${SHEET_GAP}px; padding: ${SHEET_GAP}px;
    }
    figure { margin: 0; }
    figcaption { height: ${SHEET_LABEL}px; color: #c9cdd6; font: 600 24px/32px "Helvetica Neue", sans-serif; }
    img { display: block; width: ${WIDTH / 2}px; height: ${HEIGHT / 2}px; border-radius: 8px; }`;
  const head = `<meta charset="utf-8"><style>${style}</style>`;
  return `<!doctype html><html><head>${head}</head><body><main>${cells}</main></body></html>`;
}

const server = new DevServer(devBinary(options["dev-binary"]), ["--port", "0", "--token", token]);
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-themes-")) : resolve(options["work-dir"]);
let browser = null;
try {
  const base = await server.ready();
  const api = async (method, path, body) => {
    const headers = { "X-Sceneuron-Token": token, "Content-Type": "application/json" };
    const text = body === undefined ? undefined : js(body);
    const response = await fetch(`${base}${path}`, { method, headers, body: text });
    const json = await response.json();
    if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
    return json;
  };
  const packs = await api("GET", "/api/packs");
  let project = await api("GET", "/api/project");
  for (const id of ["meeting", "subtitles"]) {
    project = withPack(project, packs.find((pack) => pack.id === id));
  }
  for (const element of project.elements) {
    Object.assign(element.props, MOVED[element.id]);
  }
  await api("PUT", "/api/project", project);
  for (const [element, lines] of Object.entries(HELD)) {
    await api("POST", "/api/override", { element, props: { text: lines.join("\n"), visible: true } });
  }

  mkdirSync(work, { recursive: true });
  mkdirSync(out, { recursive: true });
  browser = await Browser.launch({ chrome: findChrome(options.chrome), workDir: work });
  const page = await browser.newPage({ width: WIDTH, height: HEIGHT });
  await page.goto(`${base}/overlay?token=${token}`);
  await page.eval(`document.body.style.setProperty("background", ${js(BACKDROP)})`);
  const shots = [];
  for (const theme of THEMES) {
    await api("PUT", "/api/project", { ...(await api("GET", "/api/project")), theme: theme.id });
    // Raising again restarts the card's 15 second auto-hide and its enter animation.
    await api("POST", "/api/card", { deck: "meeting", card: "wrap-up", action: "raise" });
    await waitFor(`the ${theme.name} theme`, () =>
      page.eval(`document.getElementById("overlay-root").getAttribute("data-theme") === ${js(theme.id)}`));
    await waitFor("the raised card", async () =>
      String(await page.text('[data-element-id="meeting-card"] .sn-text')).endsWith("Wrap it up"));
    await page.eval("document.fonts.ready.then(() => true)");
    await sleep(900);
    // A blinking cursor is caught while it shows.
    await waitFor("every cursor lit", () =>
      page.eval(`[...document.querySelectorAll(".sn-text")].every((text) =>
        getComputedStyle(text, "::after").opacity === "1")`));
    const file = `${theme.id}.png`;
    writeFileSync(join(out, file), await page.screenshot());
    shots.push({ theme, file });
    console.log(`wrote ${join(out, file)}`);
  }
  const sheetFile = join(out, "themes.html");
  writeFileSync(sheetFile, sheet(shots));
  const rows = Math.ceil(shots.length / 2);
  const sheetPage = await browser.newPage({
    width: 2 * (WIDTH / 2) + 3 * SHEET_GAP,
    height: rows * (SHEET_LABEL + HEIGHT / 2) + (rows + 1) * SHEET_GAP,
  });
  await sheetPage.goto(pathToFileURL(sheetFile).href);
  await sheetPage.eval("Promise.all([...document.images].map((image) => image.decode())).then(() => true)");
  writeFileSync(join(out, "themes.png"), await sheetPage.screenshot());
  rmSync(sheetFile);
  console.log(`wrote ${join(out, "themes.png")}`);
} finally {
  await browser?.close();
  await server.stop();
  if (ownWork) rmSync(work, { recursive: true, force: true });
}
