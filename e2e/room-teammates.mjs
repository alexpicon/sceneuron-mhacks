// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Simulated teammates for a solo demo of the shared planning poker room. Each teammate is its own connection to the
// room's SpacetimeDB database, with its own identity, so the module counts it as one more person: they join the room
// by its code, vote one after another a few seconds apart, wait for the reveal, and vote again in each next round.
// Their cards cluster around one value, as a real team's do. Ctrl+C disconnects them, which takes them out of the
// room. Run by hand.
//
// Usage: node e2e/room-teammates.mjs CODE [--server URL] [--database NAME] [--names A,B,C] [--cards p3,p5,p5]
//                                          [--wait MS] [--gap MS]
//   CODE        the six-character room code the overlay shows
//   --server    SpacetimeDB's address (default http://127.0.0.1:3000, where `spacetime start` listens)
//   --database  the published room module (default sceneuron-room)
//   --names     who joins (default Maya,Sam,Priya,Leo)
//   --cards     each teammate's card in every round, by card id, such as p3 (default: picked anew each round)
//   --wait      milliseconds after a round starts before the first vote (default 4000)
//   --gap       milliseconds between two teammates' votes, give or take a third (default 2500)

import { parseArgs } from "node:util";

import { openProbe } from "./room-probe.mjs";

/** Story points in order, so the default cards can sit around one value. */
const SCALE = ["p1", "p2", "p3", "p5", "p8", "p13"];

const { values: options, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    server: { type: "string", default: "http://127.0.0.1:3000" },
    database: { type: "string", default: "sceneuron-room" },
    names: { type: "string", default: "Maya,Sam,Priya,Leo" },
    cards: { type: "string" },
    wait: { type: "string", default: "4000" },
    gap: { type: "string", default: "2500" },
  },
});
const code = (positionals[0] ?? "").trim().toUpperCase();
// Fresh room codes are 6 characters without look-alikes, and chosen ones, such as CHASKI, 4 to 12 (room/module/src/rules.rs).
if (!/^[A-Z0-9]{4,12}$/.test(code)) {
  console.error(
    `${JSON.stringify(code)} is not a room code. Create a room in the editor's Cards tab, Room section, and give the ` +
      "code it shows, such as K7M2QX, or the practice room CHASKI.",
  );
  process.exit(2);
}
const names = options.names.split(",").map((name) => name.trim()).filter(Boolean);
const fixed = options.cards ? options.cards.split(",").map((card) => card.trim()) : null;
const waitMs = Number(options.wait);
const gapMs = Number(options.gap);

const sleep = (ms) => new Promise((wake) => setTimeout(wake, ms));
const pick = (list) => list[Math.floor(Math.random() * list.length)];

/** Each teammate's card this round: the given ones, or values around one picked middle. */
function cardsForRound() {
  if (fixed) return names.map((_, index) => fixed[index % fixed.length]);
  const middle = 2 + Math.floor(Math.random() * 3);
  return names.map(() => SCALE[Math.min(SCALE.length - 1, Math.max(0, middle + pick([-1, 0, 0, 0, 1])))]);
}

/** The room's round and whether it is revealed, from a row the database sends as an array or an object. */
function roomState(row) {
  const value = typeof row === "string" ? JSON.parse(row) : row;
  return Array.isArray(value) ? { round: value[2], revealed: value[3] } : { round: value.round, revealed: value.revealed };
}

/** The room as the database has it now, or null once it is gone. */
async function readRoom(probe) {
  const answer = await probe.query(`SELECT * FROM room WHERE code = '${code}'`);
  const rows = (answer.tables ?? []).flatMap((table) => table.rows ?? []);
  return rows.length ? roomState(rows[0]) : null;
}

const teammates = [];
let stopping = false;

async function stop(exitCode) {
  stopping = true;
  for (const { probe } of teammates) probe.close();
  console.log("Teammates disconnected, which takes them out of the room.");
  process.exit(exitCode);
}
process.on("SIGINT", () => stop(0));

for (const name of names) {
  const probe = await openProbe(options.server, options.database);
  const status = await probe.call("join_room", [code, name]);
  if (!status.Committed) {
    console.error(`${name} could not join room ${code}: ${JSON.stringify(status.Failed ?? status)}`);
    await stop(1);
  }
  teammates.push({ name, probe });
  console.log(`${name} joined room ${code}`);
}

let round = null;
for (;;) {
  const room = await readRoom(teammates[0].probe);
  if (!room) {
    console.log(`Room ${code} is gone.`);
    await stop(0);
  }
  if (room.round !== round && !room.revealed) {
    round = room.round;
    console.log(`Round ${round}: voting in ${Math.round(waitMs / 1000)} s`);
    const cards = cardsForRound();
    await sleep(waitMs);
    for (const [index, { name, probe }] of teammates.entries()) {
      if (stopping) break;
      const now = await readRoom(probe);
      if (!now || now.round !== round || now.revealed) break;
      const status = await probe.call("cast_vote", [cards[index]]);
      console.log(status.Committed ? `${name} voted (${cards[index]})` : `${name}'s vote: ${JSON.stringify(status)}`);
      if (index < teammates.length - 1) await sleep(gapMs * (0.67 + Math.random() * 0.66));
    }
    console.log("Waiting for the reveal and the next round. Ctrl+C to leave.");
  }
  await sleep(750);
}
