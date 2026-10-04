// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the markup of the Cards tab's Room section: the form that creates or joins a room, the room
   with its people, this Sceneuron's own vote, and its actions, and the line that reports what went wrong. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const { MAX_ROOM_NAME_CHARS } = Editor.roomText;
  const { field, h } = Editor.dom;

  function input(id, attrs) {
    return h("input", Object.assign({ id, type: "text", autocomplete: "off", spellcheck: "false" }, attrs));
  }

  /** The section's controls, which room.js wires. */
  Editor.roomMarkup = function roomMarkup() {
    const button = (id, label, attrs) => h("button", Object.assign({ id, type: "button" }, attrs), label);
    return [
      h("h3", { id: "room-heading", tabindex: "-1" }, "Room"),
      h(
        "p",
        { class: "hint" },
        "Vote together on a call. Everyone runs Sceneuron and joins the same room. While you are in it, raising ",
        "a card of the room's deck casts a hidden vote: your overlay shows the card covered, and everyone sees who ",
        "voted. Reveal shows everyone's own card at the same moment, and Next round lowers them. Add {{room}} to a ",
        "text element to show who voted, then the revealed cards.",
      ),
      h(
        "div",
        { id: "room-setup", class: "room-setup" },
        field("room-name", "Your name", input("room-name", { maxlength: MAX_ROOM_NAME_CHARS })),
        field("room-deck", "Deck that votes", h("select", { id: "room-deck" })),
        h(
          "div",
          { class: "button-row" },
          button("room-create", "Create a room"),
          button("room-practice-open", "Start practice meeting"),
        ),
        h(
          "p",
          { class: "hint" },
          "Start practice meeting joins room CHASKI, creating it when nobody has it, and brings in four simulated ",
          "teammates who vote in each round, so you can try or show the room alone.",
        ),
        h(
          "div",
          { class: "field" },
          h("label", { for: "room-code" }, "Room code"),
          h(
            "div",
            { class: "inline-field" },
            input("room-code", { maxlength: 12, autocapitalize: "characters", class: "room-code", placeholder: "CHASKI" }),
            button("room-join", "Join"),
          ),
        ),
        h(
          "details",
          { class: "room-server" },
          h("summary", {}, "SpacetimeDB server"),
          field("room-server", "Server address", input("room-server", { inputmode: "url" })),
          field("room-database", "Database", input("room-database")),
        ),
      ),
      h(
        "div",
        { id: "room-joined", class: "room-joined", hidden: true },
        h("p", { id: "room-status", class: "deck-status", tabindex: "-1", "aria-live": "polite" }),
        h("ul", { id: "room-people", class: "room-people", "aria-label": "People in the room" }),
        h("p", { id: "room-spread", class: "room-spread", hidden: true }),
        h("p", { id: "room-vote", class: "hint" }),
        h(
          "div",
          { class: "button-row" },
          button("room-reveal", "Reveal votes"),
          button("room-next", "Next round"),
          button("room-practice", "Add practice teammates"),
          button("room-leave", "Leave room", { class: "danger" }),
        ),
      ),
      h("p", { id: "room-error", class: "field-error", role: "alert", hidden: true }),
    ];
  };
})();
