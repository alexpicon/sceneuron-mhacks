// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises building a profile file: page folders named the way Stream Deck's own default profiles
// name them, UUIDs made from hashes, the profile and page manifests with Sceneuron's actions and
// Stream Deck's page actions, the same layout making the same bytes, and layouts that cannot be
// built refused.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import { PAGES, PROFILES } from "../../src/profiles/layout.ts";
import { hashedUuid, pageFolder, pluginInfo, profileEntries, profileFile } from "../../src/profiles/profile.ts";
import { PLUGIN_DIR } from "../fake-stream-deck.mjs";

const plugin = pluginInfo(JSON.parse(readFileSync(path.join(PLUGIN_DIR, "manifest.json"), "utf8")));
const [deck] = PROFILES;

/** The JSON files of `entries` by path. */
function files(entries) {
  return new Map(entries.filter((entry) => entry.data).map((entry) => [entry.path, JSON.parse(entry.data)]));
}

test("page folders are named the way Stream Deck's own profiles name them", () => {
  // Page UUIDs and their folders in the default profile Stream Deck 7.4 ships for the 15-key Stream Deck.
  const shipped = [
    ["47be3134-6c33-4223-9612-b2e1d6b685d1", "8VW32D3C6D1275GIMBGTDDK5Q4Z"],
    ["52f2ee5d-f8e9-43fc-ac58-cba11053190b", "ABPESNFOT51WPB2OPEGH0KOP1CZ"],
    ["be429f7a-7513-4132-bd6b-82c5de1fdb8f", "NP19VVJL2D0J5FBBGB2TS7VRHSZ"],
    ["26aa55e8-a071-47f2-b9de-7f977b32b734", "4QL5BQ50E53W5EEVFVBNMCLN6GZ"],
    ["173a7666-09e7-444f-bbe4-649f2ef4355a", "2ST7CPG9ST24WEW4CIFITT1LB8Z"],
    ["439f5e1a-ccf9-49d7-8e06-551748ae2ba9", "8EFLS6MCW54TF3G6AKBKHBHBL4Z"],
    ["77bdb0f8-4726-4bb0-9e79-19905e871f3b", "EVVR1V274P5R17JP3685T1OW7CZ"],
  ];
  for (const [uuid, folder] of shipped) {
    assert.equal(pageFolder(uuid), folder, uuid);
  }
});

