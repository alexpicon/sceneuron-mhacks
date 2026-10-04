// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises how the director rests a cued condition after a look finds it not to hold: talking,
//! which keeps opening and closing the mouth, brings few looks for the tongue-out reaction; the
//! rest doubles with each further miss; a look that sees the tongue ends it; a declined look, or a
//! tongue seen less surely than the reaction asks, rests it too; and the status says why no look
//! comes.

mod common;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use common::sink::{RecordingSink, wait_until};
use common::{TEST_KEY, gray_frame};
use sceneuron_ai::{AiError, AiSettings, Director, MockProvider, ModelRequest};
use sceneuron_core::{AiPolicy, TONGUE_OUT, reactions};
use serde_json::json;

/// What the scripted vision model says when it is asked about the tongue.
#[derive(Clone, Copy)]
enum Answer {
    NoTongue,
    Tongue,
    /// A tongue, but less surely than the reaction's least confidence.
    Doubtful,
    Declined,
}

struct Watched {
    director: Director,
    sink: Arc<RecordingSink>,
    vision: Arc<MockProvider>,
    answer: Arc<Mutex<Answer>>,
}

impl Watched {
    fn answer(&self, answer: Answer) {
        *self.answer.lock().unwrap() = answer;
    }

    /// Reports `mouth` at each of `times`, waiting a little between reports like a camera does.
    fn show(&self, mouth: &str, times: &[u64]) {
        for &at in times {
            self.director.hint(facts(mouth), false, at);
            thread::sleep(Duration::from_millis(30));
        }
    }

    /// Opens the mouth at `at_ms` and keeps it open for the dwell time, then offers the frame
    /// wanted for the look, which must be look number `looks`.
    fn open_and_look(&self, at_ms: u64, looks: usize) {
        self.show("mouth open (0.3)", &[at_ms, at_ms + 400]);
        assert!(
            wait_until(|| self.director.wants_frame(at_ms + 410)),
            "no look wanted at {at_ms}"
        );
        self.director.offer_frame(gray_frame(32, 18, 90, at_ms + 450));
        assert!(
            wait_until(|| self.vision.calls() == looks),
            "look {looks} at {at_ms} never came"
        );
    }

    /// Asserts that the director wants no frame and makes no look, so `looks` stays the count.
    fn no_look(&self, at_ms: u64, looks: usize) {
        thread::sleep(Duration::from_millis(100));
        assert!(
            !self.director.wants_frame(at_ms),
            "a frame was wanted at {at_ms} while resting"
        );
        assert_eq!(self.vision.calls(), looks, "a look at {at_ms} while resting");
    }

    fn waits_with(&self, note: &str) -> bool {
        wait_until(|| {
            self.sink
                .last_status()
                .is_some_and(|status| status.message.contains(note))
        })
    }
}

fn start(answer: Answer) -> Watched {
    let answer = Arc::new(Mutex::new(answer));
    let said = Arc::clone(&answer);
    let vision = Arc::new(MockProvider::new(move |request: &ModelRequest| {
        let (holds, confidence) = match *said.lock().unwrap() {
            Answer::Declined => return Err(AiError::Refused("not this image".to_owned())),
            Answer::NoTongue => (false, 0.95),
            Answer::Tongue => (true, 0.95),
            Answer::Doubtful => (true, 0.5),
        };
        let verdicts = if request.text.contains(TONGUE_OUT) {
            json!([{"condition": TONGUE_OUT, "reason": "Looked at the mouth.", "holds": holds,
                    "confidence": confidence, "detail": ""}])
        } else {
            json!([])
        };
        Ok(json!({"caption": "", "verdicts": verdicts}))
    }));
    let settings = AiSettings {
        enabled: true,
        consent: true,
        api_key: TEST_KEY.to_owned(),
        ..AiSettings::default()
    };
    let sink = Arc::new(RecordingSink::default());
    let director = Director::spawn(settings, vision.connector(), Arc::clone(&sink) as _);
    let reaction = &reactions()[0];
    // Only reasons start a look, and no rate limit holds one back.
    let policy = AiPolicy {
        min_interval_ms: 0,
        refresh_ms: 3_600_000,
        change_threshold: 1.0,
        max_calls_per_minute: 10_000,
        ..AiPolicy::default()
    };
    director.set_project_with_cues(vec![reaction.condition.clone()], vec![reaction.cue()], policy, false);
    sink.wait_for_state("watching");
    Watched {
        director,
        sink,
        vision,
        answer,
    }
}

