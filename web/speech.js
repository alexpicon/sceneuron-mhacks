// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

/* Sceneuron overlay: spoken cards. Each speech cue in the snapshot plays once, after the server attached its
   audio. A cue already spoken when the page connected, or one whose audio came too late, stays silent, and a
   card that is lowered or covered stops speaking. A server that restarted counts its cues from 1 again, so a cue
   older than the last one starts the count afresh, as a first snapshot does. A reconnect to the same server keeps
   the count, so a cue that became ready while the page was away still plays. Inside OBS the page's audio goes to
   OBS's mix. */
(function () {
  "use strict";

  /** A cue older than this when its audio arrives is no longer worth saying. */
  const MAX_CUE_AGE_MS = 15000;
  const AUDIO_ID = /^[0-9a-f]{64}$/;

  function isCue(cue) {
    return Boolean(cue) && typeof cue === "object" && Number.isFinite(cue.nonce);
  }

  function isReady(cue) {
    return isCue(cue) && typeof cue.audio === "string" && AUDIO_ID.test(cue.audio);
  }

  /**
   * Decides what a snapshot means for speech: `{ state, play }`, where `play` is the cue to play now or null.
   * `state.last` is the nonce of the last cue played or passed over, or null before the first snapshot; a cue
   * ready in the first snapshot was spoken before this page connected.
   */
  function nextCue(state, snapshot) {
    const cue = snapshot && snapshot.speech;
    if (state.last === null || (isCue(cue) && cue.nonce < state.last)) {
      const last = !isCue(cue) ? 0 : isReady(cue) ? cue.nonce : cue.nonce - 1;
      return { state: { last }, play: null };
    }
    if (!isReady(cue) || cue.nonce <= state.last) {
      return { state, play: null };
    }
    const fresh = !Number.isFinite(snapshot.now_ms) || !Number.isFinite(cue.at_ms) ||
      snapshot.now_ms - cue.at_ms <= MAX_CUE_AGE_MS;
    return { state: { last: cue.nonce }, play: fresh ? cue : null };
  }

  /** Whether a cue being played should stop: its card is no longer the one its deck shows, or it is covered. */
  function shouldStop(cue, snapshot) {
    const decks = (snapshot && Array.isArray(snapshot.decks) && snapshot.decks) || [];
    const deck = decks.find((view) => view && view.id === cue.deck);
    return !deck || deck.raised !== cue.card || Boolean(deck.covered);
  }

  /** The address of a cue's audio; an audio element cannot send headers, so the token goes in the query. */
  function speechUrl(audio, token) {
    return `/api/speech/${encodeURIComponent(audio)}?token=${encodeURIComponent(token)}`;
  }

  /**
   * A function that takes each snapshot and plays its new cues on one audio element from `makeAudio()`, such as
   * `new Audio()`, made when the first cue plays. One element keeps one media player, so cues never wait for
   * another player to let go.
   */
  function createPlayer(token, makeAudio) {
    let state = { last: null };
    let audio = null;
    let playing = null;
    function consider(snapshot) {
      if (playing && shouldStop(playing, snapshot)) {
        audio.pause();
        playing = null;
      }
      const next = nextCue(state, snapshot);
      state = next.state;
      if (!next.play) {
        return;
      }
      if (!audio) {
        audio = makeAudio();
        audio.addEventListener("ended", () => {
          playing = null;
        });
      }
      audio.pause();
      audio.src = speechUrl(next.play.audio, token);
      playing = next.play;
      Promise.resolve(audio.play()).catch((error) => {
        console.warn(`Sceneuron overlay: cannot play a spoken card: ${error && error.message}`);
      });
    }
    return consider;
  }

  const speech = { MAX_CUE_AGE_MS, createPlayer, nextCue, shouldStop, speechUrl };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = speech;
  } else {
    window.SceneuronSpeech = speech;
  }
})();
