// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the live subtitles test's guards: without both keys it refuses to run, says that it spends credits, and
// reaches no server; with keys, a build that cannot run stops it before anything is spent; and with a working build,
// the free checks run on this computer, with AI settings that name Gemini, and the first request to ElevenLabs, at a
// closed address, stops it before anything is spent, as does Google refusing the kind of key or its API restrictions,
// in words that say what to do. Every run points both APIs at a closed address or a stand-in on this computer, so a
// broken guard fails here instead of reaching ElevenLabs or Gemini. The build is looked for where cargo puts it, in
// CARGO_TARGET_DIR when that is set; with SCENEURON_REQUIRE_DEV_BINARY=1, a build the test cannot find fails it
// instead of skipping it.

import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { join, resolve } from "node:path";
import { test } from "node:test";

const SCRIPT = join(import.meta.dirname, "..", "subtitles-live.mjs");
const REPO = resolve(import.meta.dirname, "..", "..");
const TARGET = process.env.CARGO_TARGET_DIR ? resolve(REPO, process.env.CARGO_TARGET_DIR) : join(REPO, "target");
const DEV = join(TARGET, "debug", "sceneuron-dev");
const REQUIRE_DEV = process.env.SCENEURON_REQUIRE_DEV_BINARY === "1";
// Port 9 (discard) is closed on a computer that runs no such service, so a request is refused at once.
const CLOSED = "http://127.0.0.1:9";
const EL = "el-test-key-not-real-0123";
const GEMINI = "gemini-test-key-not-real-0123";
const MODEL = "gemini-3.5-flash-lite";

/** The environment for a run with `keys`, both APIs at `url`. */
function environment(keys, url) {
  const env = { ...process.env, SCENEURON_ELEVENLABS_URL: url, SCENEURON_GEMINI_URL: url };
  delete env.SCENEURON_ELEVENLABS_API_KEY;
  delete env.SCENEURON_GEMINI_API_KEY;
  return Object.assign(env, keys);
}

function run(keys, args = []) {
  const env = environment(keys, CLOSED);
  return spawnSync(process.execPath, [SCRIPT, ...args], { env, encoding: "utf8", timeout: 60_000 });
}

const withoutBoth = [
  ["without either key", {}],
  ["with only the ElevenLabs key", { SCENEURON_ELEVENLABS_API_KEY: EL }],
  ["with only the Gemini key", { SCENEURON_GEMINI_API_KEY: GEMINI }],
  ["with a blank Gemini key", { SCENEURON_ELEVENLABS_API_KEY: EL, SCENEURON_GEMINI_API_KEY: "  " }],
  ["with an empty ElevenLabs key", { SCENEURON_ELEVENLABS_API_KEY: "", SCENEURON_GEMINI_API_KEY: GEMINI }],
];
for (const [name, keys] of withoutBoth) {
  test(`it refuses to run ${name}`, () => {
    const result = run(keys);
    assert.equal(result.status, 2);
    assert.match(result.stderr, /spends credits/);
    assert.match(result.stderr, /Set SCENEURON_ELEVENLABS_API_KEY and SCENEURON_GEMINI_API_KEY to run it/);
    assert.equal(result.stdout, "");
  });
}

const both = { SCENEURON_ELEVENLABS_API_KEY: EL, SCENEURON_GEMINI_API_KEY: GEMINI };

function assertNothingSpent(result) {
  assert.equal(result.status, 2, result.stdout + result.stderr);
  assert.match(result.stdout, /Nothing was spent: a free check failed/);
  assert.doesNotMatch(result.stdout, /About to spend/);
  assert.doesNotMatch(result.stdout + result.stderr, new RegExp(`${EL}|${GEMINI}`));
}

test("a build that cannot run stops the test before anything is spent", () => {
  const work = join(REPO, "target", "subtitles-live-test-missing");
  const missing = join(import.meta.dirname, "no-such-sceneuron-dev");
  const result = run(both, ["--dev-binary", missing, "--work-dir", work]);
  rmSync(work, { recursive: true, force: true });
  assertNothingSpent(result);
  assert.match(result.stdout, /FAIL this build has subtitles - cannot run /);
});

const withoutBuild = existsSync(DEV) || REQUIRE_DEV ? false : "build sceneuron-dev first";

