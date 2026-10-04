// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Hand signal steps of the browser regression test (editor-browser.mjs): a meeting deck saved before the pack gained
// its fist, pointing, and peace signs keeps its own signals; a card's Hand signal list saves a choice into the project;
// a signal another card of the deck has is refused, naming both cards; one a deck whose hand signals are on also has is
// warned about, naming that deck and card; poses of one or two hands that show two signals are noted, saying where a
// vote decides; and the Cards tab's live line says where hand signals go, which a poker card raised from the API
// changes. They run after the Cards tab steps, which leave no deck, and leave none either.

import { withPack } from "./card-steps.mjs";
import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;
const SIGNAL = "deck-0-card-signal";
const ROUTE = "hand-signals-route";

/** Runs the hand signal steps; `step(name, body)` reports each one. */
export async function runHandSignalSteps({ page, step, same, api }) {
  const helpers = editorHelpers(page, api);
  const { q, check, count, dirty, toasts, project, clean, attribute } = helpers;
  const meeting = async () => (await project()).decks.find((deck) => deck.id === "meeting");
  const signalOf = async (id) => (await meeting()).cards.find((card) => card.id === id).signal;
  const notes = () =>
    q(`[...document.querySelectorAll("#${SIGNAL}-notes li")].map((li) => [li.className, li.textContent])`);
  // Picks a hand signal with the list focused, as a keyboard user would, and checks the rebuilt list keeps the focus.
  const chooseSignal = async (value) => {
    await q(`document.getElementById(${js(SIGNAL)}).focus()`);
    await page.choose(SIGNAL, value);
    same([await page.value(SIGNAL), await page.focused()], [value, SIGNAL], "the list after choosing");
  };
  const closeToasts = () => q(`document.querySelectorAll(".toast button").forEach((button) => button.click())`);
  const line = () => page.text(`#${ROUTE}`);
  const shown = async () => !(await q(`document.getElementById(${js(ROUTE)}).hidden`));
  let pack;

  await step("a deck saved before its pack gained signs keeps its own, and a chosen hand signal is saved", async () => {
    pack = (await api("GET", "/api/packs")).find((offered) => offered.id === "meeting");
    const defaults = Object.fromEntries(pack.decks[0].cards.map((card) => [card.id, card.signal ?? null]));
    same([defaults["wrap-up"], defaults.question, defaults["next-topic"]], [{ sign: "fist" }, { sign: "pointing" },
      { sign: "peace" }], "the pack's new signs");
    // The deck as the pack was before: only agree and disagree have a signal.
    const older = structuredClone(pack);
    for (const card of older.decks[0].cards.filter((card) => !["agree", "disagree"].includes(card.id))) {
      delete card.signal;
    }
    await closeToasts();
    await page.click("#tab-cards");
    await api("PUT", "/api/project", withPack(await project(), older));
    await waitFor("the saved meeting deck", async () => (await page.text("#deck-0-heading")) === "Meeting signals");
    await page.click("#deck-list .card-editor summary");
    const label = await q(`document.querySelector('label[for="${SIGNAL}"]').textContent`);
    same([label, await count(`#${SIGNAL} option`), await page.value(SIGNAL)], ["Hand signal", 19, ""],
      "the first card's list");
    await page.choose("deck-0-edit-card", "agree");
    same(await page.value(SIGNAL), "sign:thumbs_up", "Agree's signal");
    await page.choose("deck-0-edit-card", "cant-hear");
    await chooseSignal("sign:open_palm");
    same(await dirty(), "Unsaved changes", "save state");
    same(await notes(), [
      ["hint", "Agree and Disagree have a thumbs up and a thumbs down. Two hands can show two signs at once, so this " +
        "deck raises one of them then, not Can't hear you, since they come first."],
      ["hint", "Hands often open while talking, so an open palm can raise this card by accident."],
    ], "the two signs and open palm notes");
    check((await page.text("#deck-0-camera-note")).endsWith("3 of these cards have one."), "the camera note's count");
    await page.click("#save-button");
    await clean();
    const saved = await meeting();
    same(await signalOf("cant-hear"), { sign: "open_palm" }, "the chosen signal");
    const others = saved.cards.filter((card) => card.id !== "cant-hear");
    same(others, older.decks[0].cards.filter((card) => card.id !== "cant-hear"), "every other card as it was saved");
  });

  await step("a hand signal another card of the deck has is refused, naming both cards", async () => {
    await page.choose("deck-0-edit-card", "offline");
    await q(`document.getElementById(${js(SIGNAL)}).focus()`);
    await page.choose(SIGNAL, "sign:thumbs_up");
    const refusal = "Agree already uses a thumbs up in this deck. Choose another hand signal for Let's take this " +
      "offline, or None.";
    same([await attribute(`#${SIGNAL}`, "aria-invalid"), await page.text(`#${SIGNAL}-error`)], ["true", refusal],
      "the refusal");
    same(await attribute(`#${SIGNAL}`, "aria-describedby"), `${SIGNAL}-error`, "the list's description");
    same(await dirty(), "All changes saved", "the refused signal is not in the draft");
    await helpers.ctrlS();
    const blocked = (texts) => texts.some((text) => text.startsWith("Fix the highlighted field"));
    await waitFor("the blocked save alert", async () => blocked(await toasts("toast-error")));
    same([await page.focused(), await signalOf("offline")], [SIGNAL, undefined], "focus and the saved card");
    await closeToasts();
    await page.choose(SIGNAL, "");
    same([await attribute(`#${SIGNAL}`, "aria-invalid"), await dirty()], [null, "All changes saved"], "after None");
  });

  await step("a hand signal a listening deck also has is warned about, naming that deck and card", async () => {
    await page.click('[data-pack-id="poker"]');
    await waitFor("the poker deck", async () => (await page.text("#deck-1-heading")) === "Planning poker");
    await page.click("#deck-1-camera");
    same(await page.value("deck-0-edit-card"), "offline", "the card in the meeting deck's editor");
    await chooseSignal("fingers:5");
    // The meeting deck's open palm also counts as fingers, so Planning poker takes its 5 only while a vote is open.
    const vote = "so once this deck's hand signals are on too, a vote in Planning poker decides:";
    same(await notes(), [
      ["field-warning", `Planning poker: 5 also uses 5 fingers up, and its deck's hand signals are on, ${vote} both ` +
        "are raised while one is open, and only this card is raised while none is."],
      ["hint", "Agree and Disagree have a thumbs up and a thumbs down. A thumbs up or down on one hand adds no " +
        "fingers while the other hand shows the count, so this deck raises one of them then, not Let's take this " +
        "offline, since they come first."],
      ["hint", "Can't hear you has an open palm. An open palm also counts as 5 fingers, so this deck raises only " +
        "Can't hear you then, since it comes first, but only this card while a vote is open in Planning poker."],
    ], "the warning and the notes");
    same(await attribute(`#${SIGNAL}`, "aria-describedby"), `${SIGNAL}-notes`, "the list's description");
    await page.choose("deck-0-edit-card", "next-topic");
    await chooseSignal("sign:peace");
    const peace = (await notes()).map(([, text]) => text);
    check(peace.includes(`Planning poker: 2 has 2 fingers up. A peace sign also counts as 2 fingers, ${vote} 2 is ` +
      "raised while one is open, and this card while none is."), "the peace sign's one-hand note");
    check(peace.includes("Planning poker: 5 and 8 have 5 and 8 fingers up. The camera adds up both hands: a sign " +
      `on one hand and fingers on the other can make the count, ${vote} one of them is raised while one is open, ` +
      "and this card while none is."), "the peace sign's two-hand note");
    await page.click("#save-button");
    await clean();
    same([await signalOf("offline"), await signalOf("next-topic")], [{ fingers: 5 }, { sign: "peace" }],
      "signals with notes and warnings still save");
    same((await project()).decks.find((deck) => deck.id === "poker").camera, true, "the poker deck listens");
  });

  await step("the Cards tab says where hand signals go, and a poker card raised by the API opens a vote", async () => {
    same([await shown(), await attribute(`#${ROUTE}`, "role")], [false, "status"],
      "no line while the meeting deck's hand signals are off, and a live region");
    await page.click("#deck-0-camera");
    await page.click("#save-button");
    await clean();
    const none = "Hand signals go to Meeting signals: no vote is open in Planning poker.";
    await waitFor("the line without a vote", async () => (await shown()) && (await line()) === none);
    await api("POST", "/api/card", { deck: "poker", card: "p5", action: "raise" });
    const press = new RegExp("^Finger counts go to Planning poker: raising its card opened a vote until " +
      "\\d\\d:\\d\\d:\\d\\d, or until the card is lowered\\.$");
    await waitFor("the line with the vote a press opened", async () => press.test(await line()));
    const opened = await line();
    await api("POST", "/api/card", { deck: "poker", action: "lower" });
    await waitFor("the line once the card lowered", async () => (await line()) === none);
    return opened;
  });

  await step("the hand signal decks are deleted again", async () => {
    for (const left of [1, 0]) {
      await page.click('#deck-list [data-card-command="delete-deck"]');
      await page.click('#deck-list [data-card-command="delete-confirmed"]');
      await waitFor(`${left} decks`, async () => (await count("#deck-list article")) === left);
    }
    await page.click("#save-button");
    await clean();
    same((await project()).decks, [], "saved decks");
    await waitFor("no line on where hand signals go", async () => !(await shown()));
  });
}
