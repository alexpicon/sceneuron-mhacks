// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! What the editor and control surfaces ask of the room through `/api/room`: creating, joining,
//! and leaving a room, revealing the votes, and starting the next round.

use std::sync::{Arc, PoisonError};

use sceneuron_core::RoomStep;
use sceneuron_server::{Room, RoomAction, RoomError, RoomRequest};
use serde_json::Value;

use crate::backend::{Call, Events};
use crate::client::{Form, RoomClient, Session};
use crate::local::{self, Local};
use crate::practice::Practice;
use crate::session::{stopping, unreachable};
use crate::target::Target;

/// Longest display name, in characters, as the room module allows.
const MAX_NAME_CHARS: usize = 40;

/// The deck whose cards vote when a request names none: the planning poker pack's.
pub(crate) const DEFAULT_DECK: &str = "poker";

impl Room for RoomClient {
    fn state(&self) -> Value {
        crate::state::state(self)
    }

    fn act(&self, request: RoomRequest) -> Result<Value, RoomError> {
        let _one_at_a_time = self.acting.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match request.action {
            RoomAction::Create | RoomAction::Join | RoomAction::Open => self.enter(&request)?,
            RoomAction::Leave => {
                self.stop_practice();
                self.leave()?;
            }
            RoomAction::Practice => self.start_practice()?,
            RoomAction::PracticeStop => self.stop_practice(),
            RoomAction::Reveal => self.call_in_room(Call::Reveal)?,
            RoomAction::Next => self.call_in_room(Call::NextRound)?,
        }
        Ok(self.state())
    }
}

impl RoomClient {
    /// Starts what voice asked of the room and says so in a few words. The step runs on its own
    /// thread: it waits for the room's answer, which arrives through the hub, and a voice host may
    /// be on a thread the hub needs.
    pub fn voice_step(&self, step: RoomStep) -> String {
        if self
            .lock()
            .session
            .as_ref()
            .and_then(|session| session.view.as_ref())
            .is_none()
        {
            return "this Sceneuron is not in a room; join one in the editor's Cards tab".to_owned();
        }
        let action = match step {
            RoomStep::Reveal => RoomAction::Reveal,
            RoomStep::Next => RoomAction::Next,
        };
        let request = RoomRequest {
            action,
            code: None,
            name: None,
            deck: None,
            server: None,
            database: None,
        };
        let client = self.this.clone();
        let started = std::thread::Builder::new()
            .name("sceneuron-room-voice".to_owned())
            .spawn(move || {
                if let Some(client) = client.upgrade()
                    && let Err(error) = client.act(request)
                {
                    log::warn!("voice could not change the room: {}", error.message);
                }
            });
        match (started, step) {
            (Ok(_), RoomStep::Reveal) => "revealing the votes".to_owned(),
            (Ok(_), RoomStep::Next) => "starting the next round".to_owned(),
            (Err(error), _) => format!("the room did not change: {error}"),
        }
    }

