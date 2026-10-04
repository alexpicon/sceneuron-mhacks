// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Cards tab steps of the browser regression test (editor-browser.mjs): adding the planning poker pack,
// following card slot edits made on other tabs while keeping refused entries and redrawing once the
// last one is fixed, raising a card from the keyboard and lowering it while the preview shows it,
// editing a card's name, text, and colors and the deck's settings, pairing a Stream Deck and revoking
// it, and deleting decks from the Cards tab and the element list. They run last, against the project
// the earlier steps leave.

import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";

const js = JSON.stringify;

/** Runs the Cards tab steps; `step(name, body)` reports each one. */
export async function runCardEditorSteps({ page, step, same, api, base }) {
  const helpers = editorHelpers(page, api);
  const { q, check, count, item, node, previewText, dirty, toasts, project, state, clean, attribute } = helpers;
  const card = (id) => `#deck-list [data-card-id=${js(id)}]`;
  const cardStyle = (id, name) => q(`document.querySelector(${js(card(id))}).style.getPropertyValue(${js(name)})`);
  const status = () => page.text("#deck-list article:first-child .deck-status");
  const focusedCommand = () => q(`document.activeElement.getAttribute("data-card-command")`);
  const isGhost = () => q(`document.querySelector(${js(node("poker-card"))})?.classList.contains("sn-ghost")`);
  const deck = async () => (await project()).decks.find((candidate) => candidate.id === "poker");
  const control = (path, token, init = {}) =>
    fetch(`${base}${path}`, { ...init, headers: { "X-Sceneuron-Token": token, "Content-Type": "application/json" } });
  // Sets a control's value as dragging a picker or slider, or clearing a field, does: the value, then an input event.
  const setValue = (id, value) => q(`(() => {
    const input = document.getElementById(${js(id)});
    input.value = ${js(value)};
    input.dispatchEvent(new Event("input", { bubbles: true }));
  })()`);
  const nameField = `document.getElementById("deck-0-card-name")`;
  const editSlotColor = async (color) => {
    await page.click("#tab-properties");
    await helpers.selected("Poker card");
    await setValue("prop-color", color);
    await page.click("#tab-cards");
  };
  // Notifications from the steps before stack over the bottom of the element list.
  const closeToasts = () => q(`document.querySelectorAll(".toast button").forEach((button) => button.click())`);
  let pokerPack;
  let controlToken;

  await step("the Cards tab adds the planning poker pack to the draft", async () => {
    await page.click("#tab-cards");
    const names = `[...document.querySelectorAll("#pack-list .pack-name")].map((span) => span.textContent)`;
    await waitFor("the packs", async () => (await q(`${names}.length`)) === 6);
    const offered = ["Meeting signals", "Planning poker", "Classroom", "Status", "Subtitles", "Live description"];
    same(await q(names), offered, "packs");
    pokerPack = (await api("GET", "/api/packs")).find((pack) => pack.id === "poker");
    await page.click('[data-pack-id="poker"]');
    await waitFor("the new deck", async () => (await page.text("#deck-0-heading")) === "Planning poker");
    same(await page.focused(), "deck-0-heading", "focus after adding");
    const outline = await q(`[...document.querySelectorAll("#panel-cards :is(h3, h4)")]
      .map((heading) => heading.tagName + " " + heading.textContent)`);
    same(outline, ["H3 Decks", "H4 Planning poker", "H3 Add a pack", "H3 Room", "H3 Stream Deck"], "headings");
    same(await dirty(), "Unsaved changes", "save state");
    same(await page.text(`${item("poker-card")} .element-meta`), "Text, hidden, not saved", "the card slot");
    same(await status(), "Planning poker is not saved yet. Save to raise these cards live.", "status before saving");
    same(await q(`document.querySelector(${js(card("p5"))}).disabled`), true, "card button before saving");
    await page.click('[data-pack-id="poker"]');
    await waitFor("the refusal", async () => (await toasts("toast-error")).includes(
      "Planning poker is already in this project.",
    ));
    await q(`document.querySelector(".toast-error button").click()`);
    await page.click("#save-button");
    await clean();
    same(await deck(), pokerPack.decks[0], "saved deck");
    check((await project()).elements.some((element) => element.id === "poker-card"), "no saved card slot");
  });

  await step("the Cards tab follows card slot edits made on other tabs and keeps a refused entry", async () => {
    const slotColor = (await project()).elements.find((element) => element.id === "poker-card").props.color;
    same(await cardStyle("p1", "--card-color"), slotColor, "card color before the edit");
    await closeToasts();
    await page.click(item("poker-card"));
    await editSlotColor("#00ff00");
    same(await dirty(), "Unsaved changes", "save state");
    same(await cardStyle("p1", "--card-color"), "#00ff00", "card color after the edit");
    // The editor shows the first card, which has no color of its own and so shows the slot's.
    same(await page.value("deck-0-card-color"), "#00ff00", "the card editor's text color");
    await page.choose("deck-0-edit-card", "p1");
    await setValue("deck-0-card-name", "");
    same(await attribute("#deck-0-card-name", "aria-invalid"), "true", "invalid flag");
    await page.click("#tab-properties");
    await helpers.ctrlS();
    const blocked = (texts) => texts.some((text) => text.startsWith("Fix the highlighted field"));
    await waitFor("the blocked save alert", async () => blocked(await toasts("toast-error")));
    same(await attribute("#tab-cards", "aria-selected"), "true", "the Cards tab is shown");
    same([await page.focused(), await attribute("#deck-0-card-name", "aria-invalid")], ["deck-0-card-name", "true"],
      "the refused entry");
    await closeToasts();
    await page.click("#discard-button");
    await clean();
    same(await cardStyle("p1", "--card-color"), slotColor, "card color after discarding");
  });

  await step("a card slot edit reaches the Cards tab once the last refused card entry is fixed", async () => {
    const slotColor = await cardStyle("p2", "--card-color");
    await page.choose("deck-0-edit-card", "p1");
    await setValue("deck-0-card-name", "O\u0007e");
    await setValue("deck-0-card-text", "a\u0007b");
    await editSlotColor("#00ff00");
    await page.fill("deck-0-card-text", "ab");
    same([await page.value("deck-0-card-name"), await attribute("#deck-0-card-name", "aria-invalid")],
      ["O\u0007e", "true"], "the refused name after fixing the text");
    same(await cardStyle("p2", "--card-color"), slotColor, "a card without its own color before the last fix");
    // The last fix is typed with the caret inside the name, where the redraw keeps it.
    await q(`${nameField}.focus(); ${nameField}.setSelectionRange(1, 2)`);
    await page.type("n");
    same([await page.value("deck-0-card-name"), await page.focused(), await q(`${nameField}.selectionStart`)],
      ["One", "deck-0-card-name", 2], "the name, focus, and caret after the last fix");
    same([await cardStyle("p2", "--card-color"), await page.value("deck-0-card-color")], ["#00ff00", "#00ff00"],
      "a card without its own color and the card editor's text color after the last fix");
    // The decks redraw once: a later edit keeps the fields.
    await q(`${nameField}.dataset.probe = "1"`);
    await setValue("deck-0-card-color-alpha", "50");
    same(await q(`${nameField}.dataset.probe`), "1", "the name field after another edit");
    same([await cardStyle("p1", "--card-color"), await cardStyle("p2", "--card-color")], ["#00ff0080", "#00ff00"],
      "card colors after changing only the alpha");
  });

  await step("an alpha moved before the fix uses the new slot color, and an input method's fix is kept", async () => {
    await page.choose("deck-0-edit-card", "p3");
    await setValue("deck-0-card-name", "");
    await editSlotColor("#0000ff");
    // The alpha moves while the picker still shows the old slot color, and an input method composes the fix.
    await setValue("deck-0-card-color-alpha", "50");
    await q(`${nameField}.focus()`);
    await page.send("Input.imeSetComposition", { text: "に", selectionStart: 1, selectionEnd: 1 });
    await page.send("Input.imeSetComposition", { text: "にほ", selectionStart: 2, selectionEnd: 2 });
    await page.type("日本");
    same(await page.value("deck-0-card-name"), "日本", "the composed name");
    same([await cardStyle("p3", "--card-color"), await cardStyle("p2", "--card-color")], ["#0000ff80", "#0000ff"],
      "card colors once the composition ends");
    await page.click("#discard-button");
    await clean();
    await page.click("#deck-list .card-editor summary");
    same(await q(`document.querySelector("#deck-list .card-editor").open`), false, "card editor closed");
  });

  await step("a card raised from the keyboard shows in the preview until it is lowered", async () => {
    await waitFor("the card buttons", async () => !(await q(`document.querySelector(${js(card("p5"))}).disabled`)));
    same(await status(), "Planning poker: no card is up.", "status before raising");
    await q(`document.querySelector(${js(card("p5"))}).focus()`);
    await page.key(" ", { code: "Space", text: " " });
    await waitFor("the pressed card", async () => (await attribute(card("p5"), "aria-pressed")) === "true");
    same(await q(`document.activeElement.getAttribute("data-card-id")`), "p5", "focus after Space");
    await waitFor("the raised card in the preview", async () => (await previewText("poker-card")) === "5");
    check(!(await isGhost()), "the card slot is still drawn as hidden");
    same(await status(), "Planning poker shows 5.", "status");
    same((await state()).decks.find((view) => view.id === "poker").raised, "p5", "server's raised card");
    await page.click('#deck-list [data-card-command="lower"]');
    await waitFor("the lowered card", async () => (await status()) === "Planning poker: no card is up.");
    await waitFor("the hidden card slot", () => isGhost());
    same(await attribute(card("p5"), "aria-pressed"), "false", "pressed card after lowering");
  });

  await step("a card's name, text, and colors and the deck's settings are saved, keeping its signal", async () => {
    await page.click("#deck-list .card-editor summary");
    await page.choose("deck-0-edit-card", "p5");
    await waitFor("the card in the editor", async () => (await page.value("deck-0-card-name")) === "5");
    same(await page.value("deck-0-card-signal"), "fingers:5", "signal");
    await page.fill("deck-0-card-name", "Five points");
    same(await page.text(`${card("p5")} [data-card-name]`), "Five points", "card button's name");
    await page.fill("deck-0-card-text", "Five");
    same(await dirty(), "Unsaved changes", "save state");
    await setValue("deck-0-card-background", "#ff0000");
    await setValue("deck-0-card-background-alpha", "50");
    same(await page.text("#deck-0-card-background-output"), "50%", "alpha output");
    same(await cardStyle("p5", "--card-background"), "#ff000080", "card button's background");
    await page.choose("deck-0-auto-hide", "15000");
    await page.click("#deck-0-camera");
    await page.click("#save-button");
    await clean();
    const saved = await deck();
    same([saved.auto_hide_ms, saved.camera], [15000, true], "deck settings");
    const five = saved.cards.find((candidate) => candidate.id === "p5");
    const edited = ["Five points", { text: "Five", background: "#ff000080" }, { fingers: 5 }];
    same([five.name, five.props, five.signal], edited, "edited card");
    const others = (cards) => cards.filter((other) => other.id !== "p5");
    same(others(saved.cards), others(pokerPack.decks[0].cards), "the other cards");
    same(await q(`document.querySelector("#deck-list .card-editor").open`), true, "editor still open");
    await page.click(card("p5"));
    await waitFor("the edited card in the preview", async () => (await previewText("poker-card")) === "Five");
    same(await status(), "Planning poker shows Five points.", "status");
    const hideAt = await page.text('#deck-list [data-live="hide-at"]');
    check(/^It lowers by itself at \d\d:\d\d:\d\d\.$/.test(hideAt), `hide time: ${hideAt}`);
    await page.click('#deck-list [data-card-command="lower"]');
    await waitFor("the lowered card", async () => (await status()) === "Planning poker: no card is up.");
  });

  await step("connecting a Stream Deck shows a one-time code that trades for the control token", async () => {
    await page.click("#pair-stream-deck");
    await waitFor("the code", async () => /^\d{3} \d{3}$/.test(await page.text("#pairing-digits")));
    same(await page.focused(), "pairing-code", "focus");
    same(await page.text("#pairing-port"), new URL(base).port, "port");
    const expiry = await page.text("#pairing-expiry");
    check(/^It works once, for [12]:\d\d\.$/.test(expiry), `expiry: ${expiry}`);
    const code = (await page.text("#pairing-digits")).replace(" ", "");
    const paired = await fetch(`${base}/api/control/pair`, { method: "POST", body: js({ code }) });
    same(paired.status, 200, "pairing reply");
    controlToken = (await paired.json()).token;
    const view = await control("/api/control/state", controlToken);
    same(view.status, 200, "control view with the new token");
    check((await view.json()).decks.some((candidate) => candidate.id === "poker"), "no poker deck in the control view");
  });

  await step("Disconnect Stream Decks asks first, then revokes the control token", async () => {
    await page.click("#revoke-stream-decks");
    same(await page.focused(), "revoke-confirmed", "focus in the confirmation");
    await page.key("Escape");
    same([await q(`document.getElementById("revoke-confirm").hidden`), await page.focused()],
      [true, "revoke-stream-decks"], "after Escape");
    same((await control("/api/control/state", controlToken)).status, 200, "token before confirming");
    await page.click("#revoke-stream-decks");
    await page.click("#revoke-confirmed");
    const done = "Disconnected every Stream Deck. Connect one again to use it.";
    await waitFor("the notification", async () => (await toasts()).includes(done));
    same(await q(`document.getElementById("pairing").hidden`), true, "pairing code after revoking");
    same(await page.focused(), "revoke-stream-decks", "focus after revoking");
    same((await control("/api/control/state", controlToken)).status, 401, "revoked token");
  });

  await step("deleting a deck in the Cards tab asks first and offers to delete its card slot too", async () => {
    await page.click('#deck-list [data-card-command="delete-deck"]');
    same(await focusedCommand(), "delete-confirmed", "focus");
    await page.key("Escape");
    same([await q(`document.querySelector("#deck-list [data-confirm]").hidden`), await focusedCommand()],
      [true, "delete-deck"], "after Escape");
    await page.click('#deck-list [data-card-command="delete-deck"]');
    same(await q(`document.getElementById("deck-0-delete-slot").checked`), true, "card slot choice");
    await page.click('#deck-list [data-card-command="delete-confirmed"]');
    await waitFor("no decks", async () => (await count("#deck-list article")) === 0);
    same([await q(`document.getElementById("decks-empty").hidden`), await page.focused()], [false, "decks-heading"],
      "empty note and focus");
    same(await count(item("poker-card")), 0, "card slot in the element list");
    await page.click("#save-button");
    await clean();
    const saved = await project();
    same([saved.decks, saved.elements.some((element) => element.id === "poker-card")], [[], false], "saved project");
  });

  await step("deleting a card slot in the element list removes the deck it shows", async () => {
    await page.click('[data-pack-id="meeting"]');
    await waitFor("the meeting deck", async () => (await page.text("#deck-0-heading")) === "Meeting signals");
    await page.click(item("meeting-card"));
    await helpers.selected("Meeting card");
    await closeToasts();
    await page.click("#delete-element");
    await waitFor("no decks", async () => (await count("#deck-list article")) === 0);
    await page.click("#discard-button");
    await clean();
  });
}
