// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/**
 * The hello proof. Before the plugin sends its token, it asks `/api/hello?nonce=N` and expects
 * `control` to be HMAC-SHA256 of `N:port` under that token. When OBS quits or moves Sceneuron to
 * another port, some other program may listen on the old one; it cannot answer without the
 * token, and the port in the message stops it from passing on the real server's answer.
 */

import { createHmac, randomBytes, timingSafeEqual } from "node:crypto";

/** HMAC-SHA256 of `message` under `key`, as lowercase hexadecimal. */
export function hmacHex(key: string, message: string): string {
  return createHmac("sha256", key).update(message, "utf8").digest("hex");
}

/** A fresh nonce: 32 hexadecimal characters, within the server's 16-128. */
export function newNonce(): string {
  return randomBytes(16).toString("hex");
}

/** The `control` answer a server holding `token` gives for `nonce` on `port`. */
export function helloProof(token: string, nonce: string, port: number): string {
  return hmacHex(token, `${nonce}:${port}`);
}

/** True when `answer` is exactly `expected`, compared in constant time. */
export function proofMatches(expected: string, answer: unknown): boolean {
  if (typeof answer !== "string") {
    return false;
  }
  const want = Buffer.from(expected, "utf8");
  const got = Buffer.from(answer, "utf8");
  return want.length === got.length && timingSafeEqual(want, got);
}
