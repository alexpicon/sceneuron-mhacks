// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// What the Stream Deck live test reads from the plugin it runs: the requests the plugin made, as
// streamdeck-fetch-hook.mjs writes them to its standard error, and the connection states its
// property inspector was told about.

/** Starts the line for each request sent, so the lines stand out from anything else the plugin writes. */
export const REQUEST_MARK = "sceneuron-live-request ";
/** Starts the line for each answer, or for a request nothing answered. */
export const ANSWER_MARK = "sceneuron-live-answer ";

const PAIR = "/api/control/pair";
const HELLO = "/api/hello";
const EVENTS = "/api/control/events";

/**
 * A request as a line, written when it is sent: its sequence number (1 for the first request,
 * counting up in the order they were sent), method, path, query, and whether it carried the token
 * header or an Origin. It never holds a header's value.
 */
export function requestLine({ seq, method, path, query, token, origin }) {
  return `${REQUEST_MARK}${JSON.stringify({ seq, method, path, query, token, origin })}\n`;
}

/**
 * The answer to request `seq` as a line: its status (0 when nothing answered), and `answeredAfter`,
 * the sequence number of the newest request sent by the time the answer came.
 */
export function answerLine({ seq, status, answeredAfter }) {
  return `${ANSWER_MARK}${JSON.stringify({ seq, status, answeredAfter })}\n`;
}

/**
 * The requests recorded in `output`, in the order they were sent, each with its answer's status
 * and `answeredAfter`; both are null for a request not answered yet.
 */
export function readRequests(output) {
  const requests = new Map();
  for (const line of output.split(/\r?\n/)) {
    if (line.startsWith(REQUEST_MARK)) {
      const request = JSON.parse(line.slice(REQUEST_MARK.length));
      requests.set(request.seq, { ...request, status: null, answeredAfter: null });
    } else if (line.startsWith(ANSWER_MARK)) {
      const { seq, status, answeredAfter } = JSON.parse(line.slice(ANSWER_MARK.length));
      const request = requests.get(seq);
      if (request) Object.assign(request, { status, answeredAfter });
    }
  }
  return [...requests.values()].sort((a, b) => a.seq - b.seq);
}

/** True for a hello that asked with a fresh 32-digit hexadecimal nonce and was answered. */
export function isProof(request) {
  return request.path === HELLO && /^\?nonce=[0-9a-f]{32}$/.test(request.query) && request.status === 200;
}

/**
 * The requests the link made for its own connection: those after the hello with which the newest
 * pairing checked its new token, or all of them when nothing paired. Pairing proves the token
 * once for itself, so that hello proves nothing for the link.
 */
export function linkRequests(requests) {
  const paired = requests.findLastIndex((request) => request.path === PAIR && request.status === 200);
  if (paired < 0) return requests;
  const check = requests.findIndex((request, index) => index > paired && request.path === HELLO);
  return check < 0 ? [] : requests.slice(check + 1);
}

/**
 * Why `requests` broke the rule that the token goes out only after a proof, or null when they
 * kept it. Every event stream the link opens needs a proof of its own, answered before the stream
 * was asked for, and any other request with the token needs a stream opened that way after the
 * link's newest hello, since the link says hello only to connect or after a stream ends. The first
 * hello after a pairing that worked is pairing's own check of the new token, so it proves nothing
 * for the link; this assumes the link sends nothing while it pairs, as the live test's plugin is
 * unpaired until then. A pairing request or a hello that carries the token breaks the rule too.
 */
export function tokenBeforeProof(requests) {
  let proof = null;
  let pairing = false;
  let streamed = false;
  for (const request of requests) {
    const name = `${request.method} ${request.path} (request ${request.seq})`;
    if (request.path === PAIR) {
      if (request.token) return `${name} carried the token`;
      // A new token: the link starts over once pairing has checked it.
      if (request.status === 200) {
        proof = null;
        pairing = true;
        streamed = false;
      }
    } else if (request.path === HELLO) {
      if (request.token) return `${name} carried the token`;
      if (pairing) {
        pairing = false;
      } else {
        streamed = false;
        if (isProof(request)) proof = request;
      }
    } else if (request.token && request.path === EVENTS) {
      if (!proof || proof.answeredAfter >= request.seq) return `${name} carried the token before a proof of its own`;
      proof = null;
      streamed = true;
    } else if (request.token && !streamed) {
      return `${name} carried the token before any proven stream`;
    }
  }
  return null;
}

/** The connection states the inspector was shown, in order, each change once. */
export function linkStates(messages) {
  const states = [];
  for (const message of messages) {
    const state = message.event === "sendToPropertyInspector" ? message.payload?.state : undefined;
    if (typeof state === "string" && states.at(-1) !== state) states.push(state);
  }
  return states;
}
