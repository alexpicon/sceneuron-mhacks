// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the words of the Stream Deck section, the deep link that asks the Stream Deck app on this
   computer to pair with a fresh code, whether the editor runs where it can open that link, and how the editor
   tells that a Stream Deck paired. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);

  /** Stream Deck routes `streamdeck://plugins/message/<plugin UUID>/...` to the plugin with that UUID. */
  const PAIR_LINK = "streamdeck://plugins/message/ai.chaski.sceneuron/pair";

  const WORDS = {
    sent:
      "Sent this code to the Stream Deck app on this computer; your browser may ask to open Elgato Stream Deck " +
      "first. Nothing happened? ",
    again: "Send it again",
    fallback: ", or enter it in a key's settings:",
    paired: "A Stream Deck paired with this code. Every Sceneuron key on it works now.",
    pairedToast: "A Stream Deck paired. Its Sceneuron keys work now.",
    inObs:
      "An OBS dock cannot open the Stream Deck app, so here choose Connect a Stream Deck and enter the code in a " +
      "key's settings. Open the editor in a browser to pair the Stream Deck on this computer in one click.",
  };

  /**
   * The deep link that asks Sceneuron's plugin in the Stream Deck app to pair with `code` on `port`, or null when
   * either is not valid. `streamdeck=hidden` makes it a passive deep link (Stream Deck 7.0 and newer), which leaves
   * the Stream Deck window where it is, so the editor stays in front to say when the plugin paired.
   */
  function pairingLink(port, code) {
    const portText = String(port);
    const number = Number(portText);
    if (!/^\d{1,5}$/.test(portText) || number < 1 || number > 65535 || !/^\d{6}$/.test(String(code))) {
      return null;
    }
    const query = new URLSearchParams({ port: String(number), code: String(code), streamdeck: "hidden" });
    return `${PAIR_LINK}?${query}`;
  }

  /**
   * True when the editor runs in OBS's own browser, as a Custom Browser Dock, whose user agent names OBS
   * (" OBS/31.1.2"). That browser does not hand a streamdeck:// link to the operating system: following one fails,
   * and OBS shows its error page in place of the editor.
   */
  function inObs(userAgent) {
    return / OBS\/\d/.test(String(userAgent));
  }

  /**
   * True once the server counts more codes traded for the control token than it had when the code on screen was
   * issued (`before`), which is when a Stream Deck paired with it.
   */
  function pairedSince(before, status) {
    const now = status ? status.pairings : undefined;
    return Number.isInteger(before) && Number.isInteger(now) && now > before;
  }

  /** What the editor says once the code expired: `sent` when it went to the Stream Deck app in a deep link. */
  function expiredNote(sent) {
    if (!sent) {
      return "This code expired. Choose Connect a Stream Deck for a new one.";
    }
    return (
      "This code expired before a Stream Deck paired. Check that the Stream Deck app, 7.1 or newer, is running " +
      "with the Sceneuron plugin, then choose Pair the Stream Deck on this computer again, or Connect a Stream " +
      "Deck to type a code."
    );
  }

  const streamDeckText = { WORDS, expiredNote, inObs, pairedSince, pairingLink };
  if (inNode) {
    module.exports = streamDeckText;
  } else {
    window.SceneuronEditor.streamDeckText = streamDeckText;
  }
})();
