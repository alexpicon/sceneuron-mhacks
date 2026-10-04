// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The room client: it connects only when someone creates or joins a room, turns raising a card
//! in the bound deck into a hidden vote, and follows the room's reveals and rounds on this
//! Sceneuron's overlay.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use sceneuron_server::{Hub, HubEvent};

use crate::actions::DEFAULT_DECK;
use crate::backend::{Connector, RoomBackend, RoomView};
use crate::practice::Practice;
use crate::spacetime::SpacetimeConnector;
use crate::target::Target;

/// How long shutting down waits for the connections' threads.
pub(crate) const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

/// How long the client waits for SpacetimeDB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeouts {
    /// For a new connection to see the rooms.
    pub connect: Duration,
    /// For the answer to a call, such as joining or revealing.
    pub call: Duration,
    /// For the room to show this connection in it once joining succeeded.
    pub view: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            call: Duration::from_secs(10),
            view: Duration::from_secs(5),
        }
    }
}

/// The shared room as one Sceneuron takes part in it.
pub struct RoomClient {
    pub(crate) hub: Arc<Hub>,
    pub(crate) connector: Arc<dyn Connector>,
    pub(crate) timeouts: Timeouts,
    pub(crate) state: Mutex<State>,
    /// Signals every change of the session, for actions that wait for the room.
    pub(crate) changed: Condvar,
    /// One action at a time, so two editors never join two rooms at once.
    pub(crate) acting: Mutex<()>,
    /// Simulated teammates in this Sceneuron's room, while practicing.
    pub(crate) practice: Mutex<Option<Practice>>,
    pub(crate) this: Weak<RoomClient>,
}

pub(crate) struct State {
    pub(crate) session: Option<Session>,
    /// A connection still being made, so shutting down can close it too.
    pub(crate) connecting: Option<Arc<dyn RoomBackend>>,
    /// Connections closed outside shutting down, such as by leaving or by losing them, whose
    /// threads may still be saying goodbye, so shutting down can wait for them too.
    pub(crate) closing: Vec<Arc<dyn RoomBackend>>,
    /// The host is unloading this code: nothing connects again.
    pub(crate) stopped: bool,
    /// What the last create or join asked for, which the editor offers again.
    pub(crate) form: Form,
    /// What went wrong with no request to answer, such as a lost connection.
    pub(crate) error: Option<String>,
    /// Counts sessions, so events of an old connection are ignored.
    pub(crate) generation: u64,
}

pub(crate) struct Session {
    pub(crate) generation: u64,
    pub(crate) backend: Arc<dyn RoomBackend>,
    /// The id of the local deck whose cards vote.
    pub(crate) deck: String,
    pub(crate) name: String,
    /// The room as last reported; `None` until joining is confirmed.
    pub(crate) view: Option<RoomView>,
    /// The card this connection voted this round, or last tried to.
    pub(crate) voted: Option<String>,
    /// The overlay is catching up with the room; the deck is left alone until it has.
    pub(crate) syncing: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Form {
    pub(crate) target: Target,
    pub(crate) name: String,
    pub(crate) deck: String,
}

impl RoomClient {
    /// A client that connects to SpacetimeDB once someone creates or joins a room.
    pub fn start(hub: Arc<Hub>) -> Arc<RoomClient> {
        Self::with_connector(hub, Arc::new(SpacetimeConnector::default()), Timeouts::default())
    }

    /// A client that connects through `connector` and waits as `timeouts` say.
    pub fn with_connector(hub: Arc<Hub>, connector: Arc<dyn Connector>, timeouts: Timeouts) -> Arc<RoomClient> {
        let client = Arc::new_cyclic(|this| RoomClient {
            hub: Arc::clone(&hub),
            connector,
            timeouts,
            state: Mutex::new(State {
                session: None,
                connecting: None,
                closing: Vec::new(),
                stopped: false,
                form: Form {
                    target: Target::default(),
                    name: String::new(),
                    deck: DEFAULT_DECK.to_owned(),
                },
                error: None,
                generation: 0,
            }),
            changed: Condvar::new(),
            acting: Mutex::new(()),
            practice: Mutex::new(None),
            this: this.clone(),
        });
        let watcher = Arc::downgrade(&client);
        hub.add_listener(Box::new(move |event| {
            if matches!(event, HubEvent::StateChanged { .. })
                && let Some(client) = watcher.upgrade()
            {
                client.follow_deck();
            }
        }));
        client
    }

    /// Closes the connection, one still being made, and those already closed by leaving or by
    /// losing them, without changing the overlay, and waits a moment for all their threads to end,
    /// so a host can unload this code; nothing connects again afterwards. The room drops this
    /// participant when the connection ends.
    pub fn shutdown(&self) {
        self.stop_practice();
        let backends: Vec<Arc<dyn RoomBackend>> = {
            let mut state = self.lock();
            state.stopped = true;
            let session = state.session.take().map(|session| session.backend);
            let connecting = state.connecting.take();
            session
                .into_iter()
                .chain(connecting)
                .chain(state.closing.drain(..))
                .collect()
        };
        self.changed.notify_all();
        for backend in &backends {
            backend.close();
        }
        let deadline = Instant::now() + SHUTDOWN_WAIT;
        let running = backends
            .iter()
            .filter(|backend| !backend.join(deadline.saturating_duration_since(Instant::now())))
            .count();
        if running > 0 {
            log::warn!("{running} room connections were still closing after {SHUTDOWN_WAIT:?}");
        }
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl State {
    /// Takes the session out to end it. Its connection, which the caller closes, is kept until its
    /// threads end, so shutting down can wait for them.
    pub(crate) fn take_session(&mut self) -> Option<Session> {
        let session = self.session.take()?;
        self.keep_closing(Arc::clone(&session.backend));
        Some(session)
    }

    /// Keeps a connection being closed outside shutting down until its threads end, and forgets
    /// those whose threads already did.
    pub(crate) fn keep_closing(&mut self, backend: Arc<dyn RoomBackend>) {
        self.closing.retain(|closing| !closing.join(Duration::ZERO));
        self.closing.push(backend);
    }
}

/// Closes a connection and waits a moment for its threads to end.
pub(crate) fn close_and_wait(backend: &dyn RoomBackend) {
    backend.close();
    if !backend.join(SHUTDOWN_WAIT) {
        log::warn!("the room connection was still closing after {SHUTDOWN_WAIT:?}");
    }
}
