// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises the agreement between the manifest and the code: action UUIDs under the plugin's,
// one action class per manifest action, an inspector kind for each, files that exist, the dial
// layout's pixmap key that the board draws into, a Stream Deck new enough for the editor's
// passive pairing link, and macOS as the only system, as for Sceneuron.

import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { runInThisContext } from "node:vm";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const pluginDir = path.join(root, "ai.chaski.sceneuron.sdPlugin");
const manifest = JSON.parse(readFileSync(path.join(pluginDir, "manifest.json"), "utf8"));

function image(base) {
  return [".svg", ".png"].some((extension) => existsSync(path.join(pluginDir, `${base}${extension}`)));
}

test("every action UUID sits under the plugin's", () => {
  assert.equal(manifest.UUID, "ai.chaski.sceneuron");
  for (const action of manifest.Actions) {
    assert.ok(action.UUID.startsWith(`${manifest.UUID}.`), action.UUID);
  }
});

test("each manifest action has exactly one action class, and each class is in the manifest", () => {
  const actionsDir = path.join(root, "src", "actions");
  const declared = readdirSync(actionsDir).flatMap((file) =>
    [...readFileSync(path.join(actionsDir, file), "utf8").matchAll(/@action\(\{ UUID: "([^"]+)" \}\)/g)].map(
      ([, uuid]) => uuid,
    ),
  );
  assert.deepEqual([...declared].sort(), manifest.Actions.map((action) => action.UUID).sort());
});

test("the inspector knows every action", () => {
  const source = readFileSync(path.join(pluginDir, "ui", "inspector-model.js"), "utf8");
  const module = { exports: {} };
  runInThisContext(`(function (module) {\n${source}\n})`)(module);
  for (const action of manifest.Actions) {
    assert.notEqual(module.exports.kindOf(action.UUID), null, action.UUID);
  }
});

test("every file the manifest names exists", () => {
  assert.ok(image(manifest.CategoryIcon) && existsSync(path.join(pluginDir, `${manifest.Icon}.png`)));
  assert.ok(existsSync(path.join(pluginDir, `${manifest.Icon}@2x.png`)));
  for (const action of manifest.Actions) {
    assert.ok(image(action.Icon), action.Icon);
    assert.ok(existsSync(path.join(pluginDir, action.PropertyInspectorPath)), action.PropertyInspectorPath);
    for (const state of action.States) {
      assert.ok(image(state.Image), state.Image);
    }
  }
  assert.equal(manifest.CodePath, "bin/plugin.js");
  assert.match(readFileSync(path.join(root, "rollup.config.mjs"), "utf8"), /file: `\$\{sdPlugin\}\/bin\/plugin\.js`/);
});

test("the dial layout has the one pixmap the board draws", () => {
  const dial = manifest.Actions.find((action) => action.Controllers.includes("Encoder"));
  const layout = JSON.parse(readFileSync(path.join(pluginDir, dial.Encoder.layout), "utf8"));
  assert.deepEqual(layout.items, [{ key: "card", type: "pixmap", rect: [0, 0, 200, 100] }]);
  assert.match(readFileSync(path.join(root, "src", "board.ts"), "utf8"), /setFeedback\(\{ card: /);
});

test("the automation action has two states that the plugin sets itself", () => {
  const automation = manifest.Actions.find((action) => action.UUID.endsWith(".automation"));
  assert.equal(automation.States.length, 2);
  assert.equal(automation.DisableAutomaticStates, true);
});

test("the Stream Deck the plugin needs takes passive deep links, which the editor pairs with", () => {
  // Elgato: deep links reach plugins from Stream Deck 6.5, and passive ones (streamdeck=hidden) from 7.0.
  const major = Number(manifest.Software.MinimumVersion.split(".")[0]);
  assert.ok(major >= 7, manifest.Software.MinimumVersion);
});

test("the plugin is offered on macOS only, as Sceneuron is", () => {
  assert.deepEqual(manifest.OS.map((system) => system.Platform), ["mac"]);
});
