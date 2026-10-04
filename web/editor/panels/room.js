// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the Cards tab's Room section: creating or joining a shared planning poker room on a
   SpacetimeDB server, who is in it and who voted, revealing the votes, and the next round. While this Sceneuron is
   in a room, raising a card of the chosen deck from anywhere casts a hidden vote; the server covers it on the
   overlay, so this section only shows the room and takes its actions. */
(function () {
  "use strict";

  const Editor = window.SceneuronEditor;
  const {
    MAX_ROOM_NAME_CHARS,
    isDisplayName,
    isRoomCode,
    myVoteText,
    normalizeCode,
    personLine,
    roomStatus,
    spreadText,
  } = Editor.roomText;
  const { clearInvalid, h, option, setInvalid } = Editor.dom;

  /** How often the section asks for the room while this Sceneuron is in one. */
  const POLL_MS = 1000;

  function setText(node, text) {
    if (node.textContent !== text) {
      node.textContent = text;
    }
  }

  /** Builds and wires the Room section; returns the view with load, which asks for the room, and renderDecks. */
  /** The room the practice meeting uses, created by whoever opens it first. */
  const PRACTICE_CODE = "CHASKI";

  Editor.room = function setupRoom(editor) {
    const { app, api, byId } = editor;
    byId("room-section").replaceChildren(...Editor.roomMarkup());
    const fields = {
      name: byId("room-name"),
      deck: byId("room-deck"),
      code: byId("room-code"),
      server: byId("room-server"),
      database: byId("room-database"),
    };
    const buttons = ["room-create", "room-practice-open", "room-join", "room-reveal", "room-next", "room-practice"]
      .concat("room-leave")
      .map(byId);
    let state = null;
    let shownKey = "";
    let decksKey = "";
    let filled = false;
    let busy = false;
    let timer = null;

    const savedDecks = () => (app.saved && app.saved.decks) || [];

    /** Offers the saved decks, keeping the one chosen; the room needs a deck that is saved. */
    function renderDecks() {
      const decks = savedDecks();
      const key = JSON.stringify(decks.map((deck) => [deck.id, deck.name]));
      if (key !== decksKey) {
        decksKey = key;
        const chosen = fields.deck.value || (state && state.deck) || "poker";
        fields.deck.replaceChildren(...decks.map((deck) => option(deck.id, deck.name)));
        if (decks.some((deck) => deck.id === chosen)) {
          fields.deck.value = chosen;
        }
        fields.deck.disabled = decks.length === 0;
      }
      renderButtons();
    }

    function renderButtons() {
      const noDeck = savedDecks().length === 0;
      for (const button of buttons) {
        const enters = ["room-create", "room-practice-open", "room-join"].includes(button.id);
        button.disabled = busy || (noDeck && enters);
      }
      byId("room-section").setAttribute("aria-busy", String(busy));
    }

    function render(next) {
      state = next;
      const key = JSON.stringify(next);
      if (key === shownKey) {
        return;
      }
      shownKey = key;
      const available = Boolean(next) && next.available !== false;
      if (!filled && available) {
        // The form starts from what this Sceneuron used last, or the defaults.
        filled = true;
        fields.name.value = next.name || "";
        fields.server.value = next.server || "";
        fields.database.value = next.database || "";
        if (next.deck) {
          fields.deck.value = next.deck;
        }
      }
      const joined = Boolean(next && next.joined);
      // A poll can hide the control that has focus, when someone else reveals or the room ends; focus then moves
      // to what is left instead of falling to the page.
      const focused = document.activeElement;
      const leftRoom = !joined && byId("room-joined").contains(focused);
      byId("room-setup").hidden = joined || !available;
      byId("room-joined").hidden = !joined;
      const problem = available ? next.error : next && "Shared rooms are not available in this build.";
      const error = byId("room-error");
      error.hidden = !problem;
      setText(error, problem || "");
      if (joined) {
        const deck = savedDecks().find((candidate) => candidate.id === next.deck);
        setText(byId("room-status"), roomStatus(next));
        const people = (next.participants || []).map((person) =>
          h("li", { "data-voted": String(person.voted) }, personLine(person, next))
        );
        byId("room-people").replaceChildren(...people);
        const spread = byId("room-spread");
        setText(spread, spreadText(next));
        spread.hidden = !next.revealed;
        setText(byId("room-vote"), myVoteText(next, deck && deck.name));
        byId("room-reveal").hidden = next.revealed;
        setText(byId("room-practice"), next.practice ? "Remove practice teammates" : "Add practice teammates");
        if (next.revealed && focused === byId("room-reveal")) {
          byId("room-status").focus();
        }
      }
      if (leftRoom) {
        byId("room-heading").focus();
      }
      schedule(joined);
    }

    /** Asks for the room every second while in one; the overlay does not need it, only this list. */
    function schedule(joined) {
      if (joined && !timer) {
        timer = window.setInterval(poll, POLL_MS);
      } else if (!joined && timer) {
        window.clearInterval(timer);
        timer = null;
      }
    }

    async function poll() {
      if (busy) {
        return;
      }
      try {
        render(await api.room());
      } catch {
        // The next poll tries again; the connection chip already says when the server is gone.
      }
    }

    async function load() {
      await poll();
    }

    /** Runs a room action, then shows the room it leaves and announces `done(state)`. */
    async function act(action, body, done) {
      busy = true;
      renderButtons();
      try {
        const next = await api.roomAction(action, body);
        render(next);
        editor.announce(done(next));
        return true;
      } catch (error) {
        editor.report(error);
        await poll();
        return false;
      } finally {
        busy = false;
        renderButtons();
      }
    }

    /** The fields every create or join sends; null after pointing at the first refused one. */
    function entered(withCode) {
      for (const input of Object.values(fields)) {
        clearInvalid(input);
      }
      if (!isDisplayName(fields.name.value)) {
        setInvalid(fields.name, `Enter 1 to ${MAX_ROOM_NAME_CHARS} characters, the name others in the room see.`);
        fields.name.focus();
        return null;
      }
      if (withCode && !isRoomCode(fields.code.value)) {
        setInvalid(fields.code, "Enter the room's code: 4 to 12 letters and digits, such as K7M2QX or CHASKI.");
        fields.code.focus();
        return null;
      }
      const body = {
        name: fields.name.value.trim(),
        deck: fields.deck.value,
        server: fields.server.value.trim(),
        database: fields.database.value.trim(),
      };
      return withCode ? Object.assign(body, { code: normalizeCode(fields.code.value) }) : body;
    }

    async function enter(action) {
      const body = entered(action === "join");
      if (!body) {
        return;
      }
      const joined = await act(action, body, (next) =>
        action === "create"
          ? `Created room ${next.code}. Share the code with your call.`
          : `Joined room ${next.code} with ${next.participants.length} people.`
      );
      if (joined) {
        byId("room-status").focus();
      }
    }

    /** Joins room CHASKI, creating it when nobody has it, and brings in the practice teammates. */
    async function practiceMeeting() {
      if (!fields.name.value.trim()) {
        fields.name.value = "Host";
      }
      const body = entered(false);
      if (!body) {
        return;
      }
      const opened = await act("open", Object.assign(body, { code: PRACTICE_CODE }), (next) =>
        `Joined room ${next.code}. Bringing in the practice teammates.`
      );
      if (opened && (await act("practice", {}, () => "Maya, Sam, Priya, and Leo joined; they vote a few seconds in."))) {
        byId("room-status").focus();
      }
    }

    byId("room-create").addEventListener("click", () => enter("create"));
    byId("room-practice-open").addEventListener("click", practiceMeeting);
    byId("room-practice").addEventListener("click", () =>
      state && state.practice
        ? act("practice-stop", {}, () => "The practice teammates left the room.")
        : act("practice", {}, () => "Maya, Sam, Priya, and Leo joined; they vote a few seconds in.")
    );
    byId("room-join").addEventListener("click", () => enter("join"));
    fields.code.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        enter("join");
      }
    });
    byId("room-reveal").addEventListener("click", async () => {
      if (await act("reveal", {}, (next) => `${roomStatus(next)} ${spreadText(next)}`)) {
        byId("room-status").focus();
      }
    });
    byId("room-next").addEventListener("click", () => act("next", {}, roomStatus));
    byId("room-leave").addEventListener("click", async () => {
      if (await act("leave", {}, () => "Left the room. Cards show as usual again.")) {
        byId("room-heading").focus();
      }
    });

    renderDecks();
    return { load, renderDecks };
  };
})();
