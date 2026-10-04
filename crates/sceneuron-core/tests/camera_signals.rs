// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the timing of cards raised by the camera: the 700 ms hold, lowering 1500 ms after
//! the signal ends, the first matching card winning, replacing a camera card, a sign held through
//! missed frames, and summaries that stop, arrive slowly, or arrive out of order.

mod common;

use common::camera::{by_camera, camera_project, nothing, raised, raised_by_camera, show, sign};
use common::{engine, last_event};
use sceneuron_core::{Effect, HandSign, SignalSummary};

#[test]
fn a_signal_held_for_700_ms_raises_its_card() {
    let mut engine = engine(camera_project());
    let revision = engine.revision();
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 0, 600));
    assert_eq!((raised(&engine), engine.revision()), (None, revision));
    assert!(engine.observe_signals(&sign(HandSign::ThumbsUp), 700).changed);
    assert_eq!(raised(&engine), by_camera("yes"));
    assert_eq!(engine.revision(), revision + 1);
    assert!(engine.effective_props("slot").unwrap().visible);
    assert_eq!(
        last_event(&engine, 700),
        ("camera".to_owned(), "card raised: Votes: Yes".to_owned())
    );
    assert!(!show(&mut engine, &sign(HandSign::ThumbsUp), 800, 3_000));
}

#[test]
fn a_broken_or_changed_signal_starts_its_hold_again() {
    let mut engine = engine(camera_project());
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 500);
    engine.observe_signals(&nothing(), 600);
    show(&mut engine, &sign(HandSign::ThumbsUp), 700, 1_300);
    assert_eq!(raised(&engine), None);
    engine.observe_signals(&sign(HandSign::ThumbsUp), 1_400);
    assert_eq!(raised(&engine), by_camera("yes"));

    let mut engine = common::engine(camera_project());
    show(&mut engine, &sign(HandSign::ThumbsUp), 0, 400);
    show(&mut engine, &sign(HandSign::ThumbsDown), 500, 1_100);
    assert_eq!(raised(&engine), None);
    engine.observe_signals(&sign(HandSign::ThumbsDown), 1_200);
    assert_eq!(raised(&engine), by_camera("no"));
}

#[test]
fn the_first_matching_card_is_the_candidate() {
    let mut engine = engine(camera_project());
    let both = SignalSummary {
        hands_raised: 1,
        signs: vec![HandSign::ThumbsDown],
        ..SignalSummary::default()
    };
    show(&mut engine, &both, 0, 700);
    assert_eq!(raised(&engine), by_camera("no"), "\"no\" comes before \"maybe\"");
    let arms = SignalSummary {
        arms_up: true,
        ..SignalSummary::default()
    };
    let mut engine = common::engine(camera_project());
    show(&mut engine, &arms, 0, 700);
    assert_eq!(raised(&engine), by_camera("maybe"));
}

#[test]
fn a_camera_card_lowers_1500_ms_after_the_last_matching_frame() {
    let mut engine = raised_by_camera();
    assert!(!show(&mut engine, &nothing(), 800, 2_100));
    assert_eq!(raised(&engine), by_camera("yes"));
    assert!(engine.observe_signals(&nothing(), 2_200).changed);
    assert_eq!(raised(&engine), None);
    assert_eq!(
        last_event(&engine, 2_200),
        ("camera".to_owned(), "card lowered: Votes".to_owned())
    );
}

#[test]
fn another_signal_keeps_the_card_up_and_replaces_it_after_its_hold() {
    let mut engine = raised_by_camera();
    show(&mut engine, &sign(HandSign::ThumbsDown), 800, 1_400);
    assert_eq!(raised(&engine), by_camera("yes"));
    show(&mut engine, &sign(HandSign::ThumbsDown), 1_500, 1_500);
    assert_eq!(raised(&engine), by_camera("no"));
    let slot = engine.snapshot(1_500).elements.into_iter().find(|e| e.id == "slot");
    assert_eq!(slot.unwrap().effect.map(|state| state.effect), Some(Effect::Pop));
    let flicker = [sign(HandSign::ThumbsUp), nothing(), sign(HandSign::ThumbsDown)];
    for (n, summary) in flicker.iter().cycle().take(40).enumerate() {
        engine.observe_signals(summary, 1_600 + 100 * n as u64);
    }
    assert_eq!(
        raised(&engine),
        by_camera("no"),
        "brief signals neither replace nor lower it"
    );
}

#[test]
fn a_sign_held_through_missed_frames_raises_its_card_once() {
    let mut project = camera_project();
    project.decks[0].auto_hide_ms = 15_000;
    let mut engine = engine(project);
    let revision = engine.revision();
    // Five frames a second for 40 s, and the detector misses one every 1.6 s.
    for frame in 0..200_u64 {
        let summary = if frame % 8 == 7 {
            nothing()
        } else {
            sign(HandSign::ThumbsUp)
        };
        engine.observe_signals(&summary, frame * 200);
        engine.tick(frame * 200);
        if frame * 200 == 15_600 {
            assert_eq!(raised(&engine), by_camera("yes"), "the card stays up until it hides");
        }
    }
    assert_eq!(raised(&engine), None);
    assert_eq!(engine.revision(), revision + 2, "one raise and one auto-hide");
    let events = engine.snapshot(40_000).events;
    let cards: Vec<(u64, &str, &str)> = events
        .iter()
        .filter(|event| event.detail.starts_with("card "))
        .map(|event| (event.at_ms, event.source.as_str(), event.detail.as_str()))
        .collect();
    assert_eq!(
        cards,
        [
            (15_800, "auto-hide", "card lowered: Votes"),
            (800, "camera", "card raised: Votes: Yes"),
        ]
    );
}

#[test]
fn camera_cards_lower_when_summaries_stop() {
    let mut engine = raised_by_camera();
    assert!(!engine.tick(2_700).changed, "no frame said the signal ended");
    assert!(engine.tick(2_701).changed);
    assert_eq!(raised(&engine), None);
    assert_eq!(last_event(&engine, 2_701).0, "camera");

    let mut engine = raised_by_camera();
    assert!(engine.observe_signals(&sign(HandSign::ThumbsUp), 2_701).changed);
    assert_eq!(raised(&engine), None, "a gap breaks the run");
    show(&mut engine, &sign(HandSign::ThumbsUp), 2_801, 3_401);
    assert_eq!(raised(&engine), by_camera("yes"));
}

#[test]
fn a_slow_sample_rate_keeps_the_card_up_between_frames() {
    let mut project = camera_project();
    project.detection.sample_fps = 0.5;
    let mut engine = engine(project);
    engine.observe_signals(&sign(HandSign::ThumbsUp), 0);
    engine.observe_signals(&sign(HandSign::ThumbsUp), 2_000);
    assert_eq!(raised(&engine), by_camera("yes"));
    assert!(!engine.tick(3_900).changed);
    engine.observe_signals(&sign(HandSign::ThumbsUp), 4_000);
    assert!(engine.observe_signals(&nothing(), 6_000).changed);
    assert_eq!(raised(&engine), None);
}

#[test]
fn summaries_older_than_the_last_one_are_ignored() {
    let mut engine = engine(camera_project());
    engine.observe_signals(&sign(HandSign::ThumbsUp), 1_000);
    assert!(!engine.observe_signals(&sign(HandSign::ThumbsUp), 1_500).changed);
    assert!(!engine.observe_signals(&nothing(), 1_400).changed);
    assert!(
        engine.observe_signals(&sign(HandSign::ThumbsUp), 1_700).changed,
        "the older frame did not break the run"
    );
}
