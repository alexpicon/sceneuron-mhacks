// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Live test of the Stream Deck plugin against a real Sceneuron, without Stream Deck or OBS: starts
// sceneuron-dev on a free port with a known editor token, adds the poker pack through the project
// API, and runs the built plugin the way Stream Deck runs it, under the plugin tests' fake Stream
// Deck app. The plugin pairs with a code the editor token asked for, proves the server, follows
// the control event stream, raises, lowers, and steps cards, shows and hides an element, pauses
// automation, holds push to talk, reveals the votes and starts the next round in a room on a
// stand-in SpacetimeDB (room-stand-in.mjs), plays the email reaction from a Reaction key that
// lights up while it plays, and ends refused with "Pair again" once the editor token revokes it. A
// fresh plugin then pairs from the editor's deep link alone, as Stream Deck delivers it, and links
// that are replayed, wrong, or malformed pair nothing (streamdeck-link-steps.mjs). Prints PASS or
// FAIL per step and exits nonzero on a failure.
//
// Build both first; this script builds nothing:
//   cargo build --locked -p sceneuron-dev
//   (cd streamdeck && npm ci && npm run build)
// Usage: node e2e/streamdeck-live.mjs [--dev-binary PATH]

import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual, parseArgs } from "node:util";

import { DevServer, devBinary } from "./dev-server.mjs";
import { RoomStandIn } from "./room-stand-in.mjs";
import { runStreamDeckLinkSteps } from "./streamdeck-link-steps.mjs";
import { runStreamDeckSteps } from "./streamdeck-steps.mjs";

const { values: options } = parseArgs({ options: { "dev-binary": { type: "string" } } });
const repo = resolve(import.meta.dirname, "..");
const binary = devBinary(options["dev-binary"]);
const fakeStreamDeck = join(repo, "streamdeck", "tests", "fake-stream-deck.mjs");
const hook = pathToFileURL(join(import.meta.dirname, "streamdeck-fetch-hook.mjs")).href;
const token = "streamdeck-live-editor-token-0123456789";
const js = JSON.stringify;

let failures = 0;
async function step(name, body) {
  try {
    const detail = await body();
    console.log(`PASS ${name}${detail ? ` - ${detail}` : ""}`);
  } catch (error) {
    failures += 1;
    console.log(`FAIL ${name} - ${error.message}`);
  }
}

function same(actual, expected, what) {
  if (!isDeepStrictEqual(actual, expected)) throw new Error(`${what}: expected ${js(expected)}, got ${js(actual)}`);
}

/** Calls the API with the editor token; an error reply throws. */
async function api(method, path, body) {
  const response = await fetch(`${server.base}${path}`, {
    method,
    headers: { "X-Sceneuron-Token": token, "Content-Type": "application/json" },
    body: body === undefined ? undefined : js(body),
  });
  const json = await response.json();
  if (!response.ok) throw new Error(`${method} ${path}: ${response.status} ${json.error}`);
  return json;
}

let server;
let streamDeck;
const roomStandIn = new RoomStandIn();

async function finish(code) {
  clearTimeout(deadline);
  await streamDeck?.stop().catch((error) => console.log(`cannot stop the plugin: ${error.message}`));
  await server?.stop();
  await roomStandIn.stop().catch((error) => console.log(`cannot stop the room stand-in: ${error.message}`));
  process.exit(code);
}

const deadline = setTimeout(() => {
  console.log("FAIL the run took longer than two minutes");
  finish(1);
}, 120_000);
process.on("SIGINT", () => finish(130));

try {
  // The fake Stream Deck needs the plugin's own `ws` package, so it is loaded only once that is there.
  if (!existsSync(join(repo, "streamdeck", "node_modules"))) {
    throw new Error("streamdeck/node_modules is missing; run npm ci and npm run build in streamdeck/ first");
  }
  const fake = await import(pathToFileURL(fakeStreamDeck).href);
  if (!fake.bundleExists()) throw new Error("the plugin is not built; run npm run build in streamdeck/ first");
  server = new DevServer(binary, ["--port", "0", "--token", token]);
  await server.ready();
  const port = Number(new URL(server.base).port);
  // Each start stands for a new Stream Deck app with nothing saved, so the one before it stops.
  const start = async () => {
    await streamDeck?.stop();
    streamDeck = await fake.startStreamDeck({}, { nodeArgs: ["--import", hook] });
    return streamDeck;
  };
  const roomServer = await roomStandIn.start();
  const harness = { step, same, api, port, fake, start, roomServer };
  await runStreamDeckSteps(harness);
  await runStreamDeckLinkSteps(harness);
} catch (error) {
  failures += 1;
  console.log(`FAIL the run stopped - ${error.message}`);
}
console.log(failures ? `${failures} step(s) failed` : "All steps passed");
await finish(failures ? 1 : 0);
