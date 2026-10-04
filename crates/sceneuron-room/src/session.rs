// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! How a session starts, waits, and ends: a new connection becomes the session once it sees the
//! rooms, where shutting down can still close it; calls wait for their answer; and joining waits
//! for the room to show this connection in it.

use std::sync::{Arc, PoisonError, mpsc};

use sceneuron_server::RoomError;

use crate::backend::{Call, CallError, RoomBackend};
use crate::client::{RoomClient, Session, close_and_wait};
use crate::target::Target;

impl RoomClient {
    /// Waits for a new connection to see the rooms, where shutting down can close it, then makes
    /// it the session.
    pub(crate) fn open(&self, session: Session, target: &Target) -> Result<(), RoomError> {
        let backend = Arc::clone(&session.backend);
        {
            let mut state = self.lock();
            if state.stopped {
                drop(state);
                close_and_wait(backend.as_ref());
                return Err(stopping());
            }
            state.connecting = Some(Arc::clone(&backend));
        }
        let ready = backend.ready(self.timeouts.connect);
        let mut state = self.lock();
        state.connecting = None;
        let refusal = match ready {
            // Shutting down closed the connection and waited for it.
            _ if state.stopped => stopping(),
            Ok(()) => {
                state.error = None;
                state.session = Some(session);
                return Ok(());
            }
            Err(reason) => {
                state.keep_closing(Arc::clone(&backend));
                unreachable(target, &reason)
            }
        };
        drop(state);
        backend.close();
        Err(refusal)
    }

    /// Sends `call` and waits for its answer.
    pub(crate) fn call(&self, backend: &dyn RoomBackend, call: Call) -> Result<(), RoomError> {
        let (answer, answered) = mpsc::channel();
        backend.send(
            call,
            Box::new(move |result| {
                let _ = answer.send(result);
            }),
        );
        match answered.recv_timeout(self.timeouts.call) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(CallError::Refused(reason))) => Err(RoomError::new(400, reason)),
            Ok(Err(CallError::Failed(reason))) => {
                Err(RoomError::new(502, format!("the room did not answer: {reason}")))
            }
            Err(_) => Err(RoomError::new(
                504,
                format!(
                    "the room did not answer in {} seconds",
                    self.timeouts.call.as_secs_f64()
                ),
            )),
        }
    }

    /// Waits until the room shows this connection in it.
    pub(crate) fn wait_for_room(&self, generation: u64) -> Result<(), RoomError> {
        let state = self.lock();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, self.timeouts.view, |state| {
                state
                    .session
                    .as_ref()
                    .is_some_and(|session| session.generation == generation && session.view.is_none())
            })
            .unwrap_or_else(PoisonError::into_inner);
        match state
            .session
            .as_ref()
            .filter(|session| session.generation == generation)
        {
            Some(session) if session.view.is_some() => Ok(()),
            Some(_) => Err(RoomError::new(504, "the room did not show this Sceneuron in it")),
            None => Err(RoomError::new(
                502,
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| "the connection to the room ended".to_owned()),
            )),
        }
    }

    /// Ends session `generation` when it is still the session; the caller closes its connection.
    pub(crate) fn end_session(&self, generation: u64) {
        let mut state = self.lock();
        if state
            .session
            .as_ref()
            .is_some_and(|session| session.generation == generation)
        {
            state.take_session();
        }
        drop(state);
        self.changed.notify_all();
    }
}

pub(crate) fn unreachable(target: &Target, reason: &str) -> RoomError {
    RoomError::new(502, format!("cannot reach the rooms at {}: {reason}", target.server))
}

pub(crate) fn stopping() -> RoomError {
    RoomError::new(503, "Sceneuron is shutting down")
}
