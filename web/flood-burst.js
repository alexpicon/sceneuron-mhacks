// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Seeded positions for each reaction's emoji rain and bursts. Shared by every overlay theme. */
(function () {
  "use strict";

  /** A seeded generator of numbers in [0, 1), so one flood always looks the same. */
  function generator(seed) {
    let state = Math.floor(Math.abs(Number(seed) || 0)) >>> 0;
    return () => {
      state = (state + 0x6d2b79f5) >>> 0;
      let t = state;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const api = { generator };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  } else {
    window.SceneuronFloodBurst = api;
  }
})();
