// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Draws the overlay's subtitle bars with their language labels and scripts of their own: sceneuron-dev runs with the
// simulated AI, whose scripted speaker's English shows as detected and is translated into Japanese, Quenya, and
// Klingon, and four bars show the spoken line and each translation, Quenya in Tengwar and Klingon in pIqaD. Headless
// Chrome screenshots the real overlay page at 1920x1080 over a dark backdrop once per theme, then with small Latin
// lines under the scripts, and in Sindarin and Toki Pona. Each shot waits until every bar shows the same line, whole.
// The project stays in memory, and the server runs on a free port.
//
// Build the dev server first:
//   cargo build --locked -p sceneuron-dev
// Usage: node e2e/subtitle-shots.mjs [--dev-binary PATH] [--chrome PATH] [--out DIR] [--work-dir DIR]

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
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
const out = resolve(options.out ?? join(repo, "target", "subtitle-shots"));
const token = "subtitle-shots-editor-token-0123456789abcdef";
const js = JSON.stringify;
const WIDTH = 1920;
const HEIGHT = 1080;
const BACKDROP = "linear-gradient(135deg, #334155 0%, #1e293b 50%, #0f172a 100%)";
// The spoken bar and a bar per target, stacked inside the title-safe area, and the default elements moved to the top.
const BARS = [
  ["subtitles-spoken", "{{subtitles}}", 470],
  ["subtitles-translated", "{{subtitles:1}}", 576],
  ["subtitles-second", "{{subtitles:2}}", 682],
  ["subtitles-third", "{{subtitles:3}}", 788],
];
const MOVED = { "name-label": { x: 60, y: 60 }, "cam-frame": { x: 1240, y: 60 } };
// A line and its translations stay up while the shots are taken.
const CLEAR_AFTER_MS = 60_000;
// The scripted speaker says a line every few seconds, so a whole one may take a cycle to come. A shot waits until
// nothing changed for the first of these times, and gives up on the pause after the second, as the speaker's next
// line starts five seconds after the last translation.
const LINE_WAIT_MS = 20_000;
const QUIET_MS = [700, 2_200];

const server = new DevServer(devBinary(options["dev-binary"]), ["--port", "0", "--token", token, "--demo-ai"]);
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-subtitles-")) : resolve(options["work-dir"]);
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
  let project = withPack(await api("GET", "/api/project"), packs.find((pack) => pack.id === "subtitles"));
  const translated = project.elements.find((element) => element.id === "subtitles-translated");
  for (const [id, text] of BARS.slice(2)) {
    project.elements.push({ ...translated, id, name: `Subtitles ${text}`, props: { ...translated.props, text } });
  }
  for (const element of project.elements) {
    const bar = BARS.find(([id]) => id === element.id);
    Object.assign(element.props, MOVED[element.id], bar ? { y: bar[2] } : {});
  }
  await api("PUT", "/api/project", project);
  const subtitles = (settings) => api("PUT", "/api/ai", { subtitles: settings });
  await subtitles({ consent: true, enabled: true, targets: ["ja", "qya", "tlh"], clear_after_ms: CLEAR_AFTER_MS });

  mkdirSync(work, { recursive: true });
  mkdirSync(out, { recursive: true });
  browser = await Browser.launch({ chrome: findChrome(options.chrome), workDir: work });
  const page = await browser.newPage({ width: WIDTH, height: HEIGHT });
  await page.goto(`${base}/overlay?token=${token}`);
  await page.eval(`document.body.style.setProperty("background", ${js(BACKDROP)})`);

  /**
   * Waits until every bar shows the same final line, whole: no words are still being said, every target has lines,
   * and nothing changed for a moment, as the translations come a little after their line, yet not for so long that
   * the next line is about to start. Then takes a shot.
   */
  async function shot(name, targets) {
    let last = "";
    let since = Date.now();
    await waitFor(`one whole line in ${targets.length} languages`, async () => {
      const view = await api("GET", "/api/subtitles");
      const now = js([view.lines, view.translations]);
      if (now !== last) {
        [last, since] = [now, Date.now()];
      }
      const translated = targets.every((code) => (view.translations[code] ?? []).length > 0);
      const quiet = Date.now() - since;
      return !view.live && view.lines.length > 0 && translated && quiet > QUIET_MS[0] && quiet < QUIET_MS[1];
    }, LINE_WAIT_MS);
    await page.eval("document.fonts.ready.then(() => true)");
    await sleep(700);
    const file = join(out, `${name}.png`);
    writeFileSync(file, await page.screenshot());
    console.log(`wrote ${file}`);
  }

  for (const theme of THEMES) {
    await api("PUT", "/api/project", { ...(await api("GET", "/api/project")), theme: theme.id });
    await waitFor(`the ${theme.name} theme`, () =>
      page.eval(`document.getElementById("overlay-root").getAttribute("data-theme") === ${js(theme.id)}`));
    await shot(`subtitles-${theme.id}`, ["ja", "qya", "tlh"]);
  }
  await api("PUT", "/api/project", { ...(await api("GET", "/api/project")), theme: "none" });
  await subtitles({ latin_lines: true });
  await waitFor("Latin lines", () => page.eval(`document.querySelectorAll(".sn-latin").length > 0`), LINE_WAIT_MS);
  await shot("subtitles-latin-lines", ["ja", "qya", "tlh"]);
  await subtitles({ latin_lines: false, targets: ["sjn", "tok", "ja"] });
  await shot("subtitles-sindarin-toki-pona", ["sjn", "tok", "ja"]);
} finally {
  await browser?.close();
  await server.stop();
  if (ownWork) rmSync(work, { recursive: true, force: true });
}
