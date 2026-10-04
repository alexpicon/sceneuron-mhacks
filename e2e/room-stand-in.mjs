// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A stand-in for a SpacetimeDB server running room/module, for the browser regression test (editor-browser.mjs): it
// speaks the WebSocket text protocol v1.json.spacetimedb on /v1/database/sceneuron-room/subscribe with the shapes
// SpacetimeDB 2.10.2 sends, and follows the module's rules for creating, joining, leaving, voting, revealing, and the
// next round, so the browser test drives a joined room through Sceneuron's real client without SpacetimeDB.
// room-live.mjs checks the real module.

import { randomBytes } from "node:crypto";
import { createServer } from "node:http";

import { upgrade } from "./ws-server.mjs";

const PROTOCOL = "v1.json.spacetimedb";
const DATABASE = "sceneuron-room";
const ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const TABLES = ["room", "participant", "ballot", "result"];
const js = JSON.stringify;

const stamp = (micros) => ({ __timestamp_micros_since_unix_epoch__: micros });
const identity = (hex) => ({ __identity__: `0x${hex}` });

/** 1 to 40 characters once trimmed, without control characters, as the module checks names, decks, and cards. */
function checkText(what, text) {
  const trimmed = String(text).trim();
  const count = Array.from(trimmed).length;
  if (count === 0 || count > 40) throw new Error(`the ${what} must be 1 to 40 characters, got ${count}`);
  if (/\p{Cc}/u.test(trimmed)) throw new Error(`the ${what} must not contain control characters`);
  return trimmed;
}

/** A row by position, as transaction updates send it, and by field name, as the first look at a table does. */
function row(fields) {
  // Identities and timestamps are products of one field, which a row by position writes as a one-element array.
  const inner = (value) => value.__identity__ ?? value.__timestamp_micros_since_unix_epoch__;
  const position = Object.values(fields).map((value) => (typeof value === "object" ? [inner(value)] : value));
  return { position: js(position), named: js(fields) };
}

/** The module's tables and reducers, in memory. */
class Rooms {
  rooms = new Map();
  people = new Map();
  ballots = new Map();
  votes = new Map();
  results = new Map();
  clock = 1_790_000_000_000_000;

  /** Every public row of each table. */
  rows() {
    return {
      room: [...this.rooms].map(([code, room]) =>
        row({ code, deck_name: room.deck, round: room.round, revealed: room.revealed, created_at: stamp(room.created) }),
      ),
      participant: [...this.people].map(([id, person]) =>
        row({ identity: identity(id), room: person.room, name: person.name, joined_at: stamp(person.joined) }),
      ),
      ballot: [...this.ballots].map(([id, ballot]) =>
        row({ identity: identity(id), room: ballot.room, round: ballot.round }),
      ),
      result: [...this.results].map(([id, shown]) =>
        row({ identity: identity(id), room: shown.room, round: shown.round, card: shown.card }),
      ),
    };
  }

  call(id, reducer, args) {
    this.clock += 1000;
    const mine = () => {
      const me = this.people.get(id);
      if (!me) throw new Error("join a room first");
      return [me, this.rooms.get(me.room)];
    };
    if (reducer === "create_room") {
      const [deck, name] = [checkText("deck name", args[0]), checkText("name", args[1])];
      let code;
      do code = Array.from(randomBytes(6), (byte) => ALPHABET[byte % ALPHABET.length]).join("");
      while (this.rooms.has(code));
      this.leave(id);
      this.rooms.set(code, { deck, round: 1, revealed: false, created: this.clock });
      this.people.set(id, { room: code, name, joined: this.clock });
    } else if (reducer === "join_room") {
      const code = String(args[0]).trim().toUpperCase();
      const name = checkText("name", args[1]);
      if (!this.rooms.has(code)) throw new Error(`no room has the code ${code}; check it with whoever created the room`);
      this.leave(id);
      this.people.set(id, { room: code, name, joined: this.clock });
    } else if (reducer === "leave_room") {
      mine();
      this.leave(id);
    } else if (reducer === "cast_vote") {
      const card = checkText("card", args[0]);
      const [me, room] = mine();
      if (room.revealed) throw new Error("the cards are already revealed; start the next round to vote again");
      this.votes.set(id, { room: me.room, round: room.round, card });
      this.ballots.set(id, { room: me.room, round: room.round });
    } else if (reducer === "reveal") {
      const [me, room] = mine();
      for (const [voter, vote] of this.votes) {
        if (!room.revealed && vote.room === me.room && vote.round === room.round) this.results.set(voter, vote);
      }
      room.revealed = true;
    } else if (reducer === "next_round") {
      const [me, room] = mine();
      this.clear(me.room);
      Object.assign(room, { round: room.round + 1, revealed: false });
    } else {
      throw new Error(`no such reducer ${reducer}`);
    }
  }

