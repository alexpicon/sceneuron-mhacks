// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the bundled profiles the build writes from the layout: the manifest names one for the
// 15-key Stream Deck and one for the XL, each page of the 15-key profile has every key exactly as
// specified, empty keys included, its corner keys go to the previous and next page in a cycle,
// the XL profile has the very same keys in its top-left five by three and nothing else, every
// deck and card the keys name is in Sceneuron's card packs, and the Reactions page has a key for
// every reaction preset.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { BUILT_IN_REACTIONS } from "../../src/lib/built-in-reactions.ts";
import { COLUMNS, PROFILES, ROWS } from "../../src/profiles/layout.ts";
import { pageFolder } from "../../src/profiles/profile.ts";
import { PLUGIN_DIR, bundleExists } from "../fake-stream-deck.mjs";
import { readZip } from "./zip-reader.mjs";

const skip = bundleExists() ? false : "the plugin is not built; run npm run build";
const manifest = JSON.parse(readFileSync(path.join(PLUGIN_DIR, "manifest.json"), "utf8"));
const CATALOG = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  ...["..", "..", "..", "crates", "sceneuron-core", "src", "packs", "catalog.rs"],
);

/** Each page's keys, top row first, as `describe` names them. */
const EXPECTED = {
  Meeting: [
    ["meeting/wrap-up", "meeting/off-track", "meeting/question", "meeting/offline", "meeting/next-topic"],
    ["meeting/agree", "meeting/disagree", "meeting/cant-hear", "status/muted", "status/brb"],
    ["go to page 4", "talk", "lower meeting", "automation", "go to page 2"],
  ],
  "Planning poker": [
    ["poker/p0", "poker/p1", "poker/p2", "poker/p3", "poker/p5"],
    ["poker/p8", "poker/p13", "poker/p21", "poker/unsure", "poker/break"],
    ["go to page 1", "talk", "reveal", "next-round", "go to page 3"],
  ],
  Status: [
    ["status/muted", "status/brb", "status/lunch", "status/focus", "status/presenting"],
    ["status/on-call", "lower status", "empty", "empty", "empty"],
    ["go to page 2", "talk", "empty", "automation", "go to page 4"],
  ],
  Reactions: [
    ["reaction email", "reaction mind-blown", "reaction ship-it", "reaction coffee", "reaction tongue"],
    [
      "reaction absolutely-right",
      "reaction more-tokens",
      "reaction works-on-my-gpu",
      "reaction meatproxy",
      "reaction clanker",
    ],
    ["go to page 3", "talk", "empty", "automation", "go to page 1"],
  ],
};

/** A key in a few words: a card as "deck/card", Stream Deck's page keys, "empty", or the action's name. */
function describe(action) {
  if (!action) {
    return "empty";
  }
  switch (action.UUID) {
    case "ai.chaski.sceneuron.card":
      return `${action.Settings.deck}/${action.Settings.card}`;
    case "ai.chaski.sceneuron.lower":
      return `lower ${action.Settings.deck}`;
    case "ai.chaski.sceneuron.reaction":
      return `reaction ${action.Settings.reaction}`;
    case "com.elgato.streamdeck.page.goto":
      return `go to page ${action.Settings.PageIndex}`;
    case "com.elgato.streamdeck.page.indicator":
      return "page indicator";
    default:
      return action.UUID.replace("ai.chaski.sceneuron.", "");
  }
}

/** A built profile's pages, in order, each with its name and its keys by "column,row". */
function readProfile(profile) {
  const entries = readZip(readFileSync(path.join(PLUGIN_DIR, `${profile.file}.streamDeckProfile`)));
  const files = entries.filter((entry) => !entry.path.endsWith("/"));
  const json = new Map(files.map((entry) => [entry.path, JSON.parse(entry.data)]));
  const root = [...json.keys()].find((name) => /^[^/]+\.sdProfile\/manifest\.json$/.test(name));
  const folder = root.replace(/manifest\.json$/, "Profiles/");
  return json.get(root).Pages.Pages.map((uuid) => {
    const page = json.get(`${folder}${pageFolder(uuid)}/manifest.json`);
    assert.equal(page.Controllers.length, 1);
    assert.equal(page.Controllers[0].Type, "Keypad");
    return { name: page.Name, keys: page.Controllers[0].Actions };
  });
}

