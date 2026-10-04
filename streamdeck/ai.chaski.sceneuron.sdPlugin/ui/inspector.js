// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron property inspector: talks to Stream Deck over its WebSocket, shows the connection and
   the pairing form, and saves the chosen deck, card, element, or reaction in the action's settings.

   The plugin does all network work. This page sends it the port and code, never asks for the
   global settings that hold the token, and ignores them if Stream Deck sends them. Text from the
   plugin goes in with textContent only. */
(function () {
  "use strict";

  const model = window.SceneuronInspectorModel;
  /** The settings fields the page has a list for. */
  const FIELDS = ["deck", "card", "element", "reaction"];
  const page = {};
  let socket = null;
  let context = "";
  let actionUuid = "";
  let kind = null;
  let settings = {};
  let status = model.WAITING;
  let pairing = false;
  let shownState = "";
  let portTyped = false;

  function send(message) {
    if (socket && socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(message));
    }
  }

  function toPlugin(payload) {
    send({ event: "sendToPlugin", action: actionUuid, context, payload });
  }

  function fill(select, options) {
    select.replaceChildren(
      ...options.map((option) => {
        const element = document.createElement("option");
        element.value = option.value;
        element.textContent = option.label;
        element.selected = option.selected;
        return element;
      }),
    );
  }

  function render() {
    const fields = model.fieldsFor(kind);
    for (const field of FIELDS) {
      page[`${field}Item`].hidden = !fields.includes(field);
    }
    page.automationNote.hidden = kind !== "automation";
    page.dialNote.hidden = kind !== "dial";
    page.talkNote.hidden = kind !== "talk";
    page.roomNote.hidden = kind !== "reveal" && kind !== "next-round";
    page.reactionNote.hidden = kind !== "reaction";
    fill(page.deck, model.deckOptions(status, kind, settings));
    fill(page.card, model.cardOptions(status, settings));
    fill(page.element, model.elementOptions(status, settings));
    fill(page.reaction, model.reactionOptions(status, settings));

    page.status.textContent = status.message;
    page.status.dataset.state = status.state;
    const connected = status.state === "connected";
    // The form opens by itself only when the state changes to one that needs pairing; otherwise
    // it stays as the user left it.
    if (status.state !== shownState) {
      shownState = status.state;
      page.pairDetails.open = status.state === "unpaired" || status.state === "refused";
    }
    page.pairSummary.textContent = connected ? "Pair again" : "Pair with Sceneuron";
    page.forgetItem.hidden = status.state === "unpaired";
    page.pairButton.disabled = pairing;
    // The field follows the port the plugin uses until the user types one, and again after a
    // pairing succeeds, so pairing again starts from the port Sceneuron was found on.
    if (!portTyped) {
      page.port.value = String(status.port);
    }
  }

  function choose(field) {
    settings = model.updatedSettings(kind, status, settings, field, page[field].value);
    send({ event: "setSettings", context, payload: settings });
    render();
  }

  function onMessage(event) {
    let message;
    try {
      message = JSON.parse(event.data);
    } catch {
      return;
    }
    if (message.event === "didReceiveSettings" && message.context === context) {
      settings = (message.payload && message.payload.settings) || {};
      render();
    } else if (message.event === "sendToPropertyInspector") {
      const payload = message.payload || {};
      const next = model.readStatus(payload);
      if (next) {
        status = next;
      } else if (payload.type === "paired") {
        pairing = false;
        page.pairResult.textContent = String(payload.message || "");
        page.pairResult.dataset.ok = String(Boolean(payload.ok));
        if (payload.ok) {
          page.code.value = "";
          portTyped = false;
        }
      }
      render();
    }
  }

  function wire() {
    for (const id of [...FIELDS, "port", "code", "status", "forget"]) {
      page[id] = document.getElementById(id);
    }
    Object.assign(page, {
      deckItem: document.getElementById("deck-item"),
      cardItem: document.getElementById("card-item"),
      elementItem: document.getElementById("element-item"),
      reactionItem: document.getElementById("reaction-item"),
      reactionNote: document.getElementById("reaction-note"),
      automationNote: document.getElementById("automation-note"),
      dialNote: document.getElementById("dial-note"),
      talkNote: document.getElementById("talk-note"),
      roomNote: document.getElementById("room-note"),
      pairDetails: document.getElementById("pair-details"),
      pairSummary: document.getElementById("pair-summary"),
      pairForm: document.getElementById("pair-form"),
      pairButton: document.getElementById("pair"),
      pairResult: document.getElementById("pair-result"),
      forgetItem: document.getElementById("forget-item"),
    });
    for (const field of FIELDS) {
      page[field].addEventListener("change", () => choose(field));
    }
    page.port.addEventListener("input", () => {
      portTyped = true;
    });
    page.pairForm.addEventListener("submit", (event) => {
      event.preventDefault();
      pairing = true;
      page.pairResult.textContent = "Pairing...";
      page.pairResult.dataset.ok = "";
      toPlugin({ type: "pair", port: page.port.value, code: page.code.value });
      render();
    });
    page.forget.addEventListener("click", () => {
      page.pairResult.textContent = "";
      toPlugin({ type: "forget" });
    });
  }

  /** Called by Stream Deck once the page has loaded; the name and arguments are fixed by Stream Deck. */
  window.connectElgatoStreamDeckSocket = function (port, uuid, registerEvent, info, actionInfo) {
    context = uuid;
    const action = JSON.parse(actionInfo);
    actionUuid = action.action;
    kind = model.kindOf(actionUuid);
    settings = (action.payload && action.payload.settings) || {};
    socket = new WebSocket(`ws://127.0.0.1:${port}`);
    socket.addEventListener("open", () => {
      send({ event: registerEvent, uuid });
      toPlugin({ type: "refresh" });
    });
    socket.addEventListener("message", onMessage);
    render();
  };

  wire();
  render();
})();
