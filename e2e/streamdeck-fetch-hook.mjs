// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Loaded into the Stream Deck plugin with `node --import` by streamdeck-live.mjs, before the
// plugin's own code: wraps fetch so the plugin's standard error gets a numbered line for each
// request as it is sent, and another when its answer comes (see streamdeck-log.mjs). The test can
// then check which routes the plugin called, in the order it sent them, what had been answered by
// then, and where the token went. Only whether a request carried the token is written, never the
// token.

import { answerLine, requestLine } from "./streamdeck-log.mjs";

const send = globalThis.fetch;
let sent = 0;

globalThis.fetch = async (input, init = {}) => {
  const url = new URL(input instanceof Request ? input.url : String(input));
  const headers = new Headers(init.headers);
  sent += 1;
  const seq = sent;
  process.stderr.write(
    requestLine({
      seq,
      method: init.method ?? "GET",
      path: url.pathname,
      query: url.search,
      token: headers.has("X-Sceneuron-Token"),
      origin: headers.has("Origin"),
    }),
  );
  let status = 0;
  try {
    const response = await send(input, init);
    status = response.status;
    return response;
  } finally {
    process.stderr.write(answerLine({ seq, status, answeredAfter: sent }));
  }
};
