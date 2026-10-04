// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// The subtitles part of the end-to-end proof, without audio: the Subtitles pack added through the project API, or the
// starter project's bars, load hidden, a line simulated through the editor API shows in its spoken bar and is drawn by
// the overlay inside OBS, and the bar hides again once the line clears. The scenario records failures as failed checks
// instead of throwing them, so the rest of the proof still runs.

import { withPackOnce } from "./card-steps.mjs";
import { elementBox } from "./harness.mjs";

const PACK = "subtitles";
const BAR = "subtitles-spoken";
const LINE = "Checking the subtitles bar in OBS";

/** Runs the subtitles scenario against a running OBS; `t` holds the harness helpers. */
export async function runSubtitleScenario(t) {
  try {
    await subtitleSteps(t);
  } catch (error) {
    t.check("Subtitles", "the subtitles scenario ran to the end", false, error.message);
  }
}

async function subtitleSteps({ api, ws, check, waitFor, element, overlayAlpha, overlayName }) {
  const pack = (await api("GET", "/api/packs")).find((offered) => offered.id === PACK);
  check("Subtitles", "the editor API offers the Subtitles pack", Boolean(pack));
  if (!pack) return;
  // The starter project of a fresh install has the pack's bars already.
  const project = withPackOnce(await api("GET", "/api/project"), pack);
  await api("PUT", "/api/project", project);
  const loaded = await api("GET", "/api/state");
  const bar = element(loaded, BAR);
  check("Subtitles", "the Subtitles pack loads, its bar hidden while nobody speaks",
    bar?.props.text === "" && bar?.props.visible === false, JSON.stringify(bar?.props ?? null));
  await api("POST", "/api/subtitles/simulate", { text: LINE });
  const shown = element(await api("GET", "/api/state"), BAR);
  check("Subtitles", "a simulated line shows in the spoken bar", shown?.props.text === LINE && shown.props.visible,
    JSON.stringify(shown?.props.text ?? null));
  const drawn = await waitFor("the subtitles bar on screen", async () => {
    const alpha = await overlayAlpha(ws, overlayName, elementBox(project, BAR));
    return alpha > 80 ? alpha : null;
  }, 15_000).catch(() => 0);
  check("Subtitles", "the overlay draws the subtitles bar in OBS", drawn > 80, `mean alpha ${drawn.toFixed(1)}`);
  const cleared = await waitFor("the line cleared", async () => {
    const state = await api("GET", "/api/state");
    return element(state, BAR).props.visible ? null : state;
  }, 15_000).catch(() => null);
  check("Subtitles", "the bar hides again once the line clears", Boolean(cleared));
}
