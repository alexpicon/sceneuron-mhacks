// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how the live subtitles test reads the Gemini API's free answers: a refusal of the key in words that say
// what to do, only for the reasons that mean the kind of key or its API restrictions, and whether a model's details
// say it can translate.

import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { keyRefusal, modelProblem } from "../gemini.mjs";

/** An error reply in the google.rpc.Status shape Google answers with. */
function refusal(code, status, reason, message = "Request had invalid authentication credentials.") {
  const details = [
    { "@type": "type.googleapis.com/google.rpc.ErrorInfo", reason, domain: "googleapis.com" },
    { "@type": "type.googleapis.com/google.rpc.LocalizedMessage", locale: "en-US", message },
  ];
  return JSON.stringify({ error: { code, message, status, details } });
}

const advice = (status, reason) =>
  `Google refused the key with HTTP ${status} (${reason}). Create a key in Google AI Studio ` +
  "(https://aistudio.google.com/apikey), or allow the Generative Language API in the key's API restrictions.";

describe("a refused key", () => {
  test("a kind of key Google does not take says what to do instead of asking for an OAuth token", () => {
    const body = refusal(401, "UNAUTHENTICATED", "ACCESS_TOKEN_TYPE_UNSUPPORTED", "Expected OAuth 2 access token");
    assert.equal(keyRefusal(401, body), advice(401, "ACCESS_TOKEN_TYPE_UNSUPPORTED"));
    assert.doesNotMatch(keyRefusal(401, body), /OAuth/);
  });

  test("API restrictions that block the Generative Language API say the same, with the status Google gave", () => {
    const blocked = (code, status) => refusal(code, status, "API_KEY_SERVICE_BLOCKED");
    assert.equal(keyRefusal(401, blocked(401, "UNAUTHENTICATED")), advice(401, "API_KEY_SERVICE_BLOCKED"));
    assert.equal(keyRefusal(403, blocked(403, "PERMISSION_DENIED")), advice(403, "API_KEY_SERVICE_BLOCKED"));
  });

  test("other errors keep Google's own words", () => {
    assert.equal(keyRefusal(400, refusal(400, "INVALID_ARGUMENT", "API_KEY_INVALID")), null, "a wrong key");
    const noKey = JSON.stringify({ error: { code: 403, message: "Method doesn't allow unregistered callers" } });
    assert.equal(keyRefusal(403, noKey), null, "no details");
    const oddDetails = JSON.stringify({ error: { code: 401, details: { reason: "API_KEY_SERVICE_BLOCKED" } } });
    assert.equal(keyRefusal(401, oddDetails), null, "details that are not a list");
    assert.equal(keyRefusal(401, JSON.stringify({ error: { details: [null, 7] } })), null);
    assert.equal(keyRefusal(502, "<html>Bad Gateway</html>"), null, "not JSON");
    assert.equal(keyRefusal(401, ""), null);
    assert.equal(keyRefusal(401, "null"), null);
  });
});

describe("a model's details", () => {
  const MODEL = "gemini-3.5-flash-lite";

  test("a model that takes generateContent can translate", () => {
    const details = {
      name: `models/${MODEL}`,
      displayName: "Gemini 3.5 Flash-Lite",
      supportedGenerationMethods: ["generateContent", "countTokens"],
    };
    assert.equal(modelProblem(details, MODEL), null);
    assert.equal(modelProblem({ name: `models/${MODEL}` }, MODEL), null, "details that do not list methods");
  });

  test("a model that does not take generateContent, or details that are not an object, cannot", () => {
    const embeddings = { name: "models/embedding", supportedGenerationMethods: ["embedContent"] };
    assert.equal(modelProblem(embeddings, MODEL), `${MODEL} does not take generateContent`);
    assert.equal(modelProblem(null, MODEL), `the details of ${MODEL} are not an object`);
    assert.equal(modelProblem("models", MODEL), `the details of ${MODEL} are not an object`);
  });
});
