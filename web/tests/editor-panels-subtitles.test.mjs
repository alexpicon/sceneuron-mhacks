// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/panels/subtitles.js with the language pickers of editor/panels/subtitles-fields.js: the settings
// form keeps showing the saved translation model when the server changes it, as it does when the AI provider changes,
// and keeps an edit that is not saved yet; targets show by name, or as a typed code when not listed, and save as
// picked or typed, each with its own script kept unless unticked; the language labels' switch; and the spoken
// language offers the languages Scribe hears.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { describe, test } from "node:test";
import vm from "node:vm";

const require = createRequire(import.meta.url);
const languageText = require("../editor/language-text.js");
const subtitlesText = require("../editor/subtitles-text.js");
const read = (path) => readFileSync(new URL(`../editor/panels/${path}`, import.meta.url), "utf8");
const sources = [["subtitles-fields.js", read("subtitles-fields.js")], ["subtitles.js", read("subtitles.js")]];

const LOCAL = "llama3.2";
const FLASH_LITE = "gemini-3.5-flash-lite";

function aiSettings(provider, model, extra) {
  return {
    available: true,
    provider,
    subtitles: {
      consent: false,
      enabled: false,
      language: "",
      targets: [],
      translation_model: model,
      lines: 2,
      clear_after_ms: 4000,
      hide_commands: true,
      stream_captions: false,
      stream_language: "",
      labels: true,
      latin_targets: [],
      latin_lines: false,
      ...extra,
    },
  };
}

/** Wires the section to stand-in elements, which hold only the state the section sets; returns the view. */
function setup(ai) {
  const elements = new Map();
  const invalid = new Map();
  const sent = [];
  const byId = (id) => {
    if (!elements.has(id)) {
      const listeners = {};
      elements.set(id, {
        id,
        value: "",
        checked: false,
        disabled: false,
        hidden: false,
        focused: false,
        textContent: "",
        dataset: {},
        children: [],
        listeners,
        addEventListener(type, listener) {
          listeners[type] = listener;
        },
        replaceChildren(...children) {
          this.children = children;
        },
        focus() {
          this.focused = true;
        },
      });
    }
    return elements.get(id);
  };
  const dom = {
    clearInvalid: (input) => invalid.delete(input.id),
    setInvalid: (input, message) => invalid.set(input.id, message),
    h: () => ({}),
    option: (value, label) => ({ value, label }),
  };
  const context = vm.createContext({ SceneuronEditor: { dom, languageText, subtitlesText } });
  context.window = context;
  for (const [filename, source] of sources) {
    vm.runInContext(source, context, { filename: `editor/panels/${filename}` });
  }
  const app = { ai, live: null };
  const api = {
    async updateAiSettings(change) {
      // As JSON, as it goes to the server, so its arrays compare equal outside the section's context.
      sent.push(JSON.parse(JSON.stringify(change)));
      return app.ai;
    },
  };
  const editor = { app, byId, api, views: { aiSettings: { render() {} } }, announce() {}, report() {}, toast() {} };
  const view = context.SceneuronEditor.subtitles(editor);
  view.render();
  return { app, byId, invalid, sent, view };
}

/** Picks `value` in a select, as a user's choice does. */
function choose(byId, id, value) {
  byId(id).value = value;
  byId(id).listeners.change();
}

async function save(byId) {
  await byId("subtitles-form").listeners.submit({ preventDefault() {} });
}

describe("the translation model field", () => {
  test("follows a model the server saved, such as the new provider's default", () => {
    const { app, byId, view } = setup(aiSettings("openai-compatible", LOCAL));
    assert.equal(byId("subtitles-model").value, LOCAL);
    app.ai = aiSettings("gemini", FLASH_LITE);
    view.render();
    assert.equal(byId("subtitles-model").value, FLASH_LITE);
  });

  test("keeps an edit that is not saved yet, and the form's other fields", () => {
    const { app, byId, view } = setup(aiSettings("openai-compatible", LOCAL));
    byId("subtitles-model").value = "my-translator";
    byId("subtitles-target-1").value = "es";
    app.ai = aiSettings("gemini", FLASH_LITE);
    view.render();
    assert.equal(byId("subtitles-model").value, "my-translator");
    assert.equal(byId("subtitles-target-1").value, "es");
  });

  test("follows again once the edit is undone", () => {
    const { app, byId, view } = setup(aiSettings("openai-compatible", LOCAL));
    byId("subtitles-model").value = "my-translator";
    app.ai = aiSettings("gemini", FLASH_LITE);
    view.render();
    byId("subtitles-model").value = ` ${FLASH_LITE} `;
    app.ai = aiSettings("openai-compatible", LOCAL);
    view.render();
    assert.equal(byId("subtitles-model").value, LOCAL);
  });
});

