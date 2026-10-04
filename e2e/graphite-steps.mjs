// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Graphite steps of the browser regression test (editor-browser.mjs): the Graphite theme draws a raised card with
// its icon in an amber tile and its words without the emoji, edges a regular card in amber, and gives the name
// label its amber bar and icon; shares the colorful emoji reactions; and the on-air pill and timer follow simulated OBS: LIVE with an
// amber dot and a counting timer while it streams, the scene alone with a gray dot and 00:00:00 when it stops.
// They run after the theme steps, against the project those saved, which has the meeting and poker decks, and leave
// it as they found it, on None.

import { sleep, waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
const AMBER = "rgb(214, 167, 108)";
const GRAY = "rgb(138, 134, 127)";
const PILL = { id: "on-air", text: "{{on-air}}", x: 1414 };
const TIMER = { id: "on-air-time", text: "{{on-air-time}}", x: 1730 };

/** A starter-like on-air text element. */
function onAir({ id, text, x }) {
  const props = { x, y: 40, width: id === PILL.id ? 300 : 150, height: 52, text, font_size: 24, visible: true };
  return { id, name: id, kind: "text", props };
}

/** Runs the Graphite steps; `overlay` is the overlay page and `server` the sceneuron-dev it shows. */
export async function runGraphiteSteps({ page, overlay, server, step, same, api }) {
  const { check, clean, ctrlS, project } = editorHelpers(page, api);
  const node = (id) => `#overlay-root [data-element-id=${js(id)}]`;
  const classes = (id) => overlay.eval(`[...document.querySelector(${js(node(id))}).classList]`);
  const text = (id) => overlay.text(`${node(id)} .sn-text`);
  const count = (selector) => overlay.eval(`document.querySelectorAll(${js(selector)}).length`);
  const style = (selector, property, pseudo = null) =>
    overlay.eval(`getComputedStyle(document.querySelector(${js(selector)}), ${js(pseudo)}).${property}`);
  const save = async (theme) => {
    await page.click("#tab-settings");
    await page.click(`#theme-${theme}`);
    await ctrlS();
    await clean();
    await waitFor(`the ${theme} overlay`, async () =>
      (await overlay.eval(`document.getElementById("overlay-root").getAttribute("data-theme")`)) === theme);
  };
  const decks = await project().then((saved) => Object.fromEntries(saved.decks.map((deck) => [deck.id, deck])));

  await step("Graphite edges a tiled card and shows the newest poker card's amber face", async () => {
    await save("graphite");
    const meeting = decks.meeting.element;
    const poker = decks.poker.element;
    try {
      await api("POST", "/api/card", { deck: "meeting", card: "wrap-up", action: "raise" });
      await waitFor("the tiled meeting card", async () => (await count(`${node(meeting)} .sn-icon svg`)) === 1);
      same(await text(meeting), "Wrap it up", "the card's words without the emoji the tile shows");
      same(await style(`${node(meeting)} .sn-icon`, "backgroundColor"), AMBER, "the tile");
      check((await classes(meeting)).includes("sn-active"), "the only card up is not the newest");
      same(await style(`${node(meeting)} .sn-body`, "borderTopColor"), AMBER, "the newest tiled card's edge");
      await sleep(20);
      await api("POST", "/api/card", { deck: "poker", card: "p5", action: "raise" });
      await waitFor("the newest card", async () => (await classes(poker)).includes("sn-active"));
      check(!(await classes(meeting)).includes("sn-active"), "the older card is still edged");
      same(await style(`${node(poker)} .sn-gp-front`, "backgroundColor"), AMBER, "the poker card's face");
      same(await count(`${node(poker)} .sn-icon`), 0, "a poker value is its own face");
      same(await style(`${node("name-label")} .sn-body`, "backgroundColor", "::before"), AMBER, "the label's bar");
      same(await count(`${node("name-label")} .sn-icon svg`), 1, "the label's icon");
    } finally {
      await api("POST", "/api/card", { deck: "meeting", action: "lower" });
      await api("POST", "/api/card", { deck: "poker", action: "lower" });
    }
  });

  await step("Graphite shows the shared colorful emoji reaction", async () => {
    await save("graphite");
    const layer = "#overlay-root > .sn-flood";
    await overlay.send("Page.bringToFront");
    try {
      await api("POST", "/api/flood", { flood: "email" });
      await waitFor("the Graphite emoji flood", async () => (await count(`${layer} span`)) === 94);
      same(await overlay.text(`${layer} .sn-flood-banner`), "📧 This meeting could have been an email 🤦", "banner");
      same(await style(`${layer} .sn-flood-banner`, "backgroundColor"), "rgb(29, 78, 216)", "preset color");
    } finally {
      await waitFor("the flood to end", async () => (await count(layer)) === 0, 5_000);
    }
  });

  await step("the on-air pill and timer follow the simulated OBS on and off air", async () => {
    const before = await project();
    await api("PUT", "/api/project", { ...before, elements: [...before.elements, onAir(PILL), onAir(TIMER)] });
    const dot = () => style(`${node(PILL.id)} .sn-body`, "backgroundColor", "::before");
    try {
      await waitFor("the pill off air", async () => (await text(PILL.id)) === "Scene 01");
      same([await dot(), await text(TIMER.id)], [GRAY, "00:00:00"], "the gray dot and the timer off air");
      same(await style(`${node(PILL.id)} .sn-body`, "textTransform"), "uppercase", "the pill's capitals");
      server.say("stream");
      await waitFor("the pill on air", async () => (await text(PILL.id)) === "Scene 01 · LIVE");
      same(await dot(), AMBER, "the amber dot");
      check((await classes(PILL.id)).includes("sn-live"), "the pill is not marked live");
      const first = await text(TIMER.id);
      check(/^00:00:0\d$/.test(first), `the timer starts at ${first}`);
      await waitFor("the timer counting", async () => (await text(TIMER.id)) !== first, 2500);
      server.say("stop");
      await waitFor("the pill off air again", async () => (await text(PILL.id)) === "Scene 01");
      same(await text(TIMER.id), "00:00:00", "the timer off air");
      return `counted from ${first}`;
    } finally {
      server.say("stop");
      await api("PUT", "/api/project", { ...(await project()), elements: before.elements });
      await save("none");
    }
  });
}
