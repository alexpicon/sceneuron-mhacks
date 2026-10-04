// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the plugin's side of pairing the Stream Deck by itself: the home folder it looks in,
//! with `CFFIXED_USER_HOME` before `HOME` as Foundation does, whether Sceneuron's Stream Deck
//! plugin is installed there, the rule that lets one pairing through per OBS run, and the wait
//! before pairing at start, which pairs once when it ends and never once stopped.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use sceneuron::stream_deck::{
    OncePerRun, PairAtStart, RECONNECT_GRACE, home_folder, plugin_folder, plugin_installed_under,
};

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stream-deck-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn os(text: &str) -> Option<OsString> {
    Some(OsString::from(text))
}

fn stream_deck_plugins(home: &Path) -> PathBuf {
    home.join("Library/Application Support/com.elgato.StreamDeck/Plugins")
}

#[test]
fn the_fixed_home_wins_over_home_and_an_empty_one_counts_as_unset() {
    assert_eq!(
        home_folder(os("/scratch"), os("/Users/me")),
        Some(PathBuf::from("/scratch"))
    );
    assert_eq!(home_folder(os(""), os("/Users/me")), Some(PathBuf::from("/Users/me")));
    assert_eq!(home_folder(None, os("/Users/me")), Some(PathBuf::from("/Users/me")));
    assert_eq!(home_folder(None, os("")), None);
    assert_eq!(home_folder(None, None), None);
}

#[test]
fn the_plugin_is_where_the_stream_deck_app_installs_it() {
    assert_eq!(
        plugin_folder(Path::new("/Users/me")),
        PathBuf::from(
            "/Users/me/Library/Application Support/com.elgato.StreamDeck/Plugins/ai.chaski.sceneuron.sdPlugin"
        )
    );
}

#[test]
fn the_plugin_counts_as_installed_only_when_its_folder_is_there() {
    let home = scratch("installed");
    assert!(!plugin_installed_under(&home), "no Stream Deck app");
    fs::create_dir_all(stream_deck_plugins(&home).join("com.elgato.volume-controller.sdPlugin")).unwrap();
    assert!(!plugin_installed_under(&home), "only other plugins");
    fs::write(plugin_folder(&home), "not a folder").unwrap();
    assert!(!plugin_installed_under(&home), "a file in its place");
    fs::remove_file(plugin_folder(&home)).unwrap();
    fs::create_dir_all(plugin_folder(&home)).unwrap();
    assert!(plugin_installed_under(&home));
}

#[test]
fn a_linked_plugin_counts_as_installed_while_its_folder_exists() {
    let home = scratch("linked");
    let built = home.join("checkout/ai.chaski.sceneuron.sdPlugin");
    fs::create_dir_all(stream_deck_plugins(&home)).unwrap();
    symlink(&built, plugin_folder(&home)).unwrap();
    assert!(!plugin_installed_under(&home), "a link to nothing");
    fs::create_dir_all(&built).unwrap();
    assert!(plugin_installed_under(&home));
}

#[test]
fn one_pairing_gets_through_per_run() {
    let run = OncePerRun::new();
    assert!(run.first());
    assert!(!run.first());
    assert!(!run.first());
    let shared = Arc::new(OncePerRun::new());
    let firsts: usize = (0..8)
        .map(|_| {
            let shared = Arc::clone(&shared);
            thread::spawn(move || usize::from(shared.first()))
        })
        .map(|thread| thread.join().unwrap())
        .sum();
    assert_eq!(firsts, 1, "threads asking at once");
}

#[test]
fn obs_waits_long_enough_for_a_paired_stream_deck_to_reconnect() {
    assert!(
        RECONNECT_GRACE > Duration::from_secs(30),
        "the plugin's longest wait between tries"
    );
    assert!(RECONNECT_GRACE <= Duration::from_secs(60));
}

#[test]
fn pairing_at_start_runs_once_after_the_wait() {
    let (sent, received) = mpsc::channel();
    let started = Instant::now();
    let _pairing = PairAtStart::start(Duration::from_millis(150), move || sent.send(Instant::now()).unwrap()).unwrap();
    let paired = received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(paired - started >= Duration::from_millis(150));
    assert!(
        received.recv_timeout(Duration::from_millis(300)).is_err(),
        "it pairs once"
    );
}

#[test]
fn stopping_before_the_wait_ends_pairs_nothing() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&runs);
    let mut pairing = PairAtStart::start(Duration::from_secs(60), move || {
        counted.fetch_add(1, Ordering::SeqCst);
    })
    .unwrap();
    let asked = Instant::now();
    pairing.stop();
    assert!(
        asked.elapsed() < Duration::from_secs(5),
        "stopping does not wait out the grace"
    );
    pairing.stop();
    let counted = Arc::clone(&runs);
    drop(PairAtStart::start(Duration::from_secs(60), move || {
        counted.fetch_add(1, Ordering::SeqCst);
    }));
    assert_eq!(runs.load(Ordering::SeqCst), 0, "neither a stop nor a drop lets it pair");
}

#[test]
fn stopping_while_it_pairs_waits_for_the_pairing_to_finish() {
    let (began, has_begun) = mpsc::channel();
    let finished = Arc::new(AtomicUsize::new(0));
    let done = Arc::clone(&finished);
    let mut pairing = PairAtStart::start(Duration::ZERO, move || {
        began.send(()).unwrap();
        thread::sleep(Duration::from_millis(200));
        done.fetch_add(1, Ordering::SeqCst);
    })
    .unwrap();
    has_begun.recv_timeout(Duration::from_secs(5)).unwrap();
    pairing.stop();
    assert_eq!(finished.load(Ordering::SeqCst), 1);
}

#[test]
fn two_starts_in_one_run_pair_once() {
    let run = Arc::new(OncePerRun::new());
    let (sent, received) = mpsc::channel();
    let pairings: Vec<_> = (0..2)
        .map(|_| {
            let (run, sent) = (Arc::clone(&run), sent.clone());
            PairAtStart::start(Duration::from_millis(50), move || {
                if run.first() {
                    sent.send(()).unwrap();
                }
            })
            .unwrap()
        })
        .collect();
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(pairings);
    assert!(received.try_recv().is_err());
}
