// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises how editor.html loads the editor: its classic scripts run in the page's order, each after
// the scripts it uses, and each adds the namespace entry named after it to window.SceneuronEditor.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const web = new URL("../", import.meta.url);
const page = readFileSync(new URL("editor.html", web), "utf8");
const scripts = Array.from(page.matchAll(/<script src="\/assets\/([^"]+)" defer><\/script>/g), (match) => match[1]);

test("the page loads the link check, the renderer with its themes, and its floods first and starts the editor last", () => {
  assert.deepEqual(scripts.slice(0, 22), [
    "link.js",
    "render-style.js",
    "themes/graphite.js",
    "themes/hacker.js",
    "themes/eighties.js",
    "themes/windows-95.js",
    "themes/classic-mac.js",
    "themes/vaporwave.js",
    "render-themes.js",
    "render-fit.js",
    "icons.js",
    "render-icons.js",
    "render-results.js",
    "render-cards.js",
    "render-clock.js",
    "render-subtitles.js",
    "render.js",
    "reaction-presets.js",
    "flood-banner.js",
    "flood-burst.js",
    "flood.js",
    "editor/constants.js",
  ]);
  assert.equal(scripts.at(-1), "editor/main.js");
  assert.equal(new Set(scripts).size, scripts.length, "no script is loaded twice");
});

test("every script before main.js runs after the scripts it uses", () => {
  // Classic scripts share one global object, which the page also calls window.
  const context = vm.createContext({});
  context.window = context;
  for (const script of scripts.slice(0, -1)) {
    vm.runInContext(readFileSync(new URL(script, web), "utf8"), context, { filename: script });
  }
  assert.equal(typeof context.SceneuronRender.render, "function");
  assert.equal(context.SceneuronThemes.THEMES.length, 7);
  assert.equal(typeof context.SceneuronFlood.update, "function");
  assert.equal(typeof context.SceneuronFloodBanner.planBanner, "function");
  assert.equal(context.SceneuronReactionPresets.PRESETS[1].id, "email");
  assert.equal(typeof context.SceneuronLink.verifyServer, "function");
  assert.deepEqual(Object.keys(context.SceneuronEditor), [
    "constants",
    "values",
    "project",
    "triggers",
    "placeholders",
    "conditionEdits",
    "deckEdits",
    "cardsText",
    "signalsContext",
    "signalsText",
    "roomText",
    "planText",
    "aiText",
    "consentText",
    "voiceText",
    "voiceGuideText",
    "languages",
    "languageText",
    "subtitlesText",
    "cameraText",
    "micText",
    "streamDeckText",
    "reactionsText",
    "settingsFile",
    "snapshot",
    "drag",
    "api",
    "dom",
    "createEditor",
    "notifications",
    "status",
    "aiStatus",
    "events",
    "elements",
    "preview",
    "propertyFields",
    "properties",
    "placeholderChips",
    "live",
    "ruleCards",
    "rules",
    "aiFields",
    "conditionCards",
    "conditions",
    "settings",
    "settingsTransfer",
    "theme",
    "reactions",
    "aiSettings",
    "voiceFields",
    "voice",
    "voiceGuide",
    "subtitlesFields",
    "subtitles",
    "planView",
    "planSession",
    "assistant",
    "setupWizard",
    "cameraFields",
    "camera",
    "cameraPreview",
    "microphones",
    "deckVoice",
    "deckCards",
    "cards",
    "packs",
    "roomMarkup",
    "room",
    "streamDeck",
    "tabs",
    "sync",
  ]);
});
