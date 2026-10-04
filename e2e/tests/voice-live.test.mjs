// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the live voice test's guards: without an ElevenLabs key it refuses to run, says that
// it spends credits, and reaches no server; and with a key, a free check that fails stops it
// before anything is spent or any request leaves this computer. Every run points the script at a
// closed address on this computer, so a broken guard fails here instead of reaching ElevenLabs.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { test } from "node:test";

const SCRIPT = join(import.meta.dirname, "..", "voice-live.mjs");
// Port 9 (discard) is closed on a computer that runs no such service, so a request is refused at once.
const CLOSED = "http://127.0.0.1:9";

for (const key of [undefined, "", "   "]) {
  test(`it refuses to run with the key ${JSON.stringify(key)}`, () => {
    const env = { ...process.env, SCENEURON_ELEVENLABS_URL: CLOSED };
    delete env.SCENEURON_ELEVENLABS_API_KEY;
    if (key !== undefined) env.SCENEURON_ELEVENLABS_API_KEY = key;
    const result = spawnSync(process.execPath, [SCRIPT], { env, encoding: "utf8", timeout: 10_000 });
    assert.equal(result.status, 2);
    assert.match(result.stderr, /spends credits/);
    assert.match(result.stderr, /Set SCENEURON_ELEVENLABS_API_KEY/);
    assert.equal(result.stdout, "");
  });
}

test("a build that cannot run stops the test before anything is spent", () => {
  const env = { ...process.env, SCENEURON_ELEVENLABS_API_KEY: "test-key-not-real", SCENEURON_ELEVENLABS_URL: CLOSED };
  const missing = join(import.meta.dirname, "no-such-sceneuron-dev");
  // A work directory of its own is never created, since the first check fails before anything is written.
  const work = join(import.meta.dirname, "no-such-work-dir");
  const result = spawnSync(process.execPath, [SCRIPT, "--dev-binary", missing, "--work-dir", work], {
    env,
    encoding: "utf8",
    timeout: 20_000,
  });
  assert.equal(result.status, 2, result.stdout + result.stderr);
  assert.match(result.stdout, /FAIL this build has voice control - cannot run /);
  assert.match(result.stdout, /Nothing was spent: a free check failed/);
  assert.doesNotMatch(result.stdout, /About to spend/);
  assert.doesNotMatch(result.stdout + result.stderr, /test-key-not-real/);
});
