// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the inspector page script in a fake DOM and Stream Deck socket: the Port field follows
// the port the plugin uses until the user types one, the pairing form opens when pairing is
// needed, keys without settings show their own note, and the page never asks for the global
// settings that hold the token.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { createContext, runInContext } from "node:vm";

import { inspectorStatus } from "../../src/lib/inspector.ts";
import { sampleView } from "../fake-engine.mjs";
import { PLUGIN_DIR } from "../fake-stream-deck.mjs";

/** Just enough of a DOM element for the page: fields, child options, and listeners. */
class FakeElement {
  hidden = false;
  open = false;
  disabled = false;
  value = "";
  textContent = "";
  dataset = {};
  children = [];
  listeners = new Map();

  addEventListener(type, listener) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
  }

  dispatch(type, event = {}) {
    for (const listener of this.listeners.get(type) ?? []) {
      listener({ preventDefault() {}, ...event });
    }
  }

  replaceChildren(...children) {
    this.children = children;
  }
}

/** The page's WebSocket to Stream Deck; `sent` holds what the page sent, parsed. */
class FakeSocket extends FakeElement {
  static OPEN = 1;
  readyState = FakeSocket.OPEN;
  sent = [];

  send(text) {
    this.sent.push(JSON.parse(text));
  }

  /** Delivers `message` from Stream Deck to the page. */
  receive(message) {
    this.dispatch("message", { data: JSON.stringify(message) });
  }
}

/** Loads the page for a key of action `uuid` and connects it the way Stream Deck does. */
function openPage(uuid = "ai.chaski.sceneuron.card") {
  const elements = new Map();
  const document = {
    getElementById: (id) => elements.get(id) ?? elements.set(id, new FakeElement()).get(id),
    createElement: () => new FakeElement(),
  };
  let socket = null;
  const WebSocket = class extends FakeSocket {
    constructor() {
      super();
      socket = this;
    }
  };
  const context = createContext({ document, WebSocket });
  context.window = context;
  for (const file of ["inspector-model.js", "inspector.js"]) {
    runInContext(readFileSync(path.join(PLUGIN_DIR, "ui", file), "utf8"), context);
  }
  const action = { action: uuid, payload: { settings: {} } };
  context.connectElgatoStreamDeckSocket(28196, "key-1", "registerPropertyInspector", "{}", JSON.stringify(action));
  socket.dispatch("open");
  const status = (state, port) => {
    const payload = inspectorStatus(state, port, state === "connected" ? sampleView() : null);
    socket.receive({ event: "sendToPropertyInspector", context: "key-1", payload });
  };
  return { element: (id) => document.getElementById(id), socket, status };
}

test("the Port field follows the plugin's port until the user types one", () => {
  const page = openPage();
  const port = page.element("port");
  assert.equal(port.value, "4466", "the default while the plugin has not answered");
  page.status("refused", 5123);
  assert.equal(port.value, "5123");
  page.status("refused", 5124);
  assert.equal(port.value, "5124");

  port.value = "6000";
  port.dispatch("input");
  page.status("refused", 5125);
  assert.equal(port.value, "6000", "what the user typed stays");

  page.socket.receive({ event: "sendToPropertyInspector", payload: { type: "paired", ok: true, message: "Paired" } });
  page.status("connected", 6000);
  page.status("connected", 6001);
  assert.equal(port.value, "6001", "after pairing the field follows the plugin again");
});

test("the pairing form opens when pairing is needed and stays as the user leaves it", () => {
  const page = openPage();
  const details = page.element("pair-details");
  page.status("connected", 4466);
  assert.equal(details.open, false);
  assert.equal(page.element("pair-summary").textContent, "Pair again");
  page.status("refused", 4466);
  assert.equal(details.open, true);
  details.open = false;
  page.status("refused", 4466);
  assert.equal(details.open, false);
});

test("the push to talk and room keys show their own note and no fields", () => {
  const notes = ["automation-note", "dial-note", "talk-note", "room-note"];
  const shown = (page) => notes.filter((id) => !page.element(id).hidden);
  const fields = (page) => ["deck-item", "card-item", "element-item"].filter((id) => !page.element(id).hidden);
  for (const [action, note] of [
    ["talk", "talk-note"],
    ["reveal", "room-note"],
    ["next-round", "room-note"],
  ]) {
    const page = openPage(`ai.chaski.sceneuron.${action}`);
    assert.deepEqual(shown(page), [note], action);
    assert.deepEqual(fields(page), [], action);
  }
  assert.deepEqual(shown(openPage()), [], "a card key shows no note");
});

test("the page sends the port and code to the plugin and never asks for global settings", () => {
  const page = openPage();
  page.status("unpaired", 4466);
  page.element("code").value = "123 456";
  page.element("pair-form").dispatch("submit");
  const pair = page.socket.sent.find((message) => message.payload?.type === "pair");
  assert.deepEqual(pair, {
    event: "sendToPlugin",
    action: "ai.chaski.sceneuron.card",
    context: "key-1",
    payload: { type: "pair", port: "4466", code: "123 456" },
  });
  assert.ok(page.socket.sent.every((message) => !/GlobalSettings/.test(message.event)));
});