describe("the language pickers", () => {
  test("show saved targets by name, and a code that is not listed as typed", () => {
    const { byId } = setup(aiSettings("gemini", FLASH_LITE, { targets: ["es", "qya", "art-x-huttese"] }));
    const shown = [1, 2, 3].map((n) => {
      const id = `subtitles-target-${n}`;
      return [byId(id).value, byId(`${id}-code`).value, byId(`${id}-code-field`).hidden];
    });
    assert.deepEqual(shown, [["es", "", true], ["qya", "", true], ["other", "art-x-huttese", false]]);
  });

  test("save a language picked by name and a code typed after choosing another", async () => {
    const { byId, sent } = setup(aiSettings("gemini", FLASH_LITE, { targets: ["es"] }));
    choose(byId, "subtitles-target-2", "qya");
    assert.equal(byId("subtitles-target-2-code-field").hidden, true);
    choose(byId, "subtitles-target-3", "other");
    assert.equal(byId("subtitles-target-3-code-field").hidden, false, "the code field shows");
    assert.equal(byId("subtitles-target-3-code").focused, false, "without taking the focus");
    byId("subtitles-target-3-code").value = " art-x-dothraki ";
    await save(byId);
    assert.deepEqual(sent, [{ subtitles: { targets: ["es", "qya", "art-x-dothraki"] } }]);
  });

  test("mark a typed code the server would refuse on its field, and send nothing", async () => {
    const { byId, invalid, sent } = setup(aiSettings("gemini", FLASH_LITE));
    choose(byId, "subtitles-target-1", "other");
    byId("subtitles-target-1-code").value = "x-dothraki";
    await save(byId);
    assert.deepEqual([...invalid], [
      ["subtitles-target-1-code", '"x-dothraki" is not a language code such as es, pt-BR, zh-Hans, or art-x-dothraki.'],
    ]);
    assert.deepEqual(sent, []);
    choose(byId, "subtitles-target-1", "tlh");
    choose(byId, "subtitles-target-2", "TLH".toLowerCase());
    await save(byId);
    assert.deepEqual([...invalid], [["subtitles-target-2", "tlh is listed twice."]], "a repeat is marked where it is");
  });

  test("keep a picked language's own script unless unticked, and save the ones kept in Latin letters", async () => {
    const saved = { targets: ["qya", "es", "tlh"], latin_targets: ["tlh"] };
    const { byId, sent } = setup(aiSettings("gemini", FLASH_LITE, saved));
    const script = (n) => {
      const id = `subtitles-target-${n}-script`;
      return [byId(`${id}-field`).hidden, byId(`${id}-label`).textContent, byId(id).checked];
    };
    assert.deepEqual(script(1), [false, "Write Quenya (Elvish) in Tengwar", true]);
    assert.deepEqual(script(2), [true, "", true], "Spanish has no script of its own");
    assert.deepEqual(script(3), [false, "Write Klingon in pIqaD", false], "Klingon was kept in Latin letters");
    byId("subtitles-target-1-script").checked = false;
    byId("subtitles-target-3-script").checked = true;
    byId("subtitles-latin-lines").checked = true;
    await save(byId);
    assert.deepEqual(sent, [{ subtitles: { latin_targets: ["qya"], latin_lines: true } }]);
    choose(byId, "subtitles-target-2", "tok");
    assert.deepEqual(script(2), [false, "Write Toki Pona in sitelen pona", true], "a language picked anew");
    choose(byId, "subtitles-target-2", "other");
    assert.equal(byId("subtitles-target-2-script-field").hidden, true, "a typed code offers no box");
  });

  test("name each language on the overlay until the switch is turned off", () => {
    const { byId, sent } = setup(aiSettings("gemini", FLASH_LITE));
    assert.equal(byId("subtitles-labels").checked, true, "labels start on");
    byId("subtitles-labels").checked = false;
    byId("subtitles-labels").listeners.change();
    assert.deepEqual(sent, [{ subtitles: { labels: false } }]);
    const off = setup(aiSettings("gemini", FLASH_LITE, { labels: false }));
    assert.equal(off.byId("subtitles-labels").checked, false);
  });

  test("offer the spoken languages Scribe hears, keeping a saved code that list lacks", () => {
    const heard = setup(aiSettings("gemini", FLASH_LITE, { language: "es" })).byId("subtitles-language");
    assert.equal(heard.value, "es");
    assert.equal(heard.children.length, 101);
    assert.deepEqual(heard.children[0], { value: "", label: "Detect it automatically" });
    assert.ok(!heard.children.some(({ value }) => value === "qya"), "no constructed language");
    const typed = setup(aiSettings("gemini", FLASH_LITE, { language: "spa" })).byId("subtitles-language");
    assert.deepEqual(typed.children[1], { value: "spa", label: "spa" });
    assert.equal(typed.value, "spa");
  });
});
