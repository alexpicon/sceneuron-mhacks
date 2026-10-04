// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises e2e/ws-server.mjs, e2e/room-stand-in.mjs, and e2e/room-probe.mjs: the handshake key and frame encoding of
// RFC 6455, masked and split frames, and the stand-in's protocol through the probe on Node's own WebSocket client: the
// identity first, the subscription, a reducer's answer to its caller, others' updates that say who voted but never
// what, a refused vote, and leaving when the connection closes.

import assert from "node:assert/strict";
import { test } from "node:test";

import { openProbe } from "../room-probe.mjs";
import { RoomStandIn } from "../room-stand-in.mjs";
import { acceptKey, decodeFrames, encodeFrame } from "../ws-server.mjs";

test("the handshake key and frames follow RFC 6455", () => {
  assert.equal(acceptKey("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
  assert.deepEqual([...encodeFrame(0x1, "Hi")], [0x81, 2, 0x48, 0x69]);
  assert.deepEqual([...encodeFrame(0x1, "x".repeat(200)).subarray(0, 4)], [0x81, 126, 0, 200]);
  assert.deepEqual([...encodeFrame(0x1, "x".repeat(70_000)).subarray(0, 2)], [0x81, 127]);
  const mask = [1, 2, 3, 4];
  const masked = Buffer.from([0x81, 0x80 | 2, ...mask, 0x48 ^ 1, 0x69 ^ 2]);
  const { frames, rest } = decodeFrames(Buffer.concat([masked, masked.subarray(0, 3)]));
  assert.deepEqual(
    frames.map((frame) => [frame.fin, frame.opcode, frame.payload.toString()]),
    [[true, 1, "Hi"]],
  );
  assert.equal(rest.length, 3, "a frame still arriving waits for its bytes");
});

/** The rows each table of an update inserts, parsed. */
const inserted = (tables) =>
  Object.fromEntries(tables.map((table) => [table.table_name, table.updates[0].inserts.map((row) => JSON.parse(row))]));

test("the stand-in answers callers, tells others who voted but not what, and refuses as the module does", async () => {
  const standIn = new RoomStandIn();
  const url = await standIn.start();
  try {
    const ana = await openProbe(url);
    assert.match(ana.identity, /^0x[0-9a-f]{64}$/);
    assert.deepEqual((await ana.subscribe(["SELECT * FROM room"])).database_update.tables, []);
    const created = inserted((await ana.call("create_room", ["Planning poker", "Ana"])).Committed.tables);
    const code = created.room[0][0];
    assert.match(code, /^[A-Z2-9]{6}$/);
    assert.deepEqual(created.participant[0].slice(0, 3), [[ana.identity], code, "Ana"]);

    const ben = await openProbe(url);
    const first = inserted((await ben.subscribe(["SELECT * FROM room"])).database_update.tables);
    assert.equal(first.room[0].code, code, "the first look has rows by field name");
    assert.ok((await ben.call("join_room", [code.toLowerCase(), "Ben"])).Committed);
    assert.equal((await ana.next("TransactionUpdateLight")).update.tables[0].table_name, "participant");

    assert.ok((await ana.call("cast_vote", ["p8"])).Committed);
    const ballot = await ben.next("TransactionUpdateLight");
    assert.equal(ballot.update.tables[0].table_name, "ballot");
    assert.ok(!JSON.stringify(ballot).includes("p8"), "a ballot never says what was voted");

    assert.ok((await ben.call("reveal", [])).Committed);
    assert.ok(JSON.stringify(await ana.next("TransactionUpdateLight")).includes("p8"), "the reveal publishes the vote");
    const refused = await ben.call("cast_vote", ["p3"]);
    assert.deepEqual(refused, { Failed: "the cards are already revealed; start the next round to vote again" });

    ben.close();
    const update = await ana.next("TransactionUpdateLight");
    const people = update.update.tables.find((table) => table.table_name === "participant");
    const gone = people.updates[0].deletes.map((row) => JSON.parse(row)[2]);
    assert.deepEqual(gone, ["Ben"], "a closed connection leaves its room");
    ana.close();
  } finally {
    await standIn.stop();
  }
});
