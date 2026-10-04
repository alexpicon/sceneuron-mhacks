// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Spoken cards on disk: each line of text is generated once per voice and model, kept in a
//! folder only its owner can read, and served to the overlay by its id.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::tts::SPEECH_FORMAT;

/// Files kept before the oldest are deleted; a few decks of cards in a few voices fit easily.
pub const MAX_SPEECH_FILES: usize = 300;
const EXTENSION: &str = "mp3";

/// A folder of generated speech, one MP3 file per id.
#[derive(Clone, Debug)]
pub struct SpeechCache {
    dir: PathBuf,
    max_files: usize,
}

impl SpeechCache {
    /// A cache in `dir` that keeps up to [`MAX_SPEECH_FILES`] files.
    pub fn new(dir: PathBuf) -> SpeechCache {
        Self::with_limit(dir, MAX_SPEECH_FILES)
    }

    /// A cache that keeps up to `max_files` files, at least one.
    pub fn with_limit(dir: PathBuf, max_files: usize) -> SpeechCache {
        SpeechCache {
            dir,
            max_files: max_files.max(1),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The id of `text` spoken in `voice_id` by `model`: 64 hexadecimal digits, so it reveals
    /// nothing about the text and is safe in a file name and an address.
    pub fn id(text: &str, voice_id: &str, model: &str) -> String {
        let mut hash = Sha256::new();
        for part in [SPEECH_FORMAT, model, voice_id, text] {
            hash.update(part.as_bytes());
            hash.update([0]);
        }
        hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Whether `id` has the shape [`SpeechCache::id`] gives.
    pub fn is_id(id: &str) -> bool {
        id.len() == 64 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    /// The audio for `id`, if it was generated.
    pub fn get(&self, id: &str) -> Option<Vec<u8>> {
        if !Self::is_id(id) {
            return None;
        }
        fs::read(self.path(id)).ok()
    }

    pub fn contains(&self, id: &str) -> bool {
        Self::is_id(id) && self.path(id).is_file()
    }

    /// Keeps `audio` as `id`, then deletes the oldest files beyond the limit. The folder and
    /// the file are readable only by their owner.
    pub fn put(&self, id: &str, audio: &[u8]) -> io::Result<()> {
        if !Self::is_id(id) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a speech id"));
        }
        create_private_dir(&self.dir)?;
        let temporary = self.dir.join(format!("{id}.tmp"));
        let written = write_private(&temporary, audio).and_then(|()| fs::rename(&temporary, self.path(id)));
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        written?;
        self.trim(id);
        Ok(())
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXTENSION}"))
    }

    /// Deletes the least recently written files beyond the limit, never `kept`.
    fn trim(&self, kept: &str) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().is_some_and(|extension| extension == EXTENSION)
                    && path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| Self::is_id(stem) && stem != kept)
            })
            .filter_map(|path| Some((fs::metadata(&path).ok()?.modified().ok()?, path)))
            .collect();
        // The kept file is one of the allowed files.
        let allowed = self.max_files - 1;
        if files.len() <= allowed {
            return;
        }
        files.sort();
        for (_, path) in &files[..files.len() - allowed] {
            let _ = fs::remove_file(path);
        }
    }
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
