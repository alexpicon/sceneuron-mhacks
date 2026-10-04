// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the page's last script. It creates the editor object, sets up every panel, wires the
   page-wide save and discard controls, and connects to the server. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { isDirty } = Editor.project;

  function start() {
    // The editor's shared controls include the Camera & mic tab's, so its markup is built first.
    Editor.cameraFields.buildCameraPanel(document.getElementById("panel-camera"));
    const editor = Editor.createEditor();
    const { app, ui, views } = editor;
    Editor.aiFields.buildAiPanel(editor.byId("panel-ai"));
    editor.byId("ai-provider-details").before(Editor.voiceFields.buildVoiceSection());
    editor.byId("ai-provider-details").before(Editor.subtitlesFields.buildSubtitlesSection());
    Object.assign(editor, Editor.notifications(editor));
    views.status = Editor.status(editor);
    views.aiStatus = Editor.aiStatus(editor);
    views.events = Editor.events(editor);
    views.elements = Editor.elements(editor);
    views.preview = Editor.preview(editor);
    views.properties = Editor.properties(editor);
    views.placeholderChips = Editor.placeholderChips(editor);
    views.live = Editor.live(editor);
    views.rules = Editor.rules(editor);
    views.conditions = Editor.conditions(editor);
    views.settings = Editor.settings(editor);
    views.settingsTransfer = Editor.settingsTransfer(editor);
    views.theme = Editor.theme(editor);
    views.reactions = Editor.reactions(editor);
    views.aiSettings = Editor.aiSettings(editor);
    views.voice = Editor.voice(editor);
    views.voiceGuide = Editor.voiceGuide(editor);
    views.subtitles = Editor.subtitles(editor);
    views.assistant = Editor.assistant(editor);
    views.setupWizard = Editor.setupWizard(editor);
    views.camera = Editor.camera(editor);
    views.cameraPreview = Editor.cameraPreview(editor);
    views.microphones = Editor.microphones(editor);
    views.cards = Editor.cards(editor);
    views.packs = Editor.packs(editor);
    views.room = Editor.room(editor);
    views.streamDeck = Editor.streamDeck(editor);
    views.tabs = Editor.tabs(editor);
    Object.assign(editor, Editor.sync(editor));

    ui.save.addEventListener("click", editor.save);
    ui.discard.addEventListener("click", editor.discard);
    document.addEventListener("keydown", (event) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        editor.save();
      }
    });
    window.addEventListener("beforeunload", (event) => {
      if (app.draft && isDirty(app.saved, app.draft)) {
        event.preventDefault();
        event.returnValue = "";
      }
    });

    editor.refreshAll();
    if (!editor.token) {
      editor.showBanner("This editor link has no access token. Open the editor from the link Sceneuron shows in OBS.");
      views.status.setConnection("offline");
      return;
    }
    views.status.setConnection("connecting");
    editor.connectEvents(0);
    editor.loadProject();
    views.aiSettings.load();
    views.microphones.load();
  }

  start();
})();
