// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the disk cache of spoken cards: ids per line, voice, and model, private files,
//! refusals of anything that is not an id, and deleting the oldest lines beyond the limit.

mod common;

use std::os::unix::fs::PermissionsExt;

use common::speech::mp3;
use common::temp_dir;
use sceneuron_ai::{MAX_SPEECH_FILES, SpeechCache};

#[test]
fn the_cache_keeps_each_line_once_per_voice_and_model() {
    let dir = temp_dir("speech-cache").join("speech");
    let cache = SpeechCache::new(dir.clone());
    let id = SpeechCache::id("Wrap it up", "voice123", "eleven_flash_v2_5");
    assert!(SpeechCache::is_id(&id));
    assert_eq!(id, SpeechCache::id("Wrap it up", "voice123", "eleven_flash_v2_5"));
    for other in [
        SpeechCache::id("Wrap it up!", "voice123", "eleven_flash_v2_5"),
        SpeechCache::id("Wrap it up", "voice456", "eleven_flash_v2_5"),
        SpeechCache::id("Wrap it up", "voice123", "eleven_v3"),
    ] {
        assert_ne!(id, other);
    }
    assert!(!cache.contains(&id) && cache.get(&id).is_none());
    cache.put(&id, &mp3()).unwrap();
    assert!(cache.contains(&id));
    assert_eq!(cache.get(&id), Some(mp3()));
    for bad in ["", "../ai", &id.to_uppercase(), &id[..63]] {
        assert!(!SpeechCache::is_id(bad) && cache.get(bad).is_none(), "{bad:?}");
        assert!(cache.put(bad, b"x").is_err());
    }
    let mode = |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&dir.join(format!("{id}.mp3"))), 0o600);
}

#[test]
fn the_cache_deletes_the_oldest_lines_beyond_its_limit() {
    let dir = temp_dir("speech-trim").join("speech");
    assert_eq!(MAX_SPEECH_FILES, 300);
    let cache = SpeechCache::with_limit(dir.clone(), 5);
    let ids: Vec<String> = (0..7)
        .map(|n| SpeechCache::id(&format!("line {n}"), "v", "m"))
        .collect();
    for id in &ids {
        cache.put(id, &mp3()).unwrap();
    }
    let kept = std::fs::read_dir(&dir).unwrap().count();
    assert_eq!(kept, 5);
    assert!(cache.contains(ids.last().unwrap()), "the newest line stays");
}
