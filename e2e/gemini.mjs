// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Reading the Gemini API's answers to the live subtitles test's free check: a model's details
// (GET /v1beta/models/{model}, https://ai.google.dev/api/models#method:-models.get), which cost nothing, and a
// refusal of the key, in words that say what to do about it.

/**
 * The google.rpc.ErrorInfo reasons of a key Google refuses for the Gemini API, which it answers with HTTP 401 or
 * 403: ACCESS_TOKEN_TYPE_UNSUPPORTED, saying it expected an OAuth 2 access token, for a kind of key it does not take
 * there, and API_KEY_SERVICE_BLOCKED for a key whose API restrictions leave out the Generative Language API.
 */
const KEY_REASONS = ["ACCESS_TOKEN_TYPE_UNSUPPORTED", "API_KEY_SERVICE_BLOCKED"];

/**
 * What to do about an error reply with HTTP `status` and `body` that refuses the key, or null when it is not such a
 * refusal. Google's own message is left out, since for a kind of key it does not take it asks for an OAuth 2 access
 * token, which the Gemini API does not need.
 */
export function keyRefusal(status, body) {
  let details;
  try {
    details = JSON.parse(body)?.error?.details;
  } catch {
    return null;
  }
  const reasons = Array.isArray(details) ? details.map((detail) => detail?.reason) : [];
  const reason = KEY_REASONS.find((known) => reasons.includes(known));
  return reason
    ? `Google refused the key with HTTP ${status} (${reason}). Create a key in Google AI Studio ` +
        "(https://aistudio.google.com/apikey), or allow the Generative Language API in the key's API restrictions."
    : null;
}

/** Why `model`, the details GET /v1beta/models/`id` answered, cannot translate, or null when it can. */
export function modelProblem(model, id) {
  if (!model || typeof model !== "object") {
    return `the details of ${id} are not an object`;
  }
  const methods = model.supportedGenerationMethods;
  return Array.isArray(methods) && !methods.includes("generateContent") ? `${id} does not take generateContent` : null;
}
