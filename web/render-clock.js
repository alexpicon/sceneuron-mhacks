// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay clock: the time on air, "{{on-air-time}}", which the server leaves in a text for the page to
   fill in, so it counts every second between snapshots. It counts from the snapshot's on_air.since_ms, on the
   server's clock at the snapshot's now_ms, plus the time this page has seen pass since that snapshot arrived; off
   air it shows 00:00:00. Shared by render.js and the Node tests. */
(function () {
  "use strict";

  const TOKEN = "{{on-air-time}}";
  const SECOND_MS = 1000;

  const pad = (value) => String(value).padStart(2, "0");

  /** `ms` as hours, minutes, and seconds, such as 00:42:18; past 99 hours the hours take more digits. */
  function formatTime(ms) {
    const seconds = Math.floor(Math.max(0, Number.isFinite(ms) ? ms : 0) / SECOND_MS);
    return `${pad(Math.floor(seconds / 3600))}:${pad(Math.floor(seconds / 60) % 60)}:${pad(seconds % 60)}`;
  }

  // For each root: the snapshot time it last drew, the local time it first drew it, and the pending tick.
  const roots = new WeakMap();

  /**
   * How long OBS has been on air for `root`'s drawing of `snapshot`, or null while it is off air. `options.now`
   * replaces Date.now, for tests.
   */
  function onAirMs(root, snapshot, options) {
    const now = options && typeof options.now === "function" ? options.now() : Date.now();
    const at = snapshot && Number.isFinite(snapshot.now_ms) ? snapshot.now_ms : 0;
    let state = roots.get(root);
    if (!state) {
      state = { at: null, seen: 0, timer: 0, view: null };
      roots.set(root, state);
    }
    if (state.at !== at) {
      Object.assign(state, { at, seen: now });
    }
    const since = snapshot && snapshot.on_air ? snapshot.on_air.since_ms : null;
    return Number.isFinite(since) ? Math.max(0, at - since) + Math.max(0, now - state.seen) : null;
  }

  /** `text` with the time on air, `ms`, in place of each "{{on-air-time}}". */
  function fillTime(text, ms) {
    return String(text).split(TOKEN).join(formatTime(ms === null ? 0 : ms));
  }

  /**
   * Draws `root` again with `redraw` when the time on air shown there next changes, while `counting` says a shown
   * element counts it; otherwise stops. Without a window to time it, as in Node, it never draws again.
   */
  function keepCounting(root, ms, counting, redraw) {
    const state = roots.get(root);
    const view = root.ownerDocument && root.ownerDocument.defaultView;
    if (!state) {
      return;
    }
    if (state.timer) {
      state.view.clearTimeout(state.timer);
      state.timer = 0;
    }
    if (counting && ms !== null && view && typeof view.setTimeout === "function") {
      state.view = view;
      state.timer = view.setTimeout(() => {
        state.timer = 0;
        redraw();
      }, SECOND_MS - (ms % SECOND_MS));
    }
  }

  const api = { TOKEN, fillTime, formatTime, keepCounting, onAirMs };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronClock = api;
  }
})();