test("with a working build, the first request to ElevenLabs stops the test before anything is spent", {
  skip: withoutBuild,
}, () => {
  const work = join(REPO, "target", "subtitles-live-test-closed");
  rmSync(work, { recursive: true, force: true });
  const result = run(both, ["--dev-binary", DEV, "--work-dir", work]);
  const file = join(work, "ai.json");
  const settings = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : null;
  rmSync(work, { recursive: true, force: true });
  assertNothingSpent(result);
  assert.equal(settings?.provider, "gemini", "the settings name Gemini, not the default provider");
  assert.equal(settings?.base_url, CLOSED, "the settings send translations where the test points Gemini");
  assert.equal(settings?.subtitles?.translation_model, MODEL);
  assert.match(result.stdout, /PASS this build has subtitles/);
  assert.match(result.stdout, /PASS sceneuron-dev takes subtitles into Spanish, and waits for a key without one/);
  assert.match(result.stdout, / - subtitles into Spanish by Gemini, waiting for a key\n/);
  assert.match(result.stdout, /FAIL ElevenLabs accepts the key and lists a voice to use, at no cost - fetch failed/);
  assert.doesNotMatch(result.stdout, /Gemini accepts/, "the checks stop at the first failure");
});

/**
 * A stand-in for both APIs on this computer that lists one voice and refuses the Gemini key with HTTP 401 and
 * `reason`, in the shape Google answers with; it keeps each request it gets.
 */
async function refusingStandIn(reason) {
  const requests = [];
  const server = createServer((request, response) => {
    requests.push({ method: request.method, url: request.url, headers: request.headers });
    const voices = request.url.startsWith("/v2/voices");
    const error = {
      code: 401,
      message: "Request had invalid authentication credentials. Expected OAuth 2 access token.",
      status: "UNAUTHENTICATED",
      details: [{ "@type": "type.googleapis.com/google.rpc.ErrorInfo", reason, domain: "googleapis.com" }],
    };
    const body = voices ? { voices: [{ voice_id: "voice-1", name: "Stand-in", category: "premade" }] } : { error };
    response.writeHead(voices ? 200 : 401, { "Content-Type": "application/json" }).end(JSON.stringify(body));
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const close = () => new Promise((done) => server.close(done));
  return { requests, url: `http://127.0.0.1:${server.address().port}`, close };
}

/** Runs the script while this process answers as `url`, so it cannot block as spawnSync would. */
function runAgainst(url, args) {
  const child = spawn(process.execPath, [SCRIPT, ...args], { env: environment(both, url) });
  const output = { stdout: "", stderr: "" };
  child.stdout.on("data", (chunk) => (output.stdout += chunk));
  child.stderr.on("data", (chunk) => (output.stderr += chunk));
  return new Promise((done) => child.on("close", (status) => done({ status, ...output })));
}

for (const reason of ["ACCESS_TOKEN_TYPE_UNSUPPORTED", "API_KEY_SERVICE_BLOCKED"]) {
  test(`Google refusing the key with ${reason} stops the test before anything is spent, saying what to do`, {
    skip: withoutBuild,
    timeout: 60_000,
  }, async () => {
    const standIn = await refusingStandIn(reason);
    const work = join(REPO, "target", `subtitles-live-test-${reason.toLowerCase()}`);
    rmSync(work, { recursive: true, force: true });
    const result = await runAgainst(standIn.url, ["--dev-binary", DEV, "--work-dir", work]).finally(standIn.close);
    rmSync(work, { recursive: true, force: true });
    assertNothingSpent(result);
    assert.match(result.stdout, /PASS ElevenLabs accepts the key and lists a voice to use, at no cost/);
    const refused = `FAIL Gemini accepts the key and knows ${MODEL}, at no cost - Google refused the key with ` +
      `HTTP 401 (${reason}). Create a key in Google AI Studio (https://aistudio.google.com/apikey), or allow the ` +
      "Generative Language API in the key's API restrictions.\n";
    assert.ok(result.stdout.includes(refused), result.stdout);
    assert.doesNotMatch(result.stdout, /OAuth/, "Google's own words are left out");
    const asked = standIn.requests.map(({ method, url }) => `${method} ${url}`);
    assert.deepEqual(asked, ["GET /v2/voices?page_size=100", `GET /v1beta/models/${MODEL}`], "only the free requests");
    const [voices, model] = standIn.requests;
    assert.equal(voices.headers["xi-api-key"], EL);
    assert.equal(model.headers["x-goog-api-key"], GEMINI, "the Gemini key goes in its header");
    assert.equal(model.headers["xi-api-key"], undefined, "and only the Gemini key goes to Gemini");
  });
}
