// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Steps of the shared room's live test (room-live.mjs), against two sceneuron-dev servers that share
// a SpacetimeDB: Ana creates a room and Ben joins it, each raises a different poker card, both
// overlays show the cover, and each sees that the other voted but not what. A client outside
// Sceneuron, on a raw WebSocket, checks the module itself: subscribed before anyone votes, it sees
// each ballot arrive but never a card in any message before the reveal, and the vote table cannot be
// queried. Ana's control token reveals, both overlays show their own number and the {{room}} summary
// shows both, the outside client sees the votes first in the reveal's own update, and the module
// refuses its vote after the reveal. Its connection ending takes it out of the room, the next round
// lowers both cards, quitting Ben's Sceneuron takes him out, and once Ana leaves the empty room is
// gone.

import { withPack } from "./card-steps.mjs";
import { waitFor } from "./harness.mjs";
import { openProbe } from "./room-probe.mjs";

const COVER = "✓ Voted";
const DECK = "poker";
const SLOT = "poker-card";
const SUMMARY = "room-summary";
/** The module's public tables. */
const TABLES = ["room", "participant", "ballot", "result"];

const element = (state, id) => state.elements.find((candidate) => candidate.id === id);
const deck = (state) => state.decks.find((candidate) => candidate.id === DECK);