test("hashed UUIDs are version 4 UUIDs that follow their seed", () => {
  const uuid = hashedUuid("seed");
  assert.match(uuid, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.equal(hashedUuid("seed"), uuid);
  assert.notEqual(hashedUuid("other seed"), uuid);
});

test("the profile manifest lists the pages, the first one current, and an empty default page", () => {
  const entries = profileEntries(deck, PAGES, plugin);
  const root = entries[0].path;
  assert.match(root, /^[0-9A-F]{8}-[0-9A-F]{4}-4[0-9A-F]{3}-[89AB][0-9A-F]{3}-[0-9A-F]{12}\.sdProfile\/$/);
  const json = files(entries);
  const manifest = json.get(`${root}manifest.json`);
  assert.deepEqual(manifest.Device, { Model: "20GAA9902", UUID: "" });
  assert.deepEqual([manifest.Name, manifest.Version], ["Sceneuron", "2.0"]);
  assert.equal(manifest.Pages.Pages.length, 4);
  assert.equal(manifest.Pages.Current, manifest.Pages.Pages[0]);
  const page = (uuid) => json.get(`${root}Profiles/${pageFolder(uuid)}/manifest.json`);
  const empty = { Controllers: [{ Actions: {}, Type: "Keypad" }], Icon: "", Name: "" };
  assert.deepEqual(page(manifest.Pages.Default), empty);
  const names = manifest.Pages.Pages.map((uuid) => page(uuid).Name);
  assert.deepEqual(names, ["Meeting", "Planning poker", "Status", "Reactions"]);
  assert.equal(entries.filter((entry) => entry.path.endsWith("/")).every((entry) => !entry.data), true);
});

test("keys carry the plugin's actions with their settings, Stream Deck's page actions, and gaps", () => {
  const entries = profileEntries(deck, PAGES, plugin);
  const json = files(entries);
  const manifest = json.get(`${entries[0].path}manifest.json`);
  const keys = (index) => {
    const folder = pageFolder(manifest.Pages.Pages[index]);
    return json.get(`${entries[0].path}Profiles/${folder}/manifest.json`).Controllers[0].Actions;
  };
  const first = keys(0);
  assert.deepEqual(first["0,0"].Plugin, { Name: "Sceneuron", UUID: "ai.chaski.sceneuron", Version: "0.4.0.0" });
  assert.deepEqual([first["0,0"].Name, first["0,0"].UUID], ["Raise a card", "ai.chaski.sceneuron.card"]);
  assert.deepEqual(first["0,0"].Settings, { deck: "meeting", card: "wrap-up" });
  assert.deepEqual(first["2,2"].Settings, { deck: "meeting" });
  assert.equal(first["3,2"].States.length, 2, "automation has two states");
  assert.deepEqual(first["0,2"].Plugin, { Name: "Pages", UUID: "com.elgato.streamdeck.page", Version: "1.0" });
  assert.deepEqual([first["0,2"].UUID, first["0,2"].Settings], ["com.elgato.streamdeck.page.goto", { PageIndex: 4 }]);
  assert.equal(first["0,2"].States[0].Title, "Reactions");
  assert.deepEqual([first["4,2"].UUID, first["4,2"].Settings], ["com.elgato.streamdeck.page.goto", { PageIndex: 2 }]);
  assert.equal(first["4,2"].States[0].Title, "Poker");
  const third = keys(2);
  assert.deepEqual(
    ["2,1", "3,1", "4,1", "2,2"].filter((position) => position in third),
    [],
    "a null key stays empty",
  );
  const fourth = keys(3);
  assert.deepEqual([fourth["0,0"].Name, fourth["0,0"].UUID], ["Reaction", "ai.chaski.sceneuron.reaction"]);
  assert.deepEqual(fourth["0,0"].Settings, { reaction: "email" });
  assert.deepEqual(
    ["0,1", "1,1", "2,1", "3,1", "4,1"].map((position) => fourth[position].Settings.reaction),
    ["absolutely-right", "more-tokens", "works-on-my-gpu", "meatproxy", "clanker"],
    "the reactions' second row holds the AI-era presets",
  );
  const ids = [0, 1, 2, 3].flatMap((index) => Object.values(keys(index))).map((action) => action.ActionID);
  assert.equal(new Set(ids).size, 55, "every key has its own ActionID");
});

test("the same layout makes the same file, and a changed layout new UUIDs", () => {
  assert.deepEqual(profileFile(deck, PAGES, plugin), profileFile(deck, PAGES, plugin));
  const renamed = [{ ...PAGES[0], name: "Meetings" }, PAGES[1]];
  const before = files(profileEntries(deck, PAGES, plugin));
  const after = files(profileEntries(deck, renamed, plugin));
  const uuids = (json) => JSON.stringify([...json.values()]).match(/[0-9a-f]{8}-[0-9a-f-]{27}/g);
  assert.equal(uuids(before).filter((uuid) => uuids(after).includes(uuid)).length, 0);
});

test("a layout that does not fit, or names an action the plugin lacks, is refused", () => {
  const wide = [{ name: "Wide", rows: [[...PAGES[0].rows[0], { kind: "talk" }]] }];
  assert.throws(() => profileEntries(deck, wide, plugin), /does not fit the 5 by 3 keys/);
  const unknown = [{ name: "Odd", rows: [[{ kind: "juggle" }]] }];
  assert.throws(() => profileEntries(deck, unknown, plugin), /no ai\.chaski\.sceneuron\.juggle/);
});
