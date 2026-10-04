// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron editor: the words of the Cards tab's Room section: the room's status, who is in it and whether they
   voted, this Sceneuron's own hidden vote, the revealed cards, and checks of the name and code typed. */
(function () {
  "use strict";

  const inNode = typeof module !== "undefined" && Boolean(module.exports);
  const { isControl } = inNode ? require("./values.js") : window.SceneuronEditor.values;

  /** Longest display name, as the room allows. */
  const MAX_ROOM_NAME_CHARS = 40;

  /** "Room K7M2QX, round 2: 1 of 3 voted.", "Room K7M2QX, round 2: the votes are revealed.", or "Not in a room." */
  function roomStatus(state) {
    if (!state || !state.joined) {
      return "Not in a room.";
    }
    const people = state.participants || [];
    const at = `Room ${state.code}, round ${state.round}`;
    if (state.revealed) {
      return `${at}: the votes are revealed.`;
    }
    return `${at}: ${people.filter((person) => person.voted).length} of ${people.length} voted.`;
  }

  /**
   * One line per person: before the reveal, whether they voted ("Ana (you): voted", "Ben: not yet"); after it,
   * what they voted ("Ben: 8", "Cleo: no vote"). A result belongs to the person with its id, since two people
   * may share a name.
   */
  function personLine(person, state) {
    const who = person.me ? `${person.name} (you)` : person.name;
    if (!state.revealed) {
      return `${who}: ${person.voted ? "voted" : "not yet"}`;
    }
    const shown = (state.results || []).find((result) => result.id === person.id);
    return `${who}: ${shown ? shown.label : "no vote"}`;
  }

  /** What this Sceneuron voted, which only its own editor says, or how to vote. */
  function myVoteText(state, deckName) {
    if (!state || !state.joined || state.revealed) {
      return "";
    }
    if (state.my_vote) {
      return `Your hidden vote: ${state.my_vote.label}. Raise another card to change it.`;
    }
    return `Raise a card of ${deckName || state.deck} to vote. Your overlay shows it covered until the reveal.`;
  }

  /** The revealed cards in order, such as "3, 5, 8", or "Nobody voted." */
  function spreadText(state) {
    if (!state || !state.joined || !state.revealed) {
      return "";
    }
    return state.summary === "No votes" ? "Nobody voted." : `Revealed: ${state.summary}.`;
  }

  /** A room code as typed, trimmed and in capitals. */
  function normalizeCode(text) {
    return String(text).trim().toUpperCase();
  }

  function isRoomCode(text) {
    return /^[A-Z0-9]{4,12}$/.test(normalizeCode(text));
  }

  /** 1 to 40 characters once trimmed, without control characters, as the room allows. */
  function isDisplayName(text) {
    const chars = Array.from(String(text).trim());
    return chars.length >= 1 && chars.length <= MAX_ROOM_NAME_CHARS && !chars.some(isControl);
  }

  const roomText = {
    MAX_ROOM_NAME_CHARS,
    isDisplayName,
    isRoomCode,
    myVoteText,
    normalizeCode,
    personLine,
    roomStatus,
    spreadText,
  };
  if (inNode) {
    module.exports = roomText;
  } else {
    window.SceneuronEditor.roomText = roomText;
  }
})();
