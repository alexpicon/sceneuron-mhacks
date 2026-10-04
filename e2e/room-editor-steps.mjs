// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Room steps of the browser regression test (editor-browser.mjs): the Cards tab's Room section offers
// the saved decks and the default SpacetimeDB server, points at a missing name, and reports a server
// that does not answer. Then it joins a room on a stand-in SpacetimeDB (room-stand-in.mjs) through
// Sceneuron's real client, with another participant on a raw WebSocket, and checks where focus goes
// when a reveal or leaving from the API hides the focused button. These steps need no SpacetimeDB;
// room-live.mjs checks a real room by hand. They run after the Cards tab steps, which leave no deck.

import { isDeepStrictEqual } from "node:util";

import { withPack } from "./card-steps.mjs";
import { waitFor } from "./cdp.mjs";
import { editorHelpers } from "./editor-steps.mjs";
import { openProbe } from "./room-probe.mjs";
import { freePort } from "./spacetime-server.mjs";

/** Runs the Room steps; `step(name, body)` reports each one, and `roomServer` is the stand-in's address. */
export async function runRoomEditorSteps({ page, step, same, api, roomServer }) {
  const { q, check, toasts, attribute } = editorHelpers(page, api);
  const hidden = (id) => q(`document.getElementById(${JSON.stringify(id)}).hidden`);
  const deckOptions = () => q(`[...document.getElementById("room-deck").options].map((option) => option.textContent)`);

  await step("the Room section offers the saved decks and the local SpacetimeDB", async () => {
    await page.click("#tab-cards");
    await waitFor("the decks' note", async () => (await deckOptions()).length === 0);
    same(await q(`document.getElementById("room-create").disabled`), true, "Create without a saved deck");
    const pack = (await api("GET", "/api/packs")).find((offered) => offered.id === "poker");
    await api("PUT", "/api/project", withPack(await api("GET", "/api/project"), pack));
    await waitFor("the saved poker deck", async () => (await deckOptions()).includes("Planning poker"));
    same(await q(`document.getElementById("room-create").disabled`), false, "Create with a saved deck");
    same(
      [await page.value("room-server"), await page.value("room-database"), await page.value("room-deck")],
      ["http://127.0.0.1:3000", "sceneuron-room", "poker"],
      "the defaults",
    );
    same([await hidden("room-setup"), await hidden("room-joined")], [false, true], "not in a room");
    const labels = await q(`["room-name", "room-deck", "room-code", "room-server", "room-database"]
      .map((id) => document.querySelector('label[for="' + id + '"]')?.textContent)`);
    same(labels, ["Your name", "Deck that votes", "Room code", "Server address", "Database"], "labels");
  });

  await step("creating a room without a name points at the name", async () => {
    await page.click("#room-create");
    same([await page.focused(), await attribute("#room-name", "aria-invalid")], ["room-name", "true"], "the name");
    await page.fill("room-code", "abc");
    await page.fill("room-name", "Ana");
    await page.click("#room-join");
    same([await page.focused(), await attribute("#room-code", "aria-invalid")], ["room-code", "true"], "the code");
    same(await attribute("#room-name", "aria-invalid"), null, "the fixed name");
  });

  await step("a SpacetimeDB that does not answer is reported, and no room is joined", async () => {
    const port = await freePort();
    await page.click(".room-server summary");
    await page.fill("room-server", `http://127.0.0.1:${port}`);
    await page.click("#room-create");
    const refused = `cannot reach the rooms at http://127.0.0.1:${port}`;
    await waitFor("the refusal", async () => (await toasts("toast-error")).some((text) => text.startsWith(refused)));
    same([await hidden("room-setup"), await hidden("room-joined")], [false, true], "still not in a room");
    await waitFor("the buttons back", async () => !(await q(`document.getElementById("room-create").disabled`)));
    const state = await api("GET", "/api/room");
    same([state.joined, state.name, state.server], [false, "Ana", `http://127.0.0.1:${port}`], "the room's state");
    check((await page.text("#room-error")) === "" && (await hidden("room-error")), "an inline error");
    await q(`document.querySelectorAll(".toast button").forEach((button) => button.click())`);
  });

  let ben;
  await step("a room on a stand-in SpacetimeDB shows who is in it and who voted", async () => {
    await page.fill("room-server", roomServer);
    await page.click("#room-create");
    await waitFor("the room", async () => !(await hidden("room-joined")));
    const { code } = await api("GET", "/api/room");
    ben = await openProbe(roomServer);
    for (const [reducer, args] of [["join_room", [code, "Ben"]], ["cast_vote", ["p8"]]]) {
      const status = await ben.call(reducer, args);
      if (!status.Committed) throw new Error(`${reducer} answered ${JSON.stringify(status)}`);
    }
    const people = () => q(`[...document.querySelectorAll("#room-people li")].map((item) => item.textContent)`);
    await waitFor("Ben's vote", async () => isDeepStrictEqual(await people(), ["Ana (you): not yet", "Ben: voted"]));
    same(await page.text("#room-status"), `Room ${code}, round 1: 1 of 2 voted.`, "the status");
    return `room ${code}`;
  });

  await step("a reveal from the API while Reveal has focus moves focus to the room's status", async () => {
    await q(`document.getElementById("room-reveal").focus()`);
    same(await page.focused(), "room-reveal", "focus before the reveal");
    await api("POST", "/api/room", { action: "reveal" });
    await waitFor("focus on the status", async () => (await page.focused()) === "room-status");
    same(
      [await hidden("room-reveal"), await page.text("#room-spread")],
      [true, "Revealed: Ben 8 · Average 8."],
      "the revealed room",
    );
  });

  await step("leaving from the API while Next round has focus moves focus to the Room heading", async () => {
    await q(`document.getElementById("room-next").focus()`);
    await api("POST", "/api/room", { action: "leave" });
    await waitFor("focus on the heading", async () => (await page.focused()) === "room-heading");
    same([await hidden("room-setup"), await hidden("room-joined")], [false, true], "out of the room");
    ben.close();
  });
}
