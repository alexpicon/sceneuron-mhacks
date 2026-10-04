// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

// Exercises editor/sync.js against a fake server: saves send the project revision the draft is based on,
// a save refused because the project changed elsewhere keeps the draft and offers a reload, a clean draft
// catches up quietly when another editor saves, and a draft with unsaved edits is warned once.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, test } from "node:test";
import vm from "node:vm";

import { project } from "./editor-fixtures.mjs";

const web = new URL("../", import.meta.url);
const SCRIPTS = [
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
  "editor/constants.js",
  "editor/values.js",
  "editor/project.js",
  "editor/triggers.js",
  "editor/placeholders.js",
  "editor/snapshot.js",
  "editor/api.js",
  "editor/sync.js",
];

/** The editor namespace as the page builds it, in a fresh global. */
function loadEditor() {
  const context = vm.createContext({ document: { querySelector: () => null } });
  context.window = context;
  for (const script of SCRIPTS) {
    vm.runInContext(readFileSync(new URL(script, web), "utf8"), context, { filename: script });
  }
  return context.SceneuronEditor;
}

const clone = (value) => JSON.parse(JSON.stringify(value));
const settle = () => new Promise((resolve) => setImmediate(resolve));

/** An editor wired to a server whose project is at `revision`; the draft was loaded at revision 3. */
function setup(revision = 3) {
  const Editor = loadEditor();
  const server = { revision, project: project(), saves: [] };
  const snapshot = () => ({ revision: 100 + server.revision, elements: [], project_revision: server.revision });
  const api = {
    getState: async () => snapshot(),
    getProject: async () => clone(server.project),
    saveProject: async (body, ifMatch) => {
      server.saves.push(ifMatch);
      if (Number.isFinite(ifMatch) && ifMatch !== server.revision) {
        const data = { error: "the project changed", project_revision: server.revision };
        throw new Editor.api.ApiError(data.error, 409, data);
      }
      server.revision += 1;
      server.project = clone(body);
      return snapshot();
    },
  };
  const seen = { banners: [], toasts: [], refreshes: 0 };
  const button = { textContent: "Save", setAttribute() {}, removeAttribute() {}, focus() {} };
  const app = { saved: project(), draft: project(), live: null, baseRevision: 3, selectedId: "brb" };
  const noop = { render() {}, setConnection() {}, reveal() {}, renderDecks() {} };
  const editor = {
    app,
    api,
    token: "token",
    ui: { banner: { hidden: true }, save: button },
    views: { status: noop, elements: noop, live: noop, tabs: noop, room: noop },
    toast: (message, kind) => seen.toasts.push([message, kind]),
    report: (error) => seen.toasts.push([error.message, "error"]),
    announce() {},
    renderDirty() {},
    refreshAll: () => (seen.refreshes += 1),
    applySnapshot: (value) => (app.live = value),
    showBanner(message, action, label) {
      editor.ui.banner.hidden = false;
      seen.banners.push({ message, action, label });
    },
  };
  Object.assign(editor, Editor.sync(editor));
  return { app, editor, server, seen };
}

describe("saving", () => {
  test("a save sends the revision the draft is based on and moves to the new one", async () => {
    const { app, editor, server, seen } = setup();
    app.draft.elements[0].props.text = "Away for a bit";
    await editor.save();
    assert.deepEqual(server.saves, [3]);
    assert.equal(app.baseRevision, 4);
    assert.equal(app.saved.elements[0].props.text, "Away for a bit");
    assert.deepEqual(seen.toasts, [["Saved.", "info"]]);
  });

  test("a save refused because the project changed keeps the draft and offers a reload", async () => {
    const { app, editor, server, seen } = setup(4);
    server.project.elements[1].name = "Changed elsewhere";
    app.draft.elements[0].props.text = "My edit";
    await editor.save();
    assert.deepEqual(server.saves, [3]);
    assert.equal(seen.toasts.length, 0, "a conflict is a banner, not an error toast");
    const [banner] = seen.banners;
    assert.equal(banner.label, "Reload");
    assert.match(banner.message, /^Not saved: the project was changed in another editor or by the assistant/);
    assert.equal(app.draft.elements[0].props.text, "My edit", "the draft is kept until the streamer reloads");
    assert.equal(app.saved.elements[1].name, "Camera frame");
    assert.equal(app.conflict, true);

    await banner.action();
    assert.equal(app.draft.elements[1].name, "Changed elsewhere");
    assert.equal(app.draft.elements[0].props.text, "BRB");
    assert.equal(app.baseRevision, 4);
    assert.equal(app.conflict, false);
    assert.equal(editor.ui.banner.hidden, true);
  });

  test("adopting a plan's project takes the revision its save reports", () => {
    const { app, editor, seen } = setup();
    const next = project();
    next.elements.push({ id: "hype", name: "Hype", kind: "text", props: {} });
    app.selectedId = "hype";
    editor.adopt(next, { revision: 200, elements: [], project_revision: 9 });
    assert.equal(app.baseRevision, 9);
    assert.equal(app.selectedId, "hype");
    assert.equal(JSON.stringify(app.saved), JSON.stringify(next), "objects from the page's realm compare by value");
    assert.equal(seen.refreshes, 1);
  });
});

describe("saves made elsewhere", () => {
  test("a clean draft quietly catches up", async () => {
    const { app, editor, server, seen } = setup(4);
    server.project.elements[1].name = "Renamed in another editor";
    editor.watchRevision({ project_revision: 4 });
    await settle();
    await settle();
    assert.equal(app.draft.elements[1].name, "Renamed in another editor");
    assert.equal(app.baseRevision, 4);
    assert.equal(seen.banners.length, 0);
  });

  test("a draft with unsaved edits is warned once and kept", async () => {
    const { app, editor, seen } = setup(4);
    app.draft.elements[0].props.text = "Unsaved";
    editor.watchRevision({ project_revision: 4 });
    editor.watchRevision({ project_revision: 4 });
    await settle();
    assert.equal(seen.banners.length, 1);
    assert.match(seen.banners[0].message, /your unsaved edits here are based on the older version/);
    assert.equal(app.draft.elements[0].props.text, "Unsaved");
  });

  test("discarding during a conflict takes the latest project, not the outdated saved copy", async () => {
    const { app, editor, server } = setup(4);
    server.project.elements[1].name = "Renamed elsewhere";
    app.draft.elements[0].props.text = "Unsaved";
    editor.watchRevision({ project_revision: 4 });
    await editor.discard();
    assert.equal(app.draft.elements[1].name, "Renamed elsewhere");
    assert.equal(app.draft.elements[0].props.text, "BRB");
    assert.equal(app.baseRevision, 4);
    assert.equal(app.conflict, false);
    assert.equal(editor.ui.banner.hidden, true);
  });

  test("the editor's own save in flight and older revisions are not conflicts", async () => {
    const { app, editor, seen } = setup(4);
    app.draft.elements[0].props.text = "Unsaved";
    app.saving = true;
    editor.watchRevision({ project_revision: 4 });
    app.saving = false;
    editor.watchRevision({ project_revision: 3 });
    editor.watchRevision({ project_revision: 2 });
    await settle();
    assert.equal(seen.banners.length, 0);
  });
});
