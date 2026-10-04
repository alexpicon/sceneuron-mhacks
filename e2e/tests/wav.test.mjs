// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the WAV helpers of the live voice test: silence padding around bare PCM, the 44-byte
// mono 16-bit header, and durations.

import assert from "node:assert/strict";
import { test } from "node:test";

import { padded, pcm16Wav, pcmSeconds } from "../wav.mjs";

test("padding puts silence on both sides and drops a stray byte", () => {
  const pcm = Buffer.from([1, 2, 3, 4, 5]);
  const out = padded(pcm, 4, 0.5);
  assert.deepEqual([...out], [0, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 0]);
  assert.equal(pcmSeconds(out, 4), 1.5);
});

test("the header describes mono 16-bit PCM at the given rate", () => {
  const pcm = Buffer.from([0x10, 0x00, 0xf0, 0xff]);
  const wav = pcm16Wav(pcm, 16_000);
  assert.equal(wav.length, 48);
  assert.equal(wav.toString("latin1", 0, 4), "RIFF");
  assert.equal(wav.readUInt32LE(4), 40);
  assert.equal(wav.toString("latin1", 8, 16), "WAVEfmt ");
  assert.deepEqual(
    [wav.readUInt32LE(16), wav.readUInt16LE(20), wav.readUInt16LE(22), wav.readUInt32LE(24)],
    [16, 1, 1, 16_000],
  );
  assert.deepEqual([wav.readUInt32LE(28), wav.readUInt16LE(32), wav.readUInt16LE(34)], [32_000, 2, 16]);
  assert.equal(wav.toString("latin1", 36, 40), "data");
  assert.equal(wav.readUInt32LE(40), 4);
  assert.deepEqual([wav.readInt16LE(44), wav.readInt16LE(46)], [16, -16]);
});

test("durations count two bytes per sample", () => {
  assert.equal(pcmSeconds(Buffer.alloc(32_000), 16_000), 1);
  assert.equal(pcmSeconds(Buffer.alloc(0), 16_000), 0);
});
