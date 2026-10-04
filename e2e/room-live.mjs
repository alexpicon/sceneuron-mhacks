// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Live test of the shared planning poker room, run by hand: starts a throwaway SpacetimeDB on
// a free local port with its own data directory (spacetime-server.mjs), publishes room/module to it,
// starts two sceneuron-dev servers on free ports, and runs room-steps.mjs: a room created on one and
// joined from the other, a hidden vote from each through /api/card, a client outside Sceneuron that
// checks the module keeps votes secret until the reveal, a reveal with a control token, a vote the
// module refuses after it, connections that end leaving the room, the next round, and leaving. Prints
// PASS or FAIL per step and exits nonzero on a failure.
//
// Needs the spacetime CLI and Rust's wasm32-unknown-unknown target; docs/development.md says how to
// install them. Build the dev server first; the module is built when it is published:
//   cargo build --locked -p sceneuron-dev
// Usage: node e2e/room-live.mjs [--dev-binary PATH] [--spacetime PATH] [--work-dir DIR] [--keep]

import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { isDeepStrictEqual, parseArgs } from "node:util";

import { DevServer, devBinary } from "./dev-server.mjs";
import { runRoomSteps } from "./room-steps.mjs";
import { Spacetime } from "./spacetime-server.mjs";

const { values: options } = parseArgs({
  options: {
    "dev-binary": { type: "string" },
    spacetime: { type: "string" },
    "work-dir": { type: "string" },
    keep: { type: "boolean" },
  },
});
const binary = devBinary(options["dev-binary"]);
// The installer puts the CLI in ~/.local/bin, which a shell may not have on its PATH yet.
const spacetimeBinary = options.spacetime ?? (process.env.SPACETIME || join(homedir(), ".local", "bin", "spacetime"));
// Only a directory this run created is deleted afterwards; a given one may hold other files.
const ownWork = !options["work-dir"];
const work = ownWork ? mkdtempSync(join(tmpdir(), "sceneuron-room-")) : resolve(options["work-dir"]);
mkdirSync(work, { recursive: true });
const DATABASE = "sceneuron-room";
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

/** One participant: a dev server with its editor and control tokens and API helpers for each. */
function person(name) {
  const token = `room-live-${name.toLowerCase()}-editor-0123456789`;
  const controlToken = `room-live-${name.toLowerCase()}-control-0123456789`;
  const server = new DevServer(binary, ["--port", "0", "--token", token, "--control-token", controlToken]);
  const raw = async (method, path, body, key = token) => {
    const response = await fetch(`${server.base}${path}`, {
      method,
      headers: { "X-Sceneuron-Token": key, "Content-Type": "application/json" },
      body: body === undefined ? undefined : js(body),
    });
    return { status: response.status, body: await response.json() };
  };
  const checked = (key) => async (method, path, body) => {
    const reply = await raw(method, path, body, key);
    if (reply.status !== 200) throw new Error(`${name}: ${method} ${path}: ${reply.status} ${reply.body.error}`);
    return reply.body;
  };
  return { name, server, raw, api: checked(token), control: checked(controlToken) };
}

const spacetime = new Spacetime(spacetimeBinary, join(work, "spacetime"));
const people = [];

async function finish(code) {
  clearTimeout(deadline);
  for (const { server } of people) await server.stop();
  await spacetime.stop().catch((error) => console.log(`cannot stop SpacetimeDB: ${error.message}`));
  if (code !== 0 || options.keep) console.log(`Work directory: ${work}`);
  else if (ownWork) rmSync(work, { recursive: true, force: true });
  process.exit(code);
}

// Building the module for wasm is most of the time on a first run.
const deadline = setTimeout(() => {
  console.log("FAIL the run took longer than ten minutes");
  finish(1);
}, 600_000);
process.on("SIGINT", () => finish(130));

try {
  let url;
  await step("a throwaway SpacetimeDB starts on a free local port", async () => {
    url = await spacetime.start();
    return url;
  });
  if (!url) throw new Error("no SpacetimeDB to test against");
  let published = false;
  await step("room/module is built and published to it", () => {
    const last = spacetime.publish(DATABASE);
    published = true;
    return last;
  });
  if (!published) throw new Error("no room module to test against");
  for (const name of ["Ana", "Ben"]) {
    const started = person(name);
    people.push(started);
    await started.server.ready();
  }
  await runRoomSteps({ step, same, people, room: { server: url, database: DATABASE } });
} catch (error) {
  failures += 1;
  console.log(`FAIL the run stopped - ${error.message}`);
}
console.log(failures ? `${failures} step(s) failed` : "All steps passed");
await finish(failures ? 1 : 0);