  /** Takes `id` out of its room, deleting the room once nobody is left, as leaving and disconnecting do. */
  leave(id) {
    const me = this.people.get(id);
    if (!me) return;
    for (const table of [this.people, this.ballots, this.votes, this.results]) table.delete(id);
    if (![...this.people.values()].some((person) => person.room === me.room)) {
      this.clear(me.room);
      this.rooms.delete(me.room);
    }
  }

  clear(code) {
    for (const table of [this.ballots, this.votes, this.results]) {
      for (const [id, entry] of table) if (entry.room === code) table.delete(id);
    }
  }
}

/** The tables of a database update from `before` to `after`, rows by position. */
function changes(before, after) {
  return TABLES.flatMap((table, index) => {
    const old = new Set(before[table].map((entry) => entry.position));
    const now = new Set(after[table].map((entry) => entry.position));
    const deletes = [...old].filter((entry) => !now.has(entry));
    const inserts = [...now].filter((entry) => !old.has(entry));
    if (!deletes.length && !inserts.length) return [];
    const updates = [{ deletes, inserts }];
    return [{ table_id: 4096 + index, table_name: table, num_rows: deletes.length + inserts.length, updates }];
  });
}

/** The stand-in server; `url` is its address once `start()` returned. */
export class RoomStandIn {
  url = null;
  #rooms = new Rooms();
  #clients = new Map();
  #server = createServer((_, response) => response.writeHead(404).end());

  async start() {
    this.#server.on("upgrade", (request, socket) => this.#accept(request, socket));
    await new Promise((done) => this.#server.listen(0, "127.0.0.1", done));
    this.url = `http://127.0.0.1:${this.#server.address().port}`;
    return this.url;
  }

  async stop() {
    for (const client of [...this.#clients.values()]) client.peer.end();
    await new Promise((done) => this.#server.close(done));
  }

  #accept(request, socket) {
    if (new URL(request.url, "http://stand-in").pathname !== `/v1/database/${DATABASE}/subscribe`) {
      return socket.end("HTTP/1.1 404 Not Found\r\ncontent-length: 9\r\n\r\nnot found");
    }
    const id = randomBytes(32).toString("hex");
    const peer = upgrade(request, socket, PROTOCOL, {
      onText: (text) => this.#receive(id, JSON.parse(text)),
      onClose: () => {
        this.#clients.delete(id);
        this.#apply(id, () => this.#rooms.leave(id));
      },
    });
    if (!peer) return;
    this.#clients.set(id, { peer, subscribed: false });
    peer.send(js({ IdentityToken: { identity: identity(id), token: "stand-in", connection_id: { __connection_id__: 1 } } }));
  }

  #receive(id, message) {
    const client = this.#clients.get(id);
    if (message.Subscribe) {
      client.subscribed = true;
      const rows = this.#rooms.rows();
      const tables = TABLES.filter((table) => rows[table].length).map((table) => ({
        table_id: 4096 + TABLES.indexOf(table),
        table_name: table,
        num_rows: rows[table].length,
        updates: [{ deletes: [], inserts: rows[table].map((entry) => entry.named) }],
      }));
      const update = { database_update: { tables }, request_id: message.Subscribe.request_id };
      client.peer.send(js({ InitialSubscription: { ...update, total_host_execution_duration: { __time_duration_micros__: 1 } } }));
    } else if (message.CallReducer) {
      const { reducer, args, request_id } = message.CallReducer;
      this.#apply(id, () => this.#rooms.call(id, reducer, JSON.parse(args)), { reducer, args, request_id });
    }
  }

  /** Runs `change` for client `id`, then tells the caller of `call` its outcome and other subscribers the changes. */
  #apply(id, change, call) {
    const before = this.#rooms.rows();
    let error = null;
    try {
      change();
    } catch (refusal) {
      error = refusal;
    }
    const tables = error ? [] : changes(before, this.#rooms.rows());
    for (const [other, client] of this.#clients) {
      if (call && other === id) {
        const status = error ? { Failed: error.message } : { Committed: { tables } };
        const reducer_call = { reducer_name: call.reducer, reducer_id: 0, args: call.args, request_id: call.request_id };
        const answer = { status, timestamp: stamp(this.#rooms.clock), caller_identity: identity(id), reducer_call };
        client.peer.send(js({ TransactionUpdate: answer }));
      } else if (client.subscribed && tables.length) {
        client.peer.send(js({ TransactionUpdateLight: { request_id: 0, update: { tables } } }));
      }
    }
  }
}
