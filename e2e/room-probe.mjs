// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A client of the room's database outside Sceneuron, on a raw WebSocket with SpacetimeDB's JSON text protocol
// (v1.json.spacetimedb), for tests that check the room from the side of anyone who can reach the database: what a
// subscription shows, what a query may read, and what the module's reducers refuse.

const PROTOCOL = "v1.json.spacetimedb";

/** Opens a probe on `database` at `server` (http or https) and waits for its identity. */
export async function openProbe(server, database = "sceneuron-room") {
  const url = `${server.replace(/^http/, "ws")}/v1/database/${database}/subscribe`;
  const socket = new WebSocket(url, PROTOCOL);
  const texts = [];
  const inbox = [];
  const waiting = [];
  socket.addEventListener("message", (event) => {
    texts.push(event.data);
    inbox.push(JSON.parse(event.data));
    for (const wake of waiting.splice(0)) wake();
  });
  await new Promise((open, fail) => {
    socket.addEventListener("open", open);
    socket.addEventListener("error", () => fail(new Error(`cannot open ${url}`)));
  });
  let requests = 0;

  /** The next message of `kind` that `wanted` accepts, taken from the inbox; waits at most `timeoutMs`. */
  async function next(kind, wanted = () => true, timeoutMs = 10_000) {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      const index = inbox.findIndex((message) => kind in message && wanted(message[kind]));
      if (index >= 0) return inbox.splice(index, 1)[0][kind];
      const left = deadline - Date.now();
      if (left <= 0) throw new Error(`no ${kind} message from ${url} in ${timeoutMs} ms`);
      await new Promise((wake) => {
        waiting.push(wake);
        setTimeout(wake, left);
      });
    }
  }

  const send = (message) => socket.send(JSON.stringify(message));
  const identity = (await next("IdentityToken")).identity.__identity__;
  return {
    /** This connection's identity, as `0x` and 64 hex digits. */
    identity,
    /** Every message received so far, as the text that arrived, in order. */
    texts,
    next,
    /** Subscribes to `queries` and returns the first look at their rows. */
    async subscribe(queries) {
      requests += 1;
      const request_id = requests;
      send({ Subscribe: { query_strings: queries, request_id } });
      return next("InitialSubscription", (answer) => answer.request_id === request_id);
    },
    /** Calls `reducer` with `args` and returns the status of the caller's update. */
    async call(reducer, args) {
      requests += 1;
      const request_id = requests;
      send({ CallReducer: { reducer, args: JSON.stringify(args), request_id, flags: 0 } });
      return (await next("TransactionUpdate", (update) => update.reducer_call.request_id === request_id)).status;
    },
    /** Runs one SQL query outside any subscription and returns its answer. */
    async query(sql) {
      requests += 1;
      const message_id = [requests];
      send({ OneOffQuery: { message_id, query_string: sql } });
      return next("OneOffQueryResponse");
    },
    close: () => socket.close(),
  };
}
