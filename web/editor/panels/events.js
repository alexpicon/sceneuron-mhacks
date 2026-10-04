// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the recent events log. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { h } = Editor.dom;
  const { eventWallTime, formatClock } = Editor.snapshot;

  /**
   * The detail of an event. The AI's own changes ("Waving turned on: a hand is raised") and the rules they
   * start ('ran "Say hi" because Waving turned on') keep their text, with the reason set apart.
   */
  function detailNode(event) {
    const detail = String(event.detail);
    const separator = { ai: ": ", rule: " because " }[event.source];
    const at = separator ? detail.indexOf(separator) : -1;
    if (at < 0) {
      return h("span", { class: "event-detail" }, detail);
    }
    return h(
      "span",
      { class: "event-detail" },
      h("span", { class: "event-what" }, detail.slice(0, at)),
      h("span", { class: "event-why" }, detail.slice(at)),
    );
  }

  /** Returns the events view, whose render redraws the log only when its events change. */
  Editor.events = function setupEvents(editor) {
    const { app, ui } = editor;

    function render() {
      const live = app.live;
      const events = live && Array.isArray(live.events) ? live.events : [];
      const key = events.length ? JSON.stringify([events.length, events[0], events[events.length - 1].at_ms]) : "";
      if (key === app.eventsKey) {
        return;
      }
      app.eventsKey = key;
      const received = Date.now();
      const items = events.map((event) =>
        h(
          "li",
          { "data-source": String(event.source) },
          h("span", { class: "event-time" }, formatClock(eventWallTime(event.at_ms, live.now_ms, received))),
          h("span", { class: "event-source" }, String(event.source)),
          detailNode(event),
        )
      );
      ui.eventLog.replaceChildren(...items);
      ui.eventEmpty.hidden = events.length > 0;
    }

    return { render };
  };
})();
