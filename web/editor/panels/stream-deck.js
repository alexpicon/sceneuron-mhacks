// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: Stream Decks in the Cards tab: a one-time pairing code with its countdown and how to enter it
   in Stream Deck, or the same code sent to the Stream Deck app on this computer in a deep link (except in an OBS
   dock, which cannot open it), word of when a Stream Deck paired, and disconnecting every paired Stream Deck after
   asking. A Stream Deck gets only the control token, which reaches cards, element visibility and overrides,
   effects, and automation, and reads only the control view (names, card faces, and what shows), never the project
   routes, the camera, or the AI. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { countdown, groupCode } = Editor.cardsText;
  const { WORDS, expiredNote, inObs, pairedSince, pairingLink } = Editor.streamDeckText;
  const { h } = Editor.dom;
  const { pagePort } = window.SceneuronLink;

  function markup(port, sendsLinks) {
    const button = (id, label, attrs) => h("button", Object.assign({ id, type: "button" }, attrs), label);
    return [
      h("h3", { id: "stream-deck-heading" }, "Stream Deck"),
      h(
        "p",
        { class: "hint" },
        "Once paired, an Elgato Stream Deck can raise and lower cards, show or hide elements, release their ",
        "overrides, play effects, pause or resume automation, and hold push to talk. It sees deck, card, and ",
        "element names, each card's text and colors, which cards are up, whether the camera or voice raised them ",
        "and when they lower by themselves, which elements show or hold a manual change, and whether automation ",
        "is paused. It cannot change the project, see the camera, or read the AI settings, what the AI sees or ",
        "hears, or the recent events.",
      ),
      h(
        "div",
        { class: "button-row" },
        sendsLinks && button("pair-here", "Pair the Stream Deck on this computer", { class: "primary" }),
        button("pair-stream-deck", "Connect a Stream Deck"),
        button("revoke-stream-decks", "Disconnect Stream Decks", { class: "danger" }),
      ),
      h("p", { id: "pairing-in-obs", class: "hint", hidden: sendsLinks }, WORDS.inObs),
      h(
        "div",
        { id: "pairing", class: "pairing", hidden: true },
        h(
          "p",
          { id: "pairing-code", tabindex: "-1", "aria-describedby": "pairing-expiry" },
          h("span", { class: "field-label" }, "Pairing code"),
          h("strong", { id: "pairing-digits", class: "pairing-digits" }),
        ),
        h("p", { id: "pairing-expiry", class: "hint" }),
        h(
          "p",
          { id: "pairing-sent", tabindex: "-1", hidden: true },
          WORDS.sent,
          h("a", { id: "pairing-link" }, WORDS.again),
          WORDS.fallback,
        ),
        h(
          "ol",
          { id: "pairing-steps", class: "pairing-steps" },
          h("li", {}, "Install the Sceneuron plugin for Stream Deck, from the streamdeck folder of Sceneuron."),
          h("li", {}, "In the Stream Deck app, drag a Sceneuron action onto a key and select it to see its settings."),
          h("li", {}, "Enter the port ", h("strong", { id: "pairing-port" }, port), " and this code, and choose Pair."),
        ),
      ),
      h(
        "div",
        { id: "revoke-confirm", class: "confirm-box", hidden: true },
        h(
          "p",
          { id: "revoke-text" },
          "Disconnect every paired Stream Deck? Their keys show Pair again until you pair them with a new code.",
        ),
        h(
          "div",
          { class: "button-row" },
          button("revoke-confirmed", "Disconnect them", { class: "danger", "aria-describedby": "revoke-text" }),
          button("revoke-cancel", "Keep them"),
        ),
      ),
    ];
  }

  /** Builds and wires the Stream Deck section; returns an empty view, since it needs no redraws. */
  Editor.streamDeck = function setupStreamDeck(editor) {
    const { api, byId } = editor;
    // The editor runs on the port a Stream Deck must use, so its own address names it.
    const port = pagePort(window.location);
    // Following a streamdeck:// link in an OBS dock would put OBS's error page in place of the editor, so there the
    // code is only typed.
    const sendsLinks = !inObs(window.navigator.userAgent);
    byId("stream-deck-section").replaceChildren(...markup(port, sendsLinks));
    const revoke = byId("revoke-stream-decks");
    const pairing = byId("pairing");
    const code = byId("pairing-digits");
    const expiry = byId("pairing-expiry");
    const sent = byId("pairing-sent");
    const link = byId("pairing-link");
    const steps = byId("pairing-steps");
    const confirm = byId("revoke-confirm");
    // The newest code issued here: when it expires, how many codes the server had seen traded when it was issued,
    // and whether it went to the Stream Deck app. `shown` is that code while it can still pair.
    let latest = null;
    let shown = null;
    let timer = null;
    let asking = false;
    let checking = false;

    function stop() {
      window.clearInterval(timer);
      timer = null;
    }

    /** Ends the countdown, showing `digits` and `note` in place of the code, which no link offers to send again. */
    function settle(digits, note) {
      stop();
      shown = null;
      sent.hidden = true;
      link.removeAttribute("href");
      code.textContent = digits;
      expiry.textContent = note;
    }

    /** Says "Paired" once the server counts a code traded since `current` was issued. */
    async function watch(current) {
      if (checking) {
        return;
      }
      checking = true;
      try {
        if (pairedSince(current.pairings, await api.pairings()) && latest === current) {
          steps.hidden = true;
          settle("Paired", WORDS.paired);
          editor.toast(WORDS.pairedToast, "info");
        }
      } catch {
        // The next second asks again, and the countdown says when the code stops working.
      } finally {
        checking = false;
      }
    }

    function tick() {
      const current = shown;
      const left = current.expires - Date.now();
      if (left > 0) {
        expiry.textContent = `It works once, for ${countdown(left)}.`;
        watch(current);
        return;
      }
      settle("Expired", expiredNote(current.sent));
      editor.announce("The Stream Deck pairing code expired.");
      // A Stream Deck may have paired within the last second.
      watch(current);
    }

    /** Asks for a new code and shows it; with `toApp`, also sends it to the Stream Deck app in a deep link. */
    async function issue(toApp) {
      if (asking) {
        return;
      }
      asking = true;
      try {
        const reply = await api.pairing();
        const deepLink = toApp ? pairingLink(port, reply.code) : null;
        if (toApp && !deepLink) {
          throw new Error(`The Stream Deck cannot pair with port ${port}. Connect a Stream Deck instead.`);
        }
        latest = { expires: Date.now() + Number(reply.expires_in_s) * 1000, pairings: reply.pairings, sent: toApp };
        shown = latest;
        code.textContent = groupCode(reply.code);
        confirm.hidden = true;
        pairing.hidden = false;
        steps.hidden = false;
        sent.hidden = !toApp;
        stop();
        // The first tick may already find the code expired, which stops this timer and keeps the code from being sent.
        timer = window.setInterval(tick, 1000);
        tick();
        if (deepLink && shown === latest) {
          // The link stays for another try, in case the browser's question about opening Stream Deck was declined.
          link.setAttribute("href", deepLink);
          link.click();
          sent.focus();
        } else {
          // Focus reads the code and its expiry out, and shows the steps right under it.
          byId("pairing-code").focus();
        }
      } catch (error) {
        editor.report(error);
      } finally {
        asking = false;
      }
    }

    if (sendsLinks) {
      byId("pair-here").addEventListener("click", () => issue(true));
    }
    byId("pair-stream-deck").addEventListener("click", () => issue(false));

    const cancel = () => {
      confirm.hidden = true;
      revoke.focus();
    };
    revoke.addEventListener("click", () => {
      confirm.hidden = false;
      byId("revoke-confirmed").focus();
    });
    byId("revoke-cancel").addEventListener("click", cancel);
    confirm.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
      }
    });
    byId("revoke-confirmed").addEventListener("click", async () => {
      try {
        await api.revokeControl();
      } catch (error) {
        editor.report(error);
        return;
      }
      // Revoking also drops the pairing code, so a code still shown would no longer work.
      latest = null;
      settle("", "");
      pairing.hidden = true;
      cancel();
      editor.toast("Disconnected every Stream Deck. Connect one again to use it.", "info");
    });

    return {};
  };
})();
