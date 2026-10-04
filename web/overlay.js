// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay page: draws live snapshots on a transparent, full-window canvas, plays their emoji floods
   (see flood.js) and spoken cards (see speech.js). Before each connection it asks the server to prove it holds the
   page's token (see link.js). */
(function () {
  "use strict";

  const RETRY_MIN_MS = 1000;
  const RETRY_MAX_MS = 10000;

  function streamUrl(token) {
    return `/api/events?token=${encodeURIComponent(token)}`;
  }

  /** Backoff for reconnecting after the server refused or closed the event stream. */
  function nextDelay(previous) {
    return previous > 0 ? Math.min(RETRY_MAX_MS, previous * 2) : RETRY_MIN_MS;
  }

  function parseSnapshot(data) {
    try {
      const snapshot = JSON.parse(data);
      return snapshot && typeof snapshot === "object" && Array.isArray(snapshot.elements) ? snapshot : null;
    } catch {
      return null;
    }
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = { nextDelay, parseSnapshot, streamUrl };
  }
  if (typeof document === "undefined") {
    return;
  }

  // Page styles are set through the CSSOM because the CSP blocks inline style elements.
  function stylePage() {
    for (const node of [document.documentElement, document.body]) {
      node.style.setProperty("margin", "0");
      node.style.setProperty("padding", "0");
      node.style.setProperty("width", "100%");
      node.style.setProperty("height", "100%");
      node.style.setProperty("overflow", "hidden");
      node.style.setProperty("background", "transparent");
    }
  }

  function start() {
    stylePage();
    const root = document.getElementById("overlay-root");
    const link = window.SceneuronLink;
    const token = link.tokenFrom(window.location);
    const moved = link.fragmentAddress(window.location);
    if (moved) {
      window.history.replaceState(null, "", moved);
    }
    if (!token) {
      console.warn("Sceneuron overlay: the URL has no token; copy the overlay URL from the editor.");
    }
    let snapshot = null;
    let delay = 0;
    let retryTimer = 0;
    const speak = window.SceneuronSpeech.createPlayer(token, () => new Audio());

    function draw() {
      if (snapshot) {
        window.SceneuronRender.applyFit(root, snapshot.canvas, window.innerWidth, window.innerHeight);
        window.SceneuronRender.render(root, snapshot);
        window.SceneuronFlood.update(root, snapshot);
      }
    }

    function onSnapshot(event) {
      const next = parseSnapshot(event.data);
      if (next) {
        snapshot = next;
        delay = 0;
        draw();
        speak(next);
      }
    }

    function retry() {
      if (!retryTimer) {
        delay = nextDelay(delay);
        retryTimer = window.setTimeout(connect, delay);
      }
    }

    async function connect() {
      retryTimer = 0;
      const port = Number(link.pagePort(window.location));
      const fetchImpl = (input, init) => window.fetch(input, init);
      if (!(await link.verifyServer(token, port, fetchImpl, window.crypto))) {
        retry();
        return;
      }
      const source = new EventSource(streamUrl(token));
      source.addEventListener("state", onSnapshot);
      source.addEventListener("message", onSnapshot);
      source.addEventListener("error", () => {
        // EventSource would reconnect by itself without asking who answers, so the server is
        // proved again before the token is sent again.
        source.close();
        retry();
      });
    }

    window.addEventListener("resize", draw);
    connect();
  }

  start();
})();
