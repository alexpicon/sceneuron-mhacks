// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Settings tab's "Save to file" and "Load from file". Saving downloads the saved project and
   the AI, voice, and subtitle settings as one JSON file; loading replaces them with a file's and reloads the
   editor. Keys and consents stay on this computer either way. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { build, fileName, parse } = Editor.settingsFile;
  const { isDirty } = Editor.project;

  Editor.settingsTransfer = function setupSettingsTransfer(editor) {
    const { api, app, byId } = editor;
    const saveButton = byId("settings-file-save");
    const loadButton = byId("settings-file-load");
    const input = byId("settings-file-input");
    const status = byId("settings-file-status");

    function show(message) {
      status.textContent = message;
    }

    async function save() {
      saveButton.disabled = true;
      try {
        const [project, ai] = await Promise.all([api.getProject(), api.aiSettings()]);
        const now = new Date();
        const text = `${JSON.stringify(build(project, ai, now.toISOString()), null, 2)}\n`;
        const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
        const link = document.createElement("a");
        link.href = url;
        link.download = fileName(now);
        document.body.append(link);
        link.click();
        link.remove();
        URL.revokeObjectURL(url);
        const unsaved = app.draft && isDirty(app.saved, app.draft) ? " Unsaved edits are not in it; save first." : "";
        show(`Saved ${link.download}.${unsaved}`);
      } catch (error) {
        editor.report(error);
        show(`Could not save the settings: ${error.message}`);
      } finally {
        saveButton.disabled = false;
      }
    }

    async function load(file) {
      let parsed;
      try {
        parsed = parse(await file.text());
      } catch (error) {
        show(error.message);
        return;
      }
      const question =
        `Replace this project and the AI, voice, and subtitle settings with ${file.name}? ` +
        "Unsaved edits are lost. Keys and consents stay as they are.";
      if (!window.confirm(question)) {
        return;
      }
      loadButton.disabled = true;
      try {
        if (parsed.ai) {
          await api.updateAiSettings(parsed.ai);
        }
        await api.saveProject(parsed.project);
        app.draft = null;
        show(`Loaded ${file.name}. Reloading the editor.`);
        window.location.reload();
      } catch (error) {
        editor.report(error);
        show(`Could not load ${file.name}: ${error.message}`);
        loadButton.disabled = false;
      }
    }

    saveButton.addEventListener("click", save);
    loadButton.addEventListener("click", () => input.click());
    input.addEventListener("change", () => {
      const [file] = input.files;
      input.value = "";
      if (file) {
        load(file);
      }
    });
    return {};
  };
})();
