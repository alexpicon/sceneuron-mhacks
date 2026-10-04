// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/panels/ai-fields.js: the providers the AI tab's provider form offers, in order, each of them named
// where the consent says data goes.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import vm from "node:vm";

const require = createRequire(import.meta.url);
const C = require("../editor/consent-text.js");
const source = readFileSync(new URL("../editor/panels/ai-fields.js", import.meta.url), "utf8");

/** Builds the AI tab with markup helpers that keep the tree as plain objects; returns its top nodes. */
function buildPanel() {
  const node = (tag, attrs, children) => ({ tag, attrs: { ...attrs }, children: children.flat() });
  const dom = {
    h: (tag, attrs, ...children) => node(tag, attrs, children),
    option: (value, label) => node("option", { value }, [label]),
    field: (id, label, control) => node("div", { class: "field" }, [node("label", { for: id }, [label]), control]),
    check: (id, label, input) => node("div", { class: "check" }, [input, node("label", { for: id }, [label])]),
  };
  const context = vm.createContext({ SceneuronEditor: { dom } });
  context.window = context;
  vm.runInContext(source, context, { filename: "editor/panels/ai-fields.js" });
  let top = [];
  context.SceneuronEditor.aiFields.buildAiPanel({ replaceChildren: (...children) => (top = children) });
  return top;
}

function find(nodes, id) {
  for (const node of nodes) {
    if (node && typeof node === "object") {
      const found = node.attrs.id === id ? node : find(node.children, id);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

test("the provider form offers Google Gemini first, then an OpenAI-compatible service", () => {
  const select = find(buildPanel(), "ai-provider");
  assert.equal(select.tag, "select");
  assert.equal(select.attrs["data-setting"], "provider");
  const choices = select.children.map((option) => [option.attrs.value, option.children[0]]);
  assert.deepEqual(choices, [
    ["gemini", "Google Gemini"],
    ["openai-compatible", "OpenAI-compatible (NVIDIA NIM, Ollama, llama.cpp)"],
  ]);
});

test("the consent names every provider the form offers", () => {
  const select = find(buildPanel(), "ai-provider");
  for (const option of select.children) {
    const where = C.destination({ provider: option.attrs.value, base_url: "https://example.com" });
    assert.doesNotMatch(where, /the AI provider/, option.attrs.value);
  }
});
