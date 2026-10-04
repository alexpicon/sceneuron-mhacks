// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Requests to Sceneuron's local HTTP server. Every call goes to 127.0.0.1 (the server refuses any
 * other Host), sends the control token only in the X-Sceneuron-Token header, never in an address,
 * and refuses redirects so the token cannot follow one somewhere else.
 */

import { helloProof, newNonce, proofMatches } from "./proof.ts";
import { isToken } from "./settings.ts";

export type Fetch = typeof fetch;

/** Where the server is and the token it gave this plugin. */
export interface Endpoint {
  readonly port: number;
  readonly token: string;
}

/**
 * How long a request may wait for an answer unless its caller allows longer; a server on this
 * computer answers in milliseconds.
 */
const REQUEST_TIMEOUT_MS = 5_000;

const TOKEN_HEADER = "X-Sceneuron-Token";

/** A failed request: `status` is the HTTP status, or 0 when nothing answered. */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

/** True when the server refused the token: it is unknown there, revoked, or not allowed this route. */
export function isRefusal(error: unknown): boolean {
  return error instanceof ApiError && (error.status === 401 || error.status === 403);
}

function serverUrl(port: number, path: string): string {
  return `http://127.0.0.1:${port}${path}`;
}

/**
 * Sends one request. Only the wait for the reply's head is limited, to `timeoutMs`, so an event
 * stream can stay open; an abort from `signal` propagates as is, any other failure becomes
 * ApiError(0).
 */
async function request(
  fetchImpl: Fetch,
  port: number,
  path: string,
  init: RequestInit,
  signal?: AbortSignal,
  timeoutMs = REQUEST_TIMEOUT_MS,
): Promise<Response> {
  const timeout = new AbortController();
  const timer = setTimeout(() => timeout.abort(), timeoutMs);
  try {
    const combined = signal ? AbortSignal.any([signal, timeout.signal]) : timeout.signal;
    return await fetchImpl(serverUrl(port, path), { ...init, redirect: "error", signal: combined });
  } catch (error) {
    if (signal?.aborted) {
      throw error;
    }
    throw new ApiError(0, `nothing answered on port ${port}`);
  } finally {
    clearTimeout(timer);
  }
}

async function failure(response: Response): Promise<ApiError> {
  let message = `HTTP ${response.status}`;
  try {
    const body: unknown = await response.json();
    const error = typeof body === "object" && body !== null ? (body as { error?: unknown }).error : undefined;
    if (typeof error === "string" && error) {
      message = error;
    }
  } catch {
    // The status alone says enough.
  }
  return new ApiError(response.status, message);
}

async function readJson(response: Response): Promise<unknown> {
  if (!response.ok) {
    throw await failure(response);
  }
  try {
    return await response.json();
  } catch {
    throw new ApiError(response.status, "the reply was not JSON");
  }
}

function field(value: unknown, name: string): unknown {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>)[name] : undefined;
}

/**
 * True when the server on `port` proves it holds `token` for a fresh nonce; the token itself is
 * never sent. Throws ApiError(0) when nothing answers, so callers can tell offline from refused.
 */
export async function verifyServer(
  port: number,
  token: string,
  fetchImpl: Fetch,
  signal?: AbortSignal,
): Promise<boolean> {
  const nonce = newNonce();
  const response = await request(fetchImpl, port, `/api/hello?nonce=${nonce}`, { method: "GET" }, signal);
  if (!response.ok) {
    await response.body?.cancel();
    return false;
  }
  let answer: unknown;
  try {
    answer = await response.json();
  } catch {
    return false;
  }
  return proofMatches(helloProof(token, nonce, port), field(answer, "control"));
}

/** Trades a pairing code from the editor for a control token. */
export async function claimToken(port: number, code: string, fetchImpl: Fetch): Promise<string> {
  const init = { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code }) };
  const response = await request(fetchImpl, port, "/api/control/pair", init);
  const token = field(await readJson(response), "token");
  if (!isToken(token)) {
    throw new ApiError(response.status, "the reply held no token");
  }
  return token;
}

export async function getJson(
  endpoint: Endpoint,
  path: string,
  fetchImpl: Fetch,
  signal?: AbortSignal,
): Promise<unknown> {
  const init = { method: "GET", headers: { [TOKEN_HEADER]: endpoint.token } };
  return readJson(await request(fetchImpl, endpoint.port, path, init, signal));
}

/** Posts `body` to `path`; `timeoutMs` is for a route that may take longer than most to answer. */
export async function postJson(
  endpoint: Endpoint,
  path: string,
  body: object,
  fetchImpl: Fetch,
  timeoutMs = REQUEST_TIMEOUT_MS,
): Promise<unknown> {
  const headers = { "Content-Type": "application/json", [TOKEN_HEADER]: endpoint.token };
  const init = { method: "POST", headers, body: JSON.stringify(body) };
  return readJson(await request(fetchImpl, endpoint.port, path, init, undefined, timeoutMs));
}

/** Opens an event stream and returns its body once the server accepts it. */
export async function openStream(
  endpoint: Endpoint,
  path: string,
  fetchImpl: Fetch,
  signal: AbortSignal,
): Promise<ReadableStream<Uint8Array>> {
  const init = { method: "GET", headers: { Accept: "text/event-stream", [TOKEN_HEADER]: endpoint.token } };
  const response = await request(fetchImpl, endpoint.port, path, init, signal);
  if (!response.ok) {
    throw await failure(response);
  }
  if (!response.body) {
    throw new ApiError(response.status, "the event stream had no body");
  }
  return response.body;
}