/** Runs the steps; `people` are `{ name, api, control }` for Ana and Ben, and `room` the SpacetimeDB to use. */
export async function runRoomSteps({ step, same, people, room }) {
  const [ana, ben] = people;
  const slot = async (person) => {
    const state = await person.api("GET", "/api/state");
    const props = element(state, SLOT).props;
    return { text: props.text, visible: props.visible, raised: deck(state).raised, covered: deck(state).covered };
  };
  const summary = async (person) => element(await person.api("GET", "/api/state"), SUMMARY).props.text;
  const both = (what, probe, timeoutMs = 10_000) =>
    Promise.all(people.map((person) => waitFor(`${person.name}: ${what}`, () => probe(person), timeoutMs)));
  let code;

  await step("each Sceneuron adds the poker pack and a {{room}} element", async () => {
    for (const person of people) {
      const pack = (await person.api("GET", "/api/packs")).find((offered) => offered.id === DECK);
      const project = withPack(await person.api("GET", "/api/project"), pack);
      project.elements.push({
        id: SUMMARY,
        name: "Room summary",
        kind: "text",
        props: { x: 40, y: 40, width: 900, height: 120, text: "{{room}}", background: "#111827ff" },
      });
      await person.api("PUT", "/api/project", project);
      same((await person.api("GET", "/api/room")).joined, false, `${person.name} before joining`);
    }
  });

  await step("Ana creates a room on the local SpacetimeDB", async () => {
    const state = await ana.api("POST", "/api/room", { action: "create", name: ana.name, ...room });
    code = state.code;
    if (!/^[A-Z0-9]{6}$/.test(code ?? "")) throw new Error(`the room code is ${JSON.stringify(code)}`);
    same([state.round, state.revealed, state.deck_name], [1, false, "Planning poker"], "the new room");
    return `room ${code}`;
  });

  await step("Ben joins it with the code, typed in lowercase", async () => {
    const join = { action: "join", code: code.toLowerCase(), name: ben.name, ...room };
    const state = await ben.api("POST", "/api/room", join);
    same(state.code, code, "the room Ben is in");
    same(state.participants.map((person) => person.name), [ana.name, ben.name], "the people in the room");
    await waitFor("Ana to see Ben", async () => (await ana.api("GET", "/api/room")).participants.length === 2);
  });

  let outside;
  let identities;
  // The identity message carries a token, whose random letters could spell a card id by chance.
  const votesSeen = () => outside.texts.filter((text) => !text.startsWith('{"IdentityToken"') && /p3|p8/.test(text));
  /** The rows of `table` that updates inserted after the outside client's first look. */
  const inserted = (table) =>
    outside.texts
      .map((text) => JSON.parse(text))
      .map((message) => message.TransactionUpdateLight?.update ?? message.TransactionUpdate?.status.Committed)
      .flatMap((update) => update?.tables ?? [])
      .filter((changed) => changed.table_name === table)
      .flatMap((changed) => changed.updates.flatMap((update) => update.inserts));
  await step("a client outside Sceneuron subscribes to the room's public tables before anyone votes", async () => {
    outside = await openProbe(room.server, room.database);
    const first = await outside.subscribe(TABLES.map((table) => `SELECT * FROM ${table}`));
    const rows = (name) => first.database_update.tables.find((table) => table.table_name === name)?.updates[0].inserts;
    same([rows("participant")?.length, rows("ballot"), rows("result")], [2, undefined, undefined], "rows it sees");
    identities = rows("participant").map((row) => JSON.parse(row).identity.__identity__).sort();
  });

  await step("each raises a poker card through /api/card and sees only the cover", async () => {
    await ana.api("POST", "/api/card", { deck: DECK, card: "p3", action: "raise" });
    await ben.api("POST", "/api/card", { deck: DECK, card: "p8", action: "raise" });
    for (const [person, card] of [[ana, "p3"], [ben, "p8"]]) {
      same(await slot(person), { text: COVER, visible: true, raised: card, covered: true }, `${person.name}'s slot`);
    }
  });

  await step("each sees that the other voted, but not what", async () => {
    await both("both votes", async (person) => {
      const state = await person.api("GET", "/api/room");
      return state.participants.every((participant) => participant.voted);
    });
    for (const [person, hidden] of [[ana, "p8"], [ben, "p3"]]) {
      const state = await person.api("GET", "/api/room");
      if (JSON.stringify(state).includes(hidden)) throw new Error(`${person.name} can read ${hidden}`);
      same(state.results, [], `${person.name}'s results before the reveal`);
      same(await summary(person), "Voted: 2 of 2", `${person.name}'s {{room}}`);
    }
  });

  await step("the outside client saw who voted but not what, and cannot query the votes", async () => {
    await waitFor("both ballots", () => inserted("ballot").length === 2);
    same(inserted("ballot").map((row) => JSON.parse(row)[0][0]).sort(), identities, "who it saw vote");
    same(votesSeen(), [], "messages that name a vote");
    const messages = outside.texts.length;
    const queried = await outside.query("SELECT * FROM vote");
    if (!/no such table: `vote`.*private/.test(queried.error?.some ?? "")) {
      throw new Error(`a query of the vote table answered ${JSON.stringify(queried)}`);
    }
    const sql = await fetch(`${room.server}/v1/database/${room.database}/sql`, { method: "POST", body: "SELECT * FROM vote" });
    const answer = await sql.text();
    if (sql.status !== 400 || !answer.includes("private")) throw new Error(`HTTP SQL answered ${sql.status}: ${answer}`);
    return `no card in its ${messages} messages; the vote table is private to the WebSocket and HTTP APIs`;
  });

  await step("Ana's control token reveals, and gets no names or votes back", async () => {
    const reply = await ana.control("POST", "/api/room", { action: "reveal" });
    same(reply, { ok: true }, "the control token's answer");
  });

  await step("both overlays show their own card and the {{room}} spread at once", async () => {
    const started = Date.now();
    const seen = await both("the uncovered card", async (person) => {
      const shown = await slot(person);
      return shown.text !== COVER && Date.now();
    });
    same(await slot(ana), { text: "3", visible: true, raised: "p3", covered: false }, "Ana's slot");
    same(await slot(ben), { text: "8", visible: true, raised: "p8", covered: false }, "Ben's slot");
    for (const [person, spread] of [
      [ana, "You 3 · Ben 8 · Average 5.5"],
      [ben, "Ana 3 · You 8 · Average 5.5"],
    ]) same(await summary(person), spread, `${person.name}'s {{room}}`);
    const results = (await ben.api("GET", "/api/room")).results.map((result) => [result.name, result.label]);
    same(results, [[ana.name, "3"], [ben.name, "8"]], "the results Ben sees");
    return `both showed their own card within ${Math.max(...seen) - started} ms of the reveal's answer`;
  });

  await step("the outside client first sees the votes in the reveal's own update", async () => {
    await waitFor("the revealed votes", () => votesSeen().length > 0);
    const update = JSON.parse(votesSeen()[0]).TransactionUpdateLight.update;
    const changed = Object.fromEntries(update.tables.map((table) => [table.table_name, table.updates[0].inserts]));
    same(Object.keys(changed).sort(), ["result", "room"], "the tables of that update");
    same(JSON.parse(changed.room[0])[3], true, "the room's revealed flag in that update");
    same(changed.result.map((row) => JSON.parse(row)[3]).sort(), ["p3", "p8"], "the cards it publishes");
  });

  await step("the module refuses a vote after the reveal", async () => {
    const joined = await outside.call("join_room", [code, "Outside"]);
    if (!joined.Committed) throw new Error(`joining answered ${JSON.stringify(joined)}`);
    const refused = await outside.call("cast_vote", ["p5"]);
    same(refused, { Failed: "the cards are already revealed; start the next round to vote again" }, "the vote");
    await waitFor("Ana to see three people", async () => (await ana.api("GET", "/api/room")).participants.length === 3);
  });

  await step("a client whose connection ends leaves the room", async () => {
    outside.close();
    await both("two people again", async (person) => (await person.api("GET", "/api/room")).participants.length === 2);
  });

  await step("Ben starts the next round, which lowers both cards", async () => {
    const state = await ben.api("POST", "/api/room", { action: "next" });
    same([state.round, state.revealed], [2, false], "the new round");
    await both("the lowered deck", async (person) => {
      const shown = await slot(person);
      return shown.raised === null && !shown.visible;
    });
    for (const person of people) same(await summary(person), "Voted: 0 of 2", `${person.name}'s {{room}}`);
  });

  await step("quitting Ben's Sceneuron takes him out of the room", async () => {
    await ben.server.stop();
    await waitFor("Ana alone", async () => (await ana.api("GET", "/api/room")).participants.length === 1);
    same(await summary(ana), "Voted: 0 of 1", "Ana's {{room}}");
  });

  await step("Ana leaves, and the empty room is gone", async () => {
    await ana.api("POST", "/api/room", { action: "leave" });
    const rejoin = await ana.raw("POST", "/api/room", { action: "join", code, name: ana.name, ...room });
    same(rejoin.status, 400, "joining the deleted room");
    if (!rejoin.body.error.includes(code)) throw new Error(`the refusal says ${JSON.stringify(rejoin.body.error)}`);
  });
}
