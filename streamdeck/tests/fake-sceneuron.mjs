// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// A fake Sceneuron control server on 127.0.0.1 that follows the shared HTTP contract: hello
// proofs, pairing with its Origin and attempt rules, the header-only control token, the control
// view and its deduplicated event stream, and the card, element, automation, push to talk, room,
// and reaction routes. Every request is recorded, with the body of each change, so tests can check what
// the plugin sent, and in which order. A hung fake takes requests and never answers them, and a
// slow route answers only after a wait.

import { createHmac } from "node:crypto";
import http from "node:http";

import {
  applyAutomation,
  applyCard,
  applyElement,
  applyFlood,
  applyRoom,
  applyTalk,
  sampleView,
} from "./fake-engine.mjs";

export const CONTROL_TOKEN = "control-token-0123456789abcdef";
const OTHER_TOKENS = { editor: "editor-token-0123456789abcdef", overlay: "overlay-token-0123456789abcd" };
const ROUTES = {
  "POST /api/card": applyCard,
  "POST /api/control/element": applyElement,
  "POST /api/automation": applyAutomation,
  "POST /api/voice/talk": applyTalk,
  "POST /api/room": applyRoom,
  "POST /api/flood": applyFlood,
};

const hmac = (key, message) => createHmac("sha256", key).update(message).digest("hex");

function reply(res, status, body) {
  res.writeHead(status, { "Content-Type": "application/json", Connection: "close" });
  res.end(JSON.stringify(body));
}

async function readBody(req) {
  let text = "";
  for await (const chunk of req) {
    text += chunk;
  }
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

/**
 * Starts the fake. `token` is the control token it holds, `code` an outstanding pairing code,
 * `room` true puts it in a shared room, `hung` true makes it record requests without ever
 * answering them, and `slow` maps a path to how many milliseconds its changes wait before they
 * apply and answer.
 */
export async function startFakeSceneuron(options = {}) {
  const { token = CONTROL_TOKEN, code = null, view = sampleView(), port = 0, hung = false, room = false } = options;
  const state = {
    token,
    code,
    attempts: 0,
    view,
    prove: true,
    hung,
    slow: options.slow ?? {},
    talking: false,
    room: room ? { revealed: false, round: 1 } : null,
    requests: [],
    streams: new Set(),
    sent: new Map(),
  };
  const server = http.createServer((req, res) => {
    handle(state, server.address().port, req, res).catch((error) => reply(res, 500, { error: String(error) }));
  });
  await new Promise((resolve) => server.listen(port, "127.0.0.1", resolve));

  const push = () => {
    const text = JSON.stringify(state.view);
    for (const res of state.streams) {
      if (state.sent.get(res) !== text) {
        state.sent.set(res, text);
        res.write(`event: control\ndata: ${text}\n\n`);
      }
    }
  };
  state.push = push;

  return {
    port: server.address().port,
    requests: state.requests,
    /** The bodies posted to `path`, in order. */
    posted(path) {
      const posts = state.requests.filter((request) => request.method === "POST" && request.path === path);
      return posts.map((request) => request.body);
    },
    get view() {
      return state.view;
    },
    /** Whether push to talk is held. */
    get talking() {
      return state.talking;
    },
    /** The room joined, as `{revealed, round}`, or null. */
    get room() {
      return state.room;
    },
    /** Changes the view as the server would, bumping the revision and telling every stream. */
    change(mutate) {
      mutate(state.view);
      state.view.revision += 1;
      push();
    },
    issueCode(next) {
      state.code = next;
      state.attempts = 0;
    },
    rotateToken(next) {
      state.token = next;
    },
    /** Stops answering hello with a valid proof, like another program on the port. */
    stopProving() {
      state.prove = false;
    },
    /** Answers hello with a valid proof again. */
    resumeProving() {
      state.prove = true;
    },
    /** Leaves every request from now on unanswered, like a server that has hung; `close` ends them. */
    hang() {
      state.hung = true;
    },
    get streamCount() {
      return state.streams.size;
    },
    dropStreams() {
      for (const res of state.streams) {
        res.destroy();
      }
      state.streams.clear();
    },
    /** Ends every stream cleanly, as Sceneuron does when it revokes their token. */
    endStreams() {
      for (const res of state.streams) {
        res.end();
      }
      state.streams.clear();
    },
    async close() {
      this.dropStreams();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}

async function handle(state, port, req, res) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  const request = { method: req.method, path: url.pathname, query: url.search, headers: { ...req.headers } };
  state.requests.push(request);
  if (state.hung) {
    return undefined;
  }
  if (req.headers.host !== `127.0.0.1:${port}`) {
    return reply(res, 400, { error: "unexpected Host" });
  }
  if (url.pathname === "/api/hello") {
    const nonce = url.searchParams.get("nonce") ?? "";
    if (!/^[0-9a-fA-F]{16,128}$/.test(nonce)) {
      return reply(res, 400, { error: "nonce must be 16-128 hexadecimal characters" });
    }
    const message = `${nonce}:${port}`;
    const control = state.prove ? hmac(state.token, message) : hmac("someone else", message);
    const editor = hmac(OTHER_TOKENS.editor, message);
    return reply(res, 200, { editor, overlay: hmac(OTHER_TOKENS.overlay, message), control });
  }
  if (url.pathname === "/api/control/pair" && req.method === "POST") {
    return pair(state, req, res);
  }
  // The control token counts only in the header, never in the query.
  if (req.headers["x-sceneuron-token"] !== state.token) {
    return reply(res, 401, { error: "a valid token is required" });
  }
  const route = `${req.method} ${url.pathname}`;
  if (route === "GET /api/control/state") {
    return reply(res, 200, state.view);
  }
  if (route === "GET /api/control/events") {
    res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-store" });
    state.streams.add(res);
    res.on("close", () => state.streams.delete(res));
    state.push();
    return undefined;
  }
  const apply = ROUTES[route];
  if (!apply) {
    return reply(res, 404, { error: "not found" });
  }
  request.body = await readBody(req);
  if (state.slow[url.pathname]) {
    await new Promise((resolve) => setTimeout(resolve, state.slow[url.pathname]));
  }
  const before = JSON.stringify(state.view);
  const result = apply(state.view, request.body, state);
  if (result.status !== 200) {
    return reply(res, result.status, { error: result.error });
  }
  if (JSON.stringify(state.view) !== before) {
    state.view.revision += 1;
    state.push();
  }
  return reply(res, 200, result.body ?? state.view);
}

async function pair(state, req, res) {
  const site = req.headers["sec-fetch-site"];
  if (req.headers.origin !== undefined || (site !== undefined && site !== "none")) {
    return reply(res, 403, { error: "pairing is not available to web pages" });
  }
  const body = await readBody(req);
  if (state.code === null || body?.code !== state.code) {
    state.attempts += 1;
    if (state.attempts >= 5) {
      state.code = null;
    }
    return reply(res, 403, { error: "wrong or expired code" });
  }
  state.code = null;
  return reply(res, 200, { token: state.token });
}
