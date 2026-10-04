// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the camera preview tap: frames are wanted only while the editor asks and at most
//! five a second, the reasons it gives for having no picture, the analysis it keeps only while
//! watched, what drops the frame and the analysis once nobody watches or their filter is gone,
//! and when a copy of the analysis sample can serve as the preview.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sceneuron::preview::{FRAME_INTERVAL_MS, PreviewTap, STALE_MS, WATCH_MS, keeps_aspect};
use sceneuron_ai::Frame;
use sceneuron_server::{FaceArea, PreviewGap};
use sceneuron_vision::{FaceBox, Gesture, Hand, Perception};

fn tap_with_filters(count: usize) -> (PreviewTap, Arc<AtomicUsize>) {
    let filters = Arc::new(AtomicUsize::new(count));
    (PreviewTap::new(Arc::clone(&filters)), filters)
}

fn frame(captured_ms: u64) -> Frame {
    Frame {
        rgb: vec![7; 4 * 2 * 3].into(),
        width: 4,
        height: 2,
        captured_ms,
    }
}

#[test]
fn frames_are_wanted_only_while_the_editor_asks() {
    let (tap, _) = tap_with_filters(1);
    assert!(!tap.wants_frame(0), "nobody asked yet");
    tap.request(1_000);
    assert!(tap.wants_frame(1_000));
    assert!(tap.wants_frame(1_000 + WATCH_MS));
    assert!(!tap.wants_frame(1_001 + WATCH_MS), "the request ran out");
    assert!(!tap.watching(1_001 + WATCH_MS));
}

#[test]
fn a_stored_frame_holds_off_the_next_for_the_frame_interval() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    tap.store(frame(1_000));
    assert!(!tap.wants_frame(1_000 + FRAME_INTERVAL_MS - 1));
    assert!(tap.wants_frame(1_000 + FRAME_INTERVAL_MS));
}

#[test]
fn the_latest_fresh_frame_is_shown() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    tap.store(frame(1_100));
    tap.store(frame(1_300));
    assert_eq!(tap.frame(1_300 + STALE_MS).map(|frame| frame.captured_ms), Ok(1_300));
}

#[test]
fn without_an_analysis_filter_there_is_no_source() {
    let (tap, filters) = tap_with_filters(0);
    tap.request(1_000);
    assert_eq!(tap.frame(1_000), Err(PreviewGap::NoSource));
    filters.store(1, Ordering::SeqCst);
    tap.store(frame(1_000));
    assert!(tap.frame(1_050).is_ok());
    filters.store(0, Ordering::SeqCst);
    assert_eq!(tap.frame(1_050), Err(PreviewGap::NoSource), "the filter was removed");
}

#[test]
fn a_new_watch_waits_for_its_first_frame_and_then_calls_the_source_stale() {
    let (tap, _) = tap_with_filters(1);
    tap.request(10_000);
    assert_eq!(tap.frame(10_000), Err(PreviewGap::Waiting));
    tap.request(10_000 + STALE_MS);
    assert_eq!(tap.frame(10_000 + STALE_MS), Err(PreviewGap::Waiting));
    tap.request(10_001 + STALE_MS);
    assert_eq!(tap.frame(10_001 + STALE_MS), Err(PreviewGap::Stale));
    tap.request(13_000);
    tap.store(frame(13_000));
    assert!(tap.frame(13_000).is_ok());
    tap.request(14_000);
    tap.request(13_001 + STALE_MS);
    assert_eq!(tap.frame(13_001 + STALE_MS), Err(PreviewGap::Stale), "frames stopped");
}

#[test]
fn coming_back_after_a_pause_waits_again_instead_of_showing_an_old_frame() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    tap.store(frame(1_000));
    let later = 1_000 + WATCH_MS + 60_000;
    tap.request(later);
    assert_eq!(tap.frame(later), Err(PreviewGap::Waiting));
}

#[test]
fn the_last_frame_is_dropped_once_nobody_watches() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    let last = 1_000 + WATCH_MS;
    tap.store(frame(last));
    tap.observe(&Perception::default(), last, last);
    assert!(tap.frame(last).is_ok(), "still watched");
    tap.observe(&Perception::default(), last + 1, last + 1);
    tap.request(last + 2);
    assert_eq!(
        tap.frame(last + 2),
        Err(PreviewGap::Waiting),
        "the frame, though fresh, was dropped"
    );
    assert!(tap.wants_frame(last + 2), "and the next one is wanted at once");
}