fn facts(mouth: &str) -> Vec<String> {
    vec!["face in view (0.98)".to_owned(), mouth.to_owned()]
}

/// Talks for `ms` from `from_ms` on the host's clock: five reports a second, the mouth open in
/// four of every five, as the lips part and meet in running speech. Every frame the director
/// wants is offered at once.
fn talk(director: &Director, from_ms: u64, ms: u64) {
    for at in (from_ms..from_ms + ms).step_by(200) {
        let mouth = if (at / 200) % 5 == 4 {
            "mouth closed (0.04)"
        } else {
            "mouth open (0.12)"
        };
        director.hint(facts(mouth), false, at);
        thread::sleep(Duration::from_millis(5));
        if director.wants_frame(at) {
            director.offer_frame(gray_frame(32, 18, 90, at));
            thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn a_minute_of_talking_brings_at_most_two_looks() {
    let watched = start(Answer::NoTongue);
    talk(&watched.director, 1_000, 60_000);
    let looks = watched.vision.calls();
    assert!((1..=2).contains(&looks), "{looks} looks in a minute of talking");
    watched.director.shutdown();
}

#[test]
fn each_miss_doubles_the_rest() {
    let watched = start(Answer::NoTongue);
    watched.open_and_look(1_000, 1);
    assert!(watched.waits_with("see mouth open; Tongue out rests 20 s after a look found it not to hold"));
    watched.show("mouth open (0.3)", &[2_000, 10_000, 21_400]);
    watched.no_look(21_400, 1);
    watched.open_and_look(21_450, 2);
    watched.show("mouth open (0.3)", &[40_000, 61_400]);
    watched.no_look(61_400, 2);
    watched.open_and_look(61_500, 3);
    assert!(watched.waits_with("Tongue out rests 80 s after"));
    watched.show("mouth closed (0.02)", &[62_000]);
    watched.show("mouth open (0.3)", &[63_000, 63_400, 141_000]);
    watched.no_look(141_000, 3);
    watched.director.shutdown();
}

#[test]
fn a_look_that_sees_the_tongue_ends_the_rest() {
    let watched = start(Answer::NoTongue);
    watched.open_and_look(1_000, 1);
    watched.answer(Answer::Tongue);
    watched.show("mouth closed (0.02)", &[2_000]);
    watched.open_and_look(21_450, 2);
    assert!(wait_until(|| watched.sink.observations().len() == 2));
    assert!(watched.sink.observations()[1].0.verdicts[0].holds);
    watched.answer(Answer::NoTongue);
    watched.show("mouth closed (0.02)", &[22_000]);
    watched.open_and_look(22_200, 3);
    assert!(
        watched.waits_with("Tongue out rests 20 s after"),
        "the rest starts over"
    );
    watched.director.shutdown();
}

#[test]
fn a_declined_or_doubtful_look_rests_the_reaction_too() {
    for answer in [Answer::Declined, Answer::Doubtful] {
        let watched = start(answer);
        watched.open_and_look(1_000, 1);
        watched.show("mouth closed (0.02)", &[2_000]);
        watched.show("mouth open (0.3)", &[5_000, 5_400, 21_000]);
        watched.no_look(21_000, 1);
        watched.open_and_look(21_450, 2);
        watched.director.shutdown();
    }
}
