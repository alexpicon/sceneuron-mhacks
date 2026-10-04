// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * Pairing: the editor shows a six-digit code, the user types it into the property inspector or the
 * editor sends it in a deep link, and the plugin trades it for a control token. The new token must
 * pass the hello proof on the same port before the plugin keeps it.
 */

import { ApiError, type Fetch, claimToken, verifyServer } from "./api.ts";
import { isPairingCode, isPort } from "./settings.ts";

export type PairingResult =
  | { readonly ok: true; readonly port: number; readonly token: string; readonly message: string }
  | { readonly ok: false; readonly message: string };

/** What went wrong, in words for the property inspector. */
export function pairingMessage(error: unknown, port: number): string {
  if (!(error instanceof ApiError)) {
    return "Pairing failed unexpectedly. Try again with a new code.";
  }
  switch (error.status) {
    case 0:
      return `Nothing answered on port ${port}. Start OBS with Sceneuron (or sceneuron-dev) and check the port.`;
    case 403:
      return "Sceneuron refused that code: it is wrong, used, or expired. Ask the editor for a new one.";
    case 401:
    case 404:
    case 405:
      return `The Sceneuron on port ${port} cannot pair with a Stream Deck. Update Sceneuron.`;
    default:
      return `Sceneuron could not pair (${error.message}). Try again with a new code.`;
  }
}

/** Reads the port and code the user typed; a message instead when either is not valid. */
export function readPairingInput(
  port: unknown,
  code: unknown,
): { readonly port: number; readonly code: string } | { readonly message: string } {
  const portNumber = typeof port === "string" && /^\d{1,5}$/.test(port.trim()) ? Number(port.trim()) : port;
  if (!isPort(portNumber)) {
    return { message: "The port is a number from 1 to 65535; Sceneuron uses 4466 unless you changed it." };
  }
  const digits = typeof code === "string" ? code.replace(/\s+/g, "") : code;
  if (!isPairingCode(digits)) {
    return { message: "The code is the six digits the editor shows under Connect a Stream Deck." };
  }
  return { port: portNumber, code: digits };
}

/**
 * The port and code in the editor's pairing deep link, which Stream Deck hands over without its
 * `streamdeck://plugins/message/<plugin UUID>` prefix, as `/pair?port=4466&code=123456`; null when
 * the link is anything else. Each value must appear once and exactly as the editor writes it,
 * since nobody typed it; other parameters, such as `streamdeck=hidden`, are Stream Deck's own.
 */
export function readPairingLink(
  path: string,
  query: URLSearchParams,
): { readonly port: number; readonly code: string } | null {
  const [port, ...otherPorts] = query.getAll("port");
  const [code, ...otherCodes] = query.getAll("code");
  if ((path !== "/pair" && path !== "pair") || otherPorts.length > 0 || otherCodes.length > 0) {
    return null;
  }
  const portNumber = port !== undefined && /^\d{1,5}$/.test(port) ? Number(port) : null;
  return isPort(portNumber) && isPairingCode(code) ? { port: portNumber, code } : null;
}

/** Pairs with the Sceneuron on the typed port, returning the token only once the server proves it. */
export async function pairWithCode(port: unknown, code: unknown, fetchImpl: Fetch = fetch): Promise<PairingResult> {
  const input = readPairingInput(port, code);
  if ("message" in input) {
    return { ok: false, message: input.message };
  }
  try {
    const token = await claimToken(input.port, input.code, fetchImpl);
    if (!(await verifyServer(input.port, token, fetchImpl))) {
      return { ok: false, message: `The server on port ${input.port} did not prove it is Sceneuron.` };
    }
    return { ok: true, port: input.port, token, message: `Paired with Sceneuron on port ${input.port}.` };
  } catch (error) {
    return { ok: false, message: pairingMessage(error, input.port) };
  }
}
