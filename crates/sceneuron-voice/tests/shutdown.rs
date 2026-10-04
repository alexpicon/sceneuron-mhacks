// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises shutting the voice input down while a connection is still opening: the attempt is
//! abandoned at once instead of waited out, and no thread is left behind.

mod common;

use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use common::config;
use common::recorder::{Recorder, has_status};
use sceneuron_voice::{Mode, Transport, Voice, VoiceStatus};

#[test]
fn a_server_that_never_answers_does_not_hold_up_shutdown() {
    // Accepts connections and never says a word, like a server behind a broken proxy.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (accepted, connections) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if accepted.send(stream).is_err() {
                return;
            }
        }
    });
    let recorder = Recorder::default();
    let mut config = config(Mode::PushToTalk);
    config.endpoint = format!("ws://127.0.0.1:{port}/v1/speech-to-text/realtime");
    let voice = Voice::start(config, Transport::default(), recorder.sink());
    voice.talk(true);
    recorder.wait_for("connecting", |events| {
        has_status(events, |status| *status == VoiceStatus::Connecting)
    });
    let _held = connections
        .recv_timeout(Duration::from_secs(5))
        .expect("the voice input connected");
    let started = Instant::now();
    voice.shutdown();
    let took = started.elapsed();
    assert!(took < Duration::from_secs(2), "shutdown took {took:?}");
}
