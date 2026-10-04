// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/placeholders.js: reading and filling "{{name}}" placeholders like the server does, lines of
// placeholders alone that show nothing left out, text that shows subtitles, the preview's stand-ins, live
// description, caption with its age, and live subtitles, unknown names, the placeholder list, inserting at the caret,
// and renaming.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { describe, test } from "node:test";

const require = createRequire(import.meta.url);
const P = require("../editor/placeholders.js");

const draft = {
  conditions: [
    { id: "showing-item", name: "Showing something", detail: "the object's name" },
    { id: "pet-in-view", name: "Pet on camera", detail: "" },
  ],
};

describe("reading and filling", () => {
  test("names come in order, trimmed and without repeats", () => {
    assert.deepEqual(P.placeholderNames("{{ scene }}: {{showing-item}} and {{scene}}"), ["scene", "showing-item"]);
    assert.deepEqual(P.placeholderNames("no placeholders"), []);
  });

  test("filling matches the server: unclosed braces stay as written", () => {
    const value = (name) => `<${name}>`;
    assert.equal(P.renderPlaceholders("Now showing: {{ showing-item }}", value), "Now showing: <showing-item>");
    assert.equal(P.renderPlaceholders("a {{b}} c {{d", value), "a <b> c {{d");
    assert.equal(P.renderPlaceholders("{{}}", value), "<>");
    assert.equal(P.renderPlaceholders("plain", value), "plain");
  });

  test("a line of placeholders alone that all show nothing is left out with its line break, as the server does", () => {
    const value = (name) => (name === "seen" ? "1 person" : "");
    assert.equal(P.renderPlaceholders("{{seen}}\n{{scene:age}}", value), "1 person");
    assert.equal(P.renderPlaceholders("{{scene:age}}\n{{seen}}", value), "1 person");
    assert.equal(P.renderPlaceholders("{{room}}\n  {{scene}} ", value), "");
    const text = "Top\n{{room}}{{typo}}\nCamera: {{scene}}\n\n{{ unclosed\nEnd";
    assert.equal(P.renderPlaceholders(text, value), "Top\nCamera: \n\n{{ unclosed\nEnd");
    const lines = () => "one\ntwo";
    assert.equal(P.renderPlaceholders("{{subtitles}}\n{{x}}", lines), "one\ntwo\none\ntwo");
  });

  test("text shows subtitles when it names them in any language, as the server marks it", () => {
    for (const text of ["{{subtitles}}", "Live: {{ subtitles:es }}", "{{scene}}\n{{subtitles:1}}"]) {
      assert.ok(P.showsSubtitles(text), text);
    }
    for (const text of ["Subtitles", "{{scene}}", "{{subtitles-2}}", "{{subtitles", undefined]) {
      assert.ok(!P.showsSubtitles(text), String(text));
    }
  });
});

describe("previewValues", () => {
  test("live values win, and missing ones show a bracketed stand-in", () => {
    const live = {
      scene: { caption: "A person waves." },
      conditions: [{ id: "showing-item", detail: "Coffee mug" }, { id: "pet-in-view", detail: "" }],
    };
    const value = P.previewValues(draft, live);
    assert.equal(value("scene"), "A person waves.");
    assert.equal(value("showing-item"), "Coffee mug");
    assert.equal(value("pet-in-view"), "[Pet on camera]");
    assert.equal(value("room"), "[Room votes]", "the room's votes are not in the snapshot");
    assert.equal(value("typo"), "", "unknown names show nothing, as on stream");
  });

  test("the live description and the caption with its age come from the snapshot", () => {
    const live = { seen: "1 person; mouth open", scene: { caption: "A person waves.", at_ms: 5, age: "10 s ago" } };
    const value = P.previewValues(draft, live);
    assert.equal(value("seen"), "1 person; mouth open");
    assert.equal(value("scene:age"), "A person waves. (10 s ago)");
    assert.equal(value("scene: age"), "A person waves. (10 s ago)");
    assert.equal(value("scene:agee"), "", "an unknown variant shows nothing");
  });

  test("before the AI reported anything every known placeholder has a stand-in", () => {
    const value = P.previewValues(draft, null);
    assert.equal(value("scene"), "[Scene description]");
    assert.equal(value("seen"), "[Live description]");
    assert.equal(value("scene:age"), "[Scene description (its age)]");
    assert.equal(value("showing-item"), "[Showing something]");
    assert.equal(value("subtitles"), "[Subtitles]");
    assert.equal(value("subtitles:es"), "[Subtitles: es]");
  });

  test("subtitles show the live lines with the words still being said, and each target by code or place", () => {
    const live = {
      subtitles: {
        lines: ["Good morning."],
        live: "Let's begin",
        translations: { es: ["Buenos días."], "pt-BR": [] },
        status: { targets: ["es", "pt-BR"] },
      },
    };
    const value = P.previewValues(draft, live);
    assert.equal(value("subtitles"), "Good morning.\nLet's begin");
    assert.equal(value("subtitles:es"), "Buenos días.");
    assert.equal(value("subtitles:ES"), "Buenos días.");
    assert.equal(value("subtitles:1"), "Buenos días.");
    assert.equal(value("subtitles:2"), "[Subtitles: 2]", "a target without lines yet has a stand-in");
    assert.equal(value("subtitles:fr"), "[Subtitles: fr]");
  });

  test("on air shows the live scene, with LIVE while OBS is live, and leaves the time for the renderer", () => {
    const onAir = (on_air) => P.previewValues(draft, { on_air })("on-air");
    assert.equal(onAir({ scene: "Scene 01", since_ms: null }), "Scene 01");
    assert.equal(onAir({ scene: "Scene 01", since_ms: 1200 }), "Scene 01 · LIVE");
    assert.equal(onAir({ scene: "", since_ms: 0 }), "LIVE");
    assert.equal(onAir({ scene: "", since_ms: null }), "[Scene · LIVE]");
    assert.equal(P.previewValues(draft, null)("on-air"), "[Scene · LIVE]");
    assert.equal(P.previewValues(draft, null)("on-air-time"), "{{on-air-time}}");
    assert.ok(P.shows("On air {{ on-air-time }}", "on-air-time") && !P.shows("{{on-air}}", "on-air-time"));
  });
});

