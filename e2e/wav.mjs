// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// WAV files from raw 16-bit PCM, for the live voice test: ElevenLabs text to speech answers
// `pcm_16000` as bare little-endian samples, and sceneuron-dev --voice-wav reads WAV.

/** Bare mono 16-bit little-endian PCM with `seconds` of silence before and after it. */
export function padded(pcm, sampleRate, seconds) {
  const silence = Buffer.alloc(Math.round(sampleRate * seconds) * 2);
  return Buffer.concat([silence, pcm.subarray(0, pcm.length - (pcm.length % 2)), silence]);
}

/** A mono 16-bit PCM WAV file holding `pcm`. */
export function pcm16Wav(pcm, sampleRate) {
  const header = Buffer.alloc(44);
  header.write("RIFF", 0, "latin1");
  header.writeUInt32LE(36 + pcm.length, 4);
  header.write("WAVEfmt ", 8, "latin1");
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(1, 22);
  header.writeUInt32LE(sampleRate, 24);
  header.writeUInt32LE(sampleRate * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write("data", 36, "latin1");
  header.writeUInt32LE(pcm.length, 40);
  return Buffer.concat([header, pcm]);
}

/** Seconds of mono 16-bit audio in `pcm`. */
export function pcmSeconds(pcm, sampleRate) {
  return pcm.length / 2 / sampleRate;
}