#[test]
fn the_tick_drops_the_frame_and_the_analysis_once_the_watch_lapses() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    let last = 1_000 + WATCH_MS;
    tap.store(frame(last));
    tap.observe(&perception(), last, last);
    tap.expire(last);
    assert!(!tap.is_empty(), "still watched");
    tap.expire(last + 1);
    assert!(tap.is_empty(), "dropped with no further analysis");
    tap.request(last + 2);
    assert_eq!(
        tap.frame(last + 2),
        Err(PreviewGap::Waiting),
        "the frame, though fresh, is gone"
    );
    assert_eq!(tap.analysis(last + 2).age_ms, None, "and so is the analysis");
}

#[test]
fn what_went_stale_while_watched_is_dropped_when_asked_for() {
    let (tap, _) = tap_with_filters(1);
    tap.request(1_000);
    tap.store(frame(1_000));
    tap.observe(&perception(), 1_000, 1_000);
    let later = 1_001 + STALE_MS;
    tap.request(2_000);
    tap.request(later);
    assert_eq!(tap.frame(later), Err(PreviewGap::Stale), "the camera stopped");
    assert_eq!(tap.analysis(later).age_ms, None);
    assert!(tap.is_empty(), "neither would ever be shown");
}

#[test]
fn removing_the_filter_that_fed_the_preview_drops_what_it_held() {
    let (tap, filters) = tap_with_filters(2);
    let fill = |at: u64| {
        tap.request(at);
        tap.store(frame(at));
        tap.observe(&perception(), at, at);
    };
    fill(1_000);
    tap.filter_removed(false);
    assert!(!tap.is_empty(), "another filter fed it");
    tap.filter_removed(true);
    assert!(tap.is_empty(), "the filter that fed it is gone");
    fill(2_000);
    filters.store(0, Ordering::SeqCst);
    tap.filter_removed(false);
    assert!(tap.is_empty(), "no filter is left to tick");
}

fn perception() -> Perception {
    Perception {
        faces: vec![FaceBox {
            x1: 0.25,
            y1: 0.125,
            x2: 0.5,
            y2: 0.5,
            score: 0.98,
        }],
        best_face: 0.98,
        hands: vec![Hand {
            raised: true,
            gesture: Some(Gesture::ThumbsUp),
            ..Hand::default()
        }],
        ..Perception::default()
    }
}

#[test]
fn analysis_is_kept_only_while_watched_and_while_fresh() {
    let (tap, _) = tap_with_filters(1);
    tap.observe(&perception(), 900, 1_000);
    assert_eq!(tap.analysis(1_000).age_ms, None, "nobody was watching");
    tap.request(1_000);
    tap.observe(&perception(), 1_100, 1_150);
    let analysis = tap.analysis(1_200);
    assert_eq!(analysis.seen, ["face in view (0.98)", "hand raised, thumbs up"]);
    let face = FaceArea {
        x1: 0.25,
        y1: 0.125,
        x2: 0.5,
        y2: 0.5,
        score: 0.98,
    };
    assert_eq!(analysis.faces, [face]);
    assert_eq!(analysis.age_ms, Some(100));
    assert_eq!(tap.analysis(1_101 + STALE_MS).age_ms, None, "too old to show");
    tap.observe(&perception(), 1_100 + WATCH_MS * 3, 1_100 + WATCH_MS * 3);
    assert_eq!(tap.analysis(1_100 + WATCH_MS * 3).seen.len(), 0, "the watch ended");
}

#[test]
fn only_an_analysis_sample_with_the_frames_shape_serves_as_the_preview() {
    assert!(keeps_aspect(640, 360, 1280, 720), "Apple Vision fits the frame");
    assert!(keeps_aspect(640, 480, 1920, 1440));
    assert!(keeps_aspect(1280, 720, 1280, 720));
    assert!(!keeps_aspect(320, 240, 1280, 720), "UltraFace stretches a wide frame");
    assert!(!keeps_aspect(360, 640, 1280, 720));
}
