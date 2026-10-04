// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Choosing the voice the live voice test speaks with, from the account's voice list
// (GET /v2/voices, https://elevenlabs.io/docs/api-reference/voices/search), which costs nothing.

/** "George", the default voice of the ElevenLabs quickstart, which older accounts have. */
export const DEFAULT_VOICE_ID = "JBFqnCBsd6RMkjVDRZzb";

/**
 * The voice to use from a `{ voices, has_more }` page: `wanted` when given and listed, or possibly on a later page;
 * else George when listed; else the first premade voice; else the first one. Null when the page lists none.
 */
export function chooseVoice(page, wanted) {
  const voices = Array.isArray(page?.voices) ? page.voices.filter((voice) => voice && voice.voice_id) : [];
  if (wanted) {
    const found = voices.find((voice) => voice.voice_id === wanted);
    if (found || page?.has_more) {
      return found ?? { voice_id: wanted, name: "" };
    }
  }
  return (
    voices.find((voice) => voice.voice_id === DEFAULT_VOICE_ID) ??
    voices.find((voice) => voice.category === "premade") ??
    voices[0] ??
    null
  );
}
