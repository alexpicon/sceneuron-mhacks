// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import streamDeck from "@elgato/streamdeck";

import { AutomationKey } from "./actions/automation.ts";
import { CardKey } from "./actions/card.ts";
import { CardDial } from "./actions/dial.ts";
import { ElementKey } from "./actions/element.ts";
import { LowerKey } from "./actions/lower.ts";
import { ReactionKey } from "./actions/reaction.ts";
import { NextRoundKey, RevealKey } from "./actions/room.ts";
import { TalkKey } from "./actions/talk.ts";
import { Board } from "./board.ts";
import { pairFromDeepLinks } from "./deep-link.ts";
import { bridgeInspector } from "./inspector-bridge.ts";
import { Link } from "./lib/link.ts";
import { readGlobalSettings } from "./lib/settings.ts";

// The default log level keeps Stream Deck's messages out of the log; at "trace" they would
// include the global settings, and with them the token.
const link = new Link({ log: (message) => streamDeck.logger.info(message) });
const board = new Board(link);

streamDeck.actions.registerAction(new CardKey(board));
streamDeck.actions.registerAction(new LowerKey(board));
streamDeck.actions.registerAction(new ElementKey(board));
streamDeck.actions.registerAction(new AutomationKey(board));
streamDeck.actions.registerAction(new CardDial(board));
streamDeck.actions.registerAction(new TalkKey(board));
streamDeck.actions.registerAction(new RevealKey(board));
streamDeck.actions.registerAction(new NextRoundKey(board));
streamDeck.actions.registerAction(new ReactionKey(board));
bridgeInspector(link);
const saved = Promise.withResolvers<void>();
pairFromDeepLinks(link, saved.promise);
streamDeck.devices.onDeviceDidDisconnect((ev) => {
  // The SDK passes no device for one it was never told about.
  const device: { id: string } | undefined = ev.device;
  if (device) {
    board.letGo(() => board.talk.releaseDevice(device.id));
  }
});
streamDeck.settings.onDidReceiveGlobalSettings((ev) => {
  const { port, token } = readGlobalSettings(ev.settings);
  link.configure(port, token);
});

await streamDeck.connect();
const { port, token } = readGlobalSettings(await streamDeck.settings.getGlobalSettings());
link.configure(port, token);
saved.resolve();
