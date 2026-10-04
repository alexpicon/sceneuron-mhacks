// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron pages: the access token and the port from the page address, and a check that the server
   on this port is the Sceneuron that issued it before the token is ever sent.

   Addresses carry the token in the fragment (#token=...), which browsers never send to a server.
   Older addresses with ?token= still work and are rewritten to the fragment form. Before sending
   the token, a page asks /api/hello with a random nonce and expects HMAC-SHA256 of "nonce:port"
   under its token, so another program listening on the port after OBS quits learns nothing. */
(function () {
  "use strict";

  /** The token from `#token=` or, for older links, `?token=`; empty when there is none. */
  function tokenFrom(location) {
    const fragment = new URLSearchParams(String(location.hash || "").replace(/^#/, ""));
    return fragment.get("token") || new URLSearchParams(location.search || "").get("token") || "";
  }

  /** The address without a query token, with the token in the fragment instead; null if unchanged. */
  function fragmentAddress(location) {
    const query = new URLSearchParams(location.search || "");
    const token = query.get("token");
    if (!token) {
      return null;
    }
    query.delete("token");
    const rest = query.toString();
    return `${location.pathname}${rest ? `?${rest}` : ""}#token=${encodeURIComponent(token)}`;
  }

  /** The port the page came from: the address's own, or its scheme's default when the address leaves it out. */
  function pagePort(location) {
    return location.port || (location.protocol === "https:" ? "443" : "80");
  }

  function hex(bytes) {
    return Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  /** HMAC-SHA256 of `message` under `key`, as lowercase hexadecimal. */
  async function hmacHex(key, message, subtle) {
    const encoder = new TextEncoder();
    const imported = await subtle.importKey("raw", encoder.encode(key), { name: "HMAC", hash: "SHA-256" }, false, [
      "sign",
    ]);
    return hex(await subtle.sign("HMAC", imported, encoder.encode(message)));
  }

  function randomNonce(crypto) {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    return hex(bytes);
  }

  /**
   * True when the server on `port` proves it holds `token`. Never sends the token; any failure,
   * including no answer, is false.
   */
  async function verifyServer(token, port, fetchImpl, crypto) {
    try {
      const nonce = randomNonce(crypto);
      const response = await fetchImpl(`/api/hello?nonce=${nonce}`, { cache: "no-store" });
      if (!response.ok) {
        return false;
      }
      const proofs = await response.json();
      const expected = await hmacHex(token, `${nonce}:${port}`, crypto.subtle);
      return proofs.editor === expected || proofs.overlay === expected;
    } catch {
      return false;
    }
  }

  const link = { tokenFrom, fragmentAddress, pagePort, hmacHex, verifyServer };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = link;
  }
  if (typeof window !== "undefined") {
    window.SceneuronLink = link;
  }
})();
