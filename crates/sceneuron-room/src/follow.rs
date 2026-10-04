// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! How the client follows the room and the overlay: votes for cards raised in the bound deck, and
//! the room's reveals, rounds, and endings on the overlay.

use std::sync::Arc;

use sceneuron_core::Outcome;

use crate::backend::{BackendEvent, Call, CallError};
use crate::client::RoomClient;
use crate::local::{self, Local};
use crate::summary::summary;

impl RoomClient {
    /// After every change of the overlay while a round is open: keeps the bound deck covered, and
    /// votes for a newly raised card.
    pub(crate) fn follow_deck(&self) {
        let Some((generation, deck)) = self.open_round(None) else {
            return;
        };
        let mut raised = None;
        self.hub.update(|engine, now| {
            // Checked again under the overlay's lock: a reveal marks the round revealed before it
            // uncovers the deck, so a cover decided before a reveal can never land after it.
            if self.open_round(Some(generation)).is_none() {
                return Outcome::default();
            }
            raised = engine.raised_card(&deck).map(str::to_owned);
            if engine.cover(&deck).is_some() {
                return Outcome::default();
            }
            local::apply(engine, &deck, &[Local::Cover], None, now)
        });
        let Some(card) = raised else {
            return;
        };
        let backend = {
            let mut state = self.lock();
            let Some(session) = state
                .session
                .as_mut()
                .filter(|session| session.generation == generation && !session.syncing)
            else {
                return;
            };
            if session.voted.as_deref() == Some(card.as_str()) {
                return;
            }
            session.voted = Some(card.clone());
            Arc::clone(&session.backend)
        };
        let this = self.this.clone();
        let vote = Call::Vote { card: card.clone() };
        backend.send(
            vote,
            Box::new(move |result| {
                if let (Err(error), Some(client)) = (result, this.upgrade()) {
                    client.vote_failed(generation, &card, error);
                }
            }),
        );
    }

    /// The generation and bound deck of the session while its round is open and the overlay
    /// follows the room; only of session `generation` when one is given.
    fn open_round(&self, generation: Option<u64>) -> Option<(u64, String)> {
        let state = self.lock();
        state
            .session
            .as_ref()
            .filter(|session| generation.is_none_or(|wanted| wanted == session.generation))
            .filter(|session| !session.syncing && session.view.as_ref().is_some_and(|view| !view.revealed))
            .map(|session| (session.generation, session.deck.clone()))
    }

    /// Reports a vote the room did not count. The card stays the one tried, so the same card is not
    /// sent again and again; raising another card votes again.
    fn vote_failed(&self, generation: u64, card: &str, error: CallError) {
        let reason = match error {
            CallError::Refused(reason) | CallError::Failed(reason) => reason,
        };
        let message = format!("the vote for {card} was not counted: {reason}");
        {
            let mut state = self.lock();
            if state
                .session
                .as_ref()
                .is_none_or(|session| session.generation != generation)
            {
                return;
            }
            state.error = Some(message.clone());
        }
        self.hub.update(|engine, now| engine.note("room", &message, now));
    }

    /// What the connection of session `generation` reports. The overlay follows the room while the
    /// session is marked as syncing, so no vote is cast for a card of the round that just ended.
    pub(crate) fn on_event(&self, generation: u64, event: BackendEvent) {
        let (deck, changes, view, ended) = {
            let mut state = self.lock();
            let Some(session) = state
                .session
                .as_mut()
                .filter(|session| session.generation == generation)
            else {
                return;
            };
            let deck = session.deck.clone();
            match event {
                BackendEvent::Room(Some(view)) => {
                    let changes = local::changes(session.view.as_ref(), &view);
                    if session.view.as_ref().is_none_or(|old| old.round != view.round) {
                        session.voted = None;
                    }
                    session.view = Some(view.clone());
                    session.syncing = true;
                    (deck, changes, Some(view), None)
                }
                BackendEvent::Room(None) if session.view.is_none() => return,
                BackendEvent::Room(None) => {
                    let ended = state.take_session();
                    state.error = Some("this Sceneuron is no longer in the room".to_owned());
                    (deck, vec![Local::Leave], None, ended)
                }
                BackendEvent::Disconnected(reason) => {
                    let ended = state.take_session();
                    state.error = Some(match reason {
                        Some(reason) => format!("lost the connection to the room: {reason}"),
                        None => "the room's server closed the connection".to_owned(),
                    });
                    (deck, vec![Local::Leave], None, ended)
                }
            }
        };
        self.changed.notify_all();
        if let Some(ended) = ended {
            ended.backend.close();
        }
        self.hub.update(|engine, now| {
            let text = view
                .as_ref()
                .map_or_else(String::new, |view| summary(view, engine.project().deck(&deck)));
            local::apply(engine, &deck, &changes, Some(&text), now)
        });
        if view.is_some() {
            if let Some(session) = self
                .lock()
                .session
                .as_mut()
                .filter(|session| session.generation == generation)
            {
                session.syncing = false;
            }
            // A card raised while the overlay caught up votes now.
            self.follow_deck();
        }
    }
}