    fn enter(&self, request: &RoomRequest) -> Result<(), RoomError> {
        if let Some(code) = self.lock().session.as_ref().map(room_code) {
            return Err(RoomError::new(
                409,
                format!("this Sceneuron is in room {code}; leave it first"),
            ));
        }
        let name = check_name(request.name.as_deref().unwrap_or_default())?;
        let deck = request
            .deck
            .as_deref()
            .map(str::trim)
            .filter(|deck| !deck.is_empty())
            .unwrap_or(DEFAULT_DECK)
            .to_owned();
        let target = Target::new(request.server.as_deref(), request.database.as_deref())
            .map_err(|message| RoomError::new(400, message))?;
        self.lock().form = Form {
            target: target.clone(),
            name: name.clone(),
            deck: deck.clone(),
        };
        let deck_name = self
            .hub
            .read(|engine| engine.project().deck(&deck).map(|found| found.name.clone()))
            .ok_or_else(|| {
                let message = format!("this project has no saved deck {deck:?}; add the Planning poker pack and save");
                RoomError::new(400, message)
            })?;
        let call = match request.action {
            RoomAction::Join => Call::Join {
                code: check_code(request.code.as_deref().unwrap_or_default())?,
                name: name.clone(),
            },
            RoomAction::Open => Call::Open {
                code: check_code(request.code.as_deref().unwrap_or_default())?,
                deck_name,
                name: name.clone(),
            },
            _ => Call::Create {
                deck_name,
                name: name.clone(),
            },
        };
        let generation = {
            let mut state = self.lock();
            if state.stopped {
                return Err(stopping());
            }
            state.generation += 1;
            state.generation
        };
        let this = self.this.clone();
        let events: Events = Box::new(move |event| {
            if let Some(client) = this.upgrade() {
                client.on_event(generation, event);
            }
        });
        let backend = self
            .connector
            .connect(&target, events)
            .map_err(|reason| unreachable(&target, &reason))?;
        let session = Session {
            generation,
            backend: Arc::clone(&backend),
            deck,
            name,
            view: None,
            voted: None,
            syncing: false,
        };
        self.open(session, &target)?;
        let joined = self
            .call(backend.as_ref(), call)
            .and_then(|()| self.wait_for_room(generation));
        if joined.is_err() {
            self.end_session(generation);
            backend.close();
        }
        joined
    }

    fn leave(&self) -> Result<(), RoomError> {
        let session = self
            .lock()
            .take_session()
            .ok_or_else(|| RoomError::new(409, "this Sceneuron is not in a room"))?;
        self.changed.notify_all();
        // Closing the connection takes this participant out of the room even when the call
        // fails, so the local overlay leaves either way.
        let _ = self.call(session.backend.as_ref(), Call::Leave);
        session.backend.close();
        self.hub
            .update(|engine, now| local::apply(engine, &session.deck, &[Local::Leave], Some(""), now));
        Ok(())
    }

    fn start_practice(&self) -> Result<(), RoomError> {
        if self.practice.lock().unwrap_or_else(PoisonError::into_inner).is_some() {
            return Ok(());
        }
        let (target, code, present) = {
            let state = self.lock();
            let view = state
                .session
                .as_ref()
                .and_then(|session| session.view.as_ref())
                .ok_or_else(|| RoomError::new(409, "join a room first"))?;
            let present: Vec<String> = view.members.iter().map(|member| member.name.clone()).collect();
            (state.form.target.clone(), view.code.clone(), present)
        };
        let practice = Practice::start(self.connector.as_ref(), &target, &code, &present)
            .map_err(|reason| RoomError::new(502, reason))?;
        *self.practice.lock().unwrap_or_else(PoisonError::into_inner) = Some(practice);
        Ok(())
    }

    /// Takes the simulated teammates out of the room, if any are in it.
    pub(crate) fn stop_practice(&self) {
        let practice = self.practice.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(practice) = practice {
            practice.stop();
        }
    }

    fn call_in_room(&self, call: Call) -> Result<(), RoomError> {
        let backend = self
            .lock()
            .session
            .as_ref()
            .filter(|session| session.view.is_some())
            .map(|session| Arc::clone(&session.backend))
            .ok_or_else(|| RoomError::new(409, "join a room first"))?;
        self.call(backend.as_ref(), call)
    }
}

fn room_code(session: &Session) -> String {
    session
        .view
        .as_ref()
        .map_or_else(|| "being joined".to_owned(), |view| view.code.clone())
}

fn check_name(name: &str) -> Result<String, RoomError> {
    let name = name.trim();
    let count = name.chars().count();
    if count == 0 || count > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        let message = format!("enter a display name of 1 to {MAX_NAME_CHARS} characters");
        return Err(RoomError::new(400, message));
    }
    Ok(name.to_owned())
}

/// A room code as typed, in capitals; the room module checks it fully.
fn check_code(code: &str) -> Result<String, RoomError> {
    let code = code.trim().to_ascii_uppercase();
    if (4..=12).contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Ok(code)
    } else {
        Err(RoomError::new(400, "enter the room's code: 4 to 12 letters and digits"))
    }
}