/** Every pack's deck id with its card ids, read from Sceneuron's pack catalog. */
function packDecks() {
  const decks = new Map();
  for (const [, body] of readFileSync(CATALOG, "utf8").matchAll(/^fn \w+\(\) -> Pack \{\n([\s\S]*?)^\}/gm)) {
    const deck = /\bpack\(\s*"([^"]+)"/.exec(body)?.[1];
    if (deck) {
      decks.set(deck, new Set([...body.matchAll(/(?<![\w!])\(\s*"([a-z0-9-]+)",/g)].map(([, id]) => id)));
    }
  }
  return decks;
}

test("the manifest names a profile for the 15-key Stream Deck and one for the XL", { skip }, () => {
  const named = manifest.Profiles.map((entry) => [entry.Name, entry.DeviceType]);
  assert.deepEqual(named, [
    ["profiles/sceneuron-stream-deck", 0],
    ["profiles/sceneuron-stream-deck-xl", 2],
  ]);
  assert.deepEqual(
    named,
    PROFILES.map((profile) => [profile.file, profile.deviceType]),
  );
  for (const profile of PROFILES) {
    assert.ok(existsSync(path.join(PLUGIN_DIR, `${profile.file}.streamDeckProfile`)), profile.file);
  }
});

test("each page of the 15-key profile has every key exactly as specified", { skip }, () => {
  const pages = readProfile(PROFILES[0]);
  assert.deepEqual(
    pages.map((page) => page.name),
    Object.keys(EXPECTED),
  );
  for (const page of pages) {
    const empty = EXPECTED[page.name].flat().filter((key) => key === "empty").length;
    assert.equal(Object.keys(page.keys).length, COLUMNS * ROWS - empty, `${page.name} has no stray key`);
    const at = (column, row) => describe(page.keys[`${column},${row}`]);
    const rows = EXPECTED[page.name].map((keys, row) => keys.map((_, column) => at(column, row)));
    assert.deepEqual(rows, EXPECTED[page.name], page.name);
  }
});

test("the corner keys go to the previous and the next page, around in a cycle", { skip }, () => {
  const pages = readProfile(PROFILES[0]);
  pages.forEach((page, index) => {
    const goes = (position) => page.keys[position].Settings.PageIndex;
    const previous = ((index + pages.length - 1) % pages.length) + 1;
    const next = ((index + 1) % pages.length) + 1;
    assert.deepEqual([goes(`0,${ROWS - 1}`), goes(`${COLUMNS - 1},${ROWS - 1}`)], [previous, next], page.name);
    assert.equal(page.keys[`0,${ROWS - 1}`].States[0].Title, ["Meeting", "Poker", "Status", "Reactions"][previous - 1]);
  });
});

test("the Reactions page has one key for every reaction preset", { skip }, () => {
  const reactions = readProfile(PROFILES[0]).find((page) => page.name === "Reactions");
  const named = Object.values(reactions.keys)
    .filter((action) => action.UUID === "ai.chaski.sceneuron.reaction")
    .map((action) => action.Settings.reaction);
  assert.deepEqual([...named].sort(), BUILT_IN_REACTIONS.map((preset) => preset.id).sort());
});

test("the XL profile has the same keys in its top-left five by three and no others", { skip }, () => {
  const small = readProfile(PROFILES[0]);
  const xl = readProfile(PROFILES[1]);
  assert.equal(xl.length, small.length);
  const same = ({ ActionID, ...rest }) => rest;
  xl.forEach((page, index) => {
    assert.equal(page.name, small[index].name);
    assert.deepEqual(Object.keys(page.keys).sort(), Object.keys(small[index].keys).sort(), "only the top-left keys");
    for (const [position, action] of Object.entries(page.keys)) {
      const [column, row] = position.split(",").map(Number);
      assert.ok(column < COLUMNS && row < ROWS, position);
      assert.deepEqual(same(action), same(small[index].keys[position]), `${page.name} ${position}`);
    }
  });
});

test("every deck and card the profiles name is in Sceneuron's card packs", { skip }, () => {
  const decks = packDecks();
  for (const pack of ["meeting", "poker", "status"]) {
    assert.ok((decks.get(pack)?.size ?? 0) >= 6, `the catalog's ${pack} pack was read`);
  }
  const settings = PROFILES.flatMap(readProfile)
    .flatMap((page) => Object.values(page.keys))
    .filter((action) => action.UUID.startsWith(`${manifest.UUID}.`))
    .map((action) => action.Settings);
  const named = settings.filter((entry) => entry.deck);
  assert.equal(named.length, 2 * (26 + 2), "26 card keys and 2 lower keys in each profile");
  for (const { deck, card } of named) {
    assert.ok(decks.has(deck), `no deck ${deck}`);
    assert.ok(card === undefined || decks.get(deck).has(card), `no card ${card} in ${deck}`);
  }
});
