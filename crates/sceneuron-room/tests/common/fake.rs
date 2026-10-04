// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A fake of the room module and its connections, in memory: the same rules as `room/module`,
//! with every connection told about its room after each change, as SpacetimeDB tells subscribers.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sceneuron_room::{BackendEvent, Call, CallError, Connector, Done, Events, RoomBackend, RoomView, Target};

use super::world::World;

/// The rooms every fake connection shares.
#[derive(Default)]
pub struct FakeRooms {
    world: Mutex<World>,
    clients: Mutex<BTreeMap<String, Arc<Client>>>,
    connections: AtomicUsize,
    /// What the next connection attempt fails with.
    pub refuse_connect: Mutex<Option<String>>,
    /// New connections do not see the rooms yet, as when the server is slow to answer.
    pub hold_ready: AtomicBool,
    /// Calls get no answer at all, as when the server hangs.
    pub silent: AtomicBool,
    /// What the module refuses the next call with.
    pub refuse_call: Mutex<Option<String>>,
    /// Closed connections' threads do not end, as when a server is slow to answer the close.
    pub hold_join: AtomicBool,
    /// Calls that never got an answer, kept so they are not dropped.
    unanswered: Mutex<Vec<Done>>,
    /// Connections a room client is waiting for to see the rooms.
    waiting: AtomicUsize,
}

struct Client {
    events: Events,
    closed: AtomicBool,
    last: Mutex<Option<Option<RoomView>>>,
}

impl FakeRooms {
    pub fn new() -> Arc<FakeRooms> {
        Arc::new(FakeRooms::default())
    }

    pub fn connector(self: &Arc<Self>) -> Arc<dyn Connector> {
        Arc::new(FakeConnector(Arc::clone(self)))
    }

    /// The private vote of connection `id`, which no client can read.
    pub fn vote_of(&self, id: &str) -> Option<String> {
        self.world.lock().unwrap().votes.get(id).cloned()
    }

    /// Connections a room client is waiting for to see the rooms, which it does only once it
    /// tracks them as being made.
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::SeqCst)
    }

    /// Ids of every connection, oldest first.
    pub fn connection_ids(&self) -> Vec<String> {
        self.clients.lock().unwrap().keys().cloned().collect()
    }

    /// Connections that are not closed.
    pub fn open_connections(&self) -> usize {
        let clients = self.clients.lock().unwrap();
        clients
            .values()
            .filter(|client| !client.closed.load(Ordering::SeqCst))
            .count()
    }

    /// Ends connection `id` from the server's side.
    pub fn drop_connection(&self, id: &str) {
        let client = self.clients.lock().unwrap().get(id).cloned();
        self.world.lock().unwrap().leave(id);
        if let Some(client) = client
            && !client.closed.swap(true, Ordering::SeqCst)
        {
            (client.events)(BackendEvent::Disconnected(Some("the server went away".to_owned())));
        }
        self.publish();
    }

    /// Takes connection `id` out of its room while the connection stays open.
    pub fn remove_from_room(&self, id: &str) {
        self.world.lock().unwrap().leave(id);
        self.publish();
    }

    /// Tells every open connection about its room, when it changed.
    fn publish(&self) {
        let clients: Vec<(String, Arc<Client>)> = self
            .clients
            .lock()
            .unwrap()
            .iter()
            .map(|(id, client)| (id.clone(), Arc::clone(client)))
            .collect();
        for (id, client) in clients {
            let view = self.world.lock().unwrap().view(&id);
            let changed = {
                let mut last = client.last.lock().unwrap();
                let changed = last.as_ref() != Some(&view);
                *last = Some(view.clone());
                changed
            };
            if changed && !client.closed.load(Ordering::SeqCst) {
                (client.events)(BackendEvent::Room(view));
            }
        }
    }
}

struct FakeConnector(Arc<FakeRooms>);

impl Connector for FakeConnector {
    fn connect(&self, _target: &Target, events: Events) -> Result<Arc<dyn RoomBackend>, String> {
        let rooms = &self.0;
        if let Some(reason) = rooms.refuse_connect.lock().unwrap().take() {
            return Err(reason);
        }
        let id = format!("client-{}", rooms.connections.fetch_add(1, Ordering::SeqCst) + 1);
        let client = Arc::new(Client {
            events,
            closed: AtomicBool::new(false),
            last: Mutex::new(None),
        });
        rooms.clients.lock().unwrap().insert(id.clone(), client);
        Ok(Arc::new(FakeBackend {
            rooms: Arc::clone(rooms),
            id,
        }))
    }
}

struct FakeBackend {
    rooms: Arc<FakeRooms>,
    id: String,
}

impl FakeBackend {
    fn closed(&self) -> bool {
        let clients = self.rooms.clients.lock().unwrap();
        clients[&self.id].closed.load(Ordering::SeqCst)
    }
}

impl RoomBackend for FakeBackend {
    fn ready(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        self.rooms.waiting.fetch_add(1, Ordering::SeqCst);
        while self.rooms.hold_ready.load(Ordering::SeqCst) && !self.closed() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        self.rooms.waiting.fetch_sub(1, Ordering::SeqCst);
        if self.closed() {
            Err("the connection was closed".to_owned())
        } else if self.rooms.hold_ready.load(Ordering::SeqCst) {
            Err("no answer".to_owned())
        } else {
            Ok(())
        }
    }

    fn send(&self, call: Call, done: Done) {
        if self.rooms.silent.load(Ordering::SeqCst) {
            self.rooms.unanswered.lock().unwrap().push(done);
            return;
        }
        if let Some(reason) = self.rooms.refuse_call.lock().unwrap().take() {
            return done(Err(CallError::Refused(reason)));
        }
        let result = self.rooms.world.lock().unwrap().apply(&self.id, call);
        self.rooms.publish();
        done(result.map_err(CallError::Refused));
    }

    fn close(&self) {
        let client = self.rooms.clients.lock().unwrap().get(&self.id).cloned();
        if let Some(client) = client {
            client.closed.store(true, Ordering::SeqCst);
        }
        self.rooms.world.lock().unwrap().leave(&self.id);
        self.rooms.publish();
    }

    /// The connection's threads end once it is closed, and not while `hold_join` is set.
    fn join(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let ended = self.closed() && !self.rooms.hold_join.load(Ordering::SeqCst);
            if ended || Instant::now() >= deadline {
                return ended;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}