describe("helpers for the text field", () => {
  test("unknownPlaceholders names everything but the live description, the scene, the room, and the conditions", () => {
    assert.deepEqual(P.unknownPlaceholders("{{scene}} {{room}} {{showing-item}} {{showing-itme}} {{x}}", draft), [
      "showing-itme",
      "x",
    ]);
    assert.deepEqual(P.unknownPlaceholders("{{scene}} {{on-air}} {{on-air-time}} {{on-air-times}}", null), [
      "on-air-times",
    ]);
    const live = "{{seen}}\n{{scene:age}} {{ scene: age }} {{scene:agee}} {{seen-it}}";
    assert.deepEqual(P.unknownPlaceholders(live, null), ["scene:agee", "seen-it"]);
    assert.deepEqual(P.unknownPlaceholders("{{subtitles}} {{subtitles:es}} {{subtitles:1}} {{subtitle}}", null), [
      "subtitle",
    ]);
  });

  test("availablePlaceholders lists the live description, scenes, room, subtitles, on air, then conditions", () => {
    assert.deepEqual(P.availablePlaceholders(draft), [
      {
        token: "{{seen}}",
        name: "Live description",
        meaning: "what the camera sees now, from on-device analysis: free, and nothing is sent",
      },
      { token: "{{scene}}", name: "Scene description", meaning: "what the AI last saw, in one sentence" },
      {
        token: "{{scene:age}}",
        name: "Scene description with its age",
        meaning: "what the AI last saw and how long ago; each look costs money",
      },
      { token: "{{room}}", name: "Room votes", meaning: "who voted in the shared room, then the revealed cards" },
      { token: "{{subtitles}}", name: "Subtitles", meaning: "what you say, live, in your language" },
      {
        token: "{{subtitles:1}}",
        name: "Translated subtitles",
        meaning: "what you say, in the first language subtitles translate into",
      },
      { token: "{{on-air}}", name: "On air", meaning: "OBS's scene, then LIVE while it streams or records" },
      {
        token: "{{on-air-time}}",
        name: "Time on air",
        meaning: "how long OBS has streamed or recorded, counting every second",
      },
      { token: "{{showing-item}}", name: "Showing something", meaning: "what it reports: the object's name" },
      { token: "{{pet-in-view}}", name: "Pet on camera", meaning: "what it reports while it is on" },
    ]);
  });

  test("insertToken replaces the selection and puts the caret after the token", () => {
    assert.deepEqual(P.insertToken("Now showing: ", 13, 13, "{{x}}"), { text: "Now showing: {{x}}", caret: 18 });
    assert.deepEqual(P.insertToken("Hello world", 6, 11, "{{x}}"), { text: "Hello {{x}}", caret: 11 });
    assert.deepEqual(P.insertToken("abc", undefined, undefined, "{{x}}"), { text: "abc{{x}}", caret: 8 });
    assert.deepEqual(P.insertToken("abc", 99, -5, "!"), { text: "abc!", caret: 4 });
  });

  test("renamePlaceholder rewrites one name and leaves text without it alone", () => {
    assert.equal(P.renamePlaceholder("{{a}} {{ b }} {{a}}", "a", "c"), "{{c}} {{b}} {{c}}");
    assert.equal(P.renamePlaceholder("{{ b }}", "a", "c"), "{{ b }}");
  });
});
