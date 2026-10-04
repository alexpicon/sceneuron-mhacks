// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Practice teammates, so one person can show the shared room: each is its own connection to the
//! rooms, with its own identity, so the room module counts it as one more person. They join the
//! room this Sceneuron is in, vote one after another a few seconds into each round with cards
//! around one value, as a real team's are, and wait for the reveal. Stopping closes their
//! connections, which takes them out of the room.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::backend::{BackendEvent, Call, CallError, Connector, Events, RoomBackend, RoomView};
use crate::target::Target;

/// Who joins, in this order.
pub(crate) const NAMES: [&str; 4] = ["Maya", "Sam", "Priya", "Leo"];
/// Story points in order, so a round's cards can sit around one value.
const SCALE: [&str; 6] = ["p1", "p2", "p3", "p5", "p8", "p13"];
/// From a round's start to the first teammate's vote.
const FIRST_VOTE: Duration = Duration::from_secs(4);
/// Between two teammates' votes, give or take a third.
const VOTE_GAP: Duration = Duration::from_millis(2_500);
/// How long a teammate waits to connect and to join.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);

/// Running practice teammates; dropping them does not stop them, [`Practice::stop`] does.
pub(crate) struct Practice {
    backends: Vec<Arc<dyn RoomBackend>>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl Practice {
    /// Connects the teammates to `target`, puts them in room `code`, and starts them voting; an
    /// error says why one could not join, and the ones already in leave again. A teammate whose
    /// name is in `present` stays out, so a room another Sceneuron already practices in gets no
    /// second Maya.
    pub(crate) fn start(
        connector: &dyn Connector,
        target: &Target,
        code: &str,
        present: &[String],
    ) -> Result<Practice, String> {
        let (views, inbox) = mpsc::channel::<RoomView>();
        let mut backends: Vec<Arc<dyn RoomBackend>> = Vec::new();
        let absent = NAMES.iter().filter(|name| !present.iter().any(|here| here == *name));
        let joined = absent.into_iter().try_for_each(|name| {
            let sender = Mutex::new(views.clone());
            let events: Events = Box::new(move |event| {
                if let BackendEvent::Room(Some(view)) = event {
                    let _ = sender.lock().unwrap_or_else(PoisonError::into_inner).send(view);
                }
            });
            let backend = connector.connect(target, events)?;
            backends.push(Arc::clone(&backend));
            backend.ready(JOIN_TIMEOUT)?;
            join(backend.as_ref(), code, name)
        });
        if let Err(reason) = joined {
            close_all(&backends);
            return Err(reason);
        }
        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let voters = backends.clone();
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("sceneuron-practice".to_owned())
            .spawn(move || vote_each_round(&voters, &inbox, &flag))
            .map_err(|error| {
                close_all(&backends);
                format!("cannot start the practice teammates: {error}")
            })?;
        Ok(Practice {
            backends,
            stop,
            thread: Some(thread),
        })
    }

    /// Stops the votes and closes every teammate's connection, which takes them out of the room.
    pub(crate) fn stop(mut self) {
        let (lock, wake) = &*self.stop;
        *lock.lock().unwrap_or_else(PoisonError::into_inner) = true;
        wake.notify_all();
        close_all(&self.backends);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn close_all(backends: &[Arc<dyn RoomBackend>]) {
    for backend in backends {
        backend.close();
    }
}

/// Sends one teammate's join and waits for its answer.
fn join(backend: &dyn RoomBackend, code: &str, name: &str) -> Result<(), String> {
    let (done, answer) = mpsc::channel();
    let call = Call::Join {
        code: code.to_owned(),
        name: (*name).to_owned(),
    };
    backend.send(
        call,
        Box::new(move |result| {
            let _ = done.send(result);
        }),
    );
    match answer.recv_timeout(JOIN_TIMEOUT) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(CallError::Refused(reason) | CallError::Failed(reason))) => {
            Err(format!("{name} could not join: {reason}"))
        }
        Err(_) => Err(format!("{name} could not join in {JOIN_TIMEOUT:?}")),
    }
}

/// Votes in every round that starts while the teammates run, until stopped.
fn vote_each_round(backends: &[Arc<dyn RoomBackend>], inbox: &Receiver<RoomView>, stop: &(Mutex<bool>, Condvar)) {
    let mut latest: Option<RoomView> = None;
    let mut voted_round = None;
    let mut random = Random::seeded();
    loop {
        if stopped(stop) {
            return;
        }
        match inbox.recv_timeout(Duration::from_millis(250)) {
            Ok(view) => latest = Some(view),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(view) = latest.as_ref() else { continue };
        if view.revealed || voted_round == Some(view.round) {
            continue;
        }
        let round = view.round;
        voted_round = Some(round);
        let middle = 2 + random.below(3);
        if !pause(stop, FIRST_VOTE) {
            return;
        }
        for (index, backend) in backends.iter().enumerate() {
            while let Ok(view) = inbox.try_recv() {
                latest = Some(view);
            }
            if latest.as_ref().is_none_or(|view| view.round != round || view.revealed) {
                break;
            }
            let step = [0, 0, 0, 1, 2][random.below(5)];
            let card = SCALE[(middle + step).saturating_sub(1).min(SCALE.len() - 1)];
            backend.send(Call::Vote { card: card.to_owned() }, Box::new(|_| {}));
            let jitter = VOTE_GAP.as_millis() as u64 / 3;
            let gap = VOTE_GAP.as_millis() as u64 - jitter + random.below(2 * jitter as usize + 1) as u64;
            if index + 1 < backends.len() && !pause(stop, Duration::from_millis(gap)) {
                return;
            }
        }
    }
}

fn stopped(stop: &(Mutex<bool>, Condvar)) -> bool {
    *stop.0.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Waits `time` unless stopped first; false when stopped.
fn pause(stop: &(Mutex<bool>, Condvar), time: Duration) -> bool {
    let (lock, wake) = stop;
    let deadline = Instant::now() + time;
    let mut done = lock.lock().unwrap_or_else(PoisonError::into_inner);
    while !*done {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        done = wake.wait_timeout(done, left).unwrap_or_else(PoisonError::into_inner).0;
    }
    false
}

/// A small xorshift generator: practice cards need variety, not quality.
struct Random(u64);

impl Random {
    fn seeded() -> Random {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |since| since.as_nanos() as u64);
        Random(nanos | 1)
    }

    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound.max(1) as u64) as usize
    }
}
