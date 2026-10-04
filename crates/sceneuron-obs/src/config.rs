// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Plugin settings stored next to the project: the editor port and the access tokens.
//!
//! The file holds secrets, so it is written atomically and readable only by its owner inside a
//! directory only its owner can enter.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

/// Default editor port; obs-websocket uses 4455, so this stays clear of it.
pub const DEFAULT_PORT: u16 = 4466;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub port: u16,
    /// The editor's token, which allows every API request.
    pub token: String,
    /// The overlay's read-only token.
    pub overlay_token: String,
    /// The token paired Stream Decks hold. Revoking it in the editor replaces it, and a new
    /// editor address leaves it alone.
    pub control_token: String,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            token: String::new(),
            overlay_token: String::new(),
            control_token: String::new(),
        }
    }
}

impl PluginConfig {
    /// Loads the config, creating and saving missing or invalid tokens and repairing a
    /// privileged port. A file that others could read is made private.
    pub fn load_or_create(path: &Path) -> PluginConfig {
        let mut config: PluginConfig = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let mut changed = false;
        if !valid_token(&config.token) {
            config.token = sceneuron_server::generate_token();
            changed = true;
        }
        if !valid_token(&config.overlay_token) || config.overlay_token == config.token {
            config.overlay_token = sceneuron_server::generate_token();
            changed = true;
        }
        let reused = config.control_token == config.token || config.control_token == config.overlay_token;
        if !valid_token(&config.control_token) || reused {
            config.control_token = sceneuron_server::generate_token();
            changed = true;
        }
        if config.port < 1024 {
            config.port = DEFAULT_PORT;
            changed = true;
        }
        if changed {
            let _ = config.save(path);
        } else {
            make_private(path);
        }
        config
    }

    /// Replaces the editor and overlay tokens, so every address handed out before stops
    /// working. Paired Stream Decks keep working; revoking the control token is separate.
    pub fn regenerate_tokens(&mut self) {
        self.token = sceneuron_server::generate_token();
        self.overlay_token = sceneuron_server::generate_token();
    }

    /// Writes the file atomically: a crash leaves either the old or the new file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            create_private_dir(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".tmp");
        let temporary = path.with_file_name(name);
        {
            let mut file = private_file(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&temporary, path)
    }
}

/// The config as the plugin and its server share it, saved after every change.
///
/// Each change is made and saved under one lock, so two changes from different threads never
/// save over each other with an older copy.
pub struct SharedConfig {
    path: PathBuf,
    config: Mutex<PluginConfig>,
    /// The server that may change the control token; changed only under the config lock.
    server: AtomicU64,
}

impl SharedConfig {
    /// Loads the config from `path` as [`PluginConfig::load_or_create`] does.
    pub fn load_or_create(path: PathBuf) -> SharedConfig {
        let config = PluginConfig::load_or_create(&path);
        SharedConfig {
            path,
            config: Mutex::new(config),
            server: AtomicU64::new(0),
        }
    }

    /// Gives the control token to a new server and returns its id for
    /// [`SharedConfig::set_control_token`]. Call it before stopping the old server and read the
    /// config only after: a revoke the old server finished is then in the copy the new server
    /// starts with, and one it had not finished fails instead of saving a token the new server
    /// would never use.
    pub fn next_server(&self) -> u64 {
        let _config = self.lock();
        self.server.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn get(&self) -> PluginConfig {
        self.lock().clone()
    }

    /// Changes the config and saves it. The change stays in effect even when saving fails;
    /// the error tells the caller to warn that it will not survive a restart.
    pub fn update(&self, change: impl FnOnce(&mut PluginConfig)) -> (PluginConfig, io::Result<()>) {
        let mut config = self.lock();
        change(&mut config);
        let saved = config.save(&self.path);
        (config.clone(), saved)
    }

    /// Saves a new control token for `server` and only then puts it in effect, so a revoked
    /// token can never come back after a restart. When saving fails, or a newer server has
    /// replaced `server`, the old token stays.
    pub fn set_control_token(&self, server: u64, token: &str) -> Result<(), String> {
        let mut config = self.lock();
        if self.server.load(Ordering::SeqCst) != server {
            return Err("the editor server is restarting; try again".to_owned());
        }
        let next = PluginConfig {
            control_token: token.to_owned(),
            ..config.clone()
        };
        next.save(&self.path)
            .map_err(|error| format!("could not save settings: {error}"))?;
        *config = next;
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, PluginConfig> {
        self.config.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The server accepts at most 128 token characters; anything else is replaced.
fn valid_token(token: &str) -> bool {
    (32..=128).contains(&token.len()) && token.chars().all(|c| c.is_ascii_hexdigit())
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

fn private_file(path: &Path) -> io::Result<fs::File> {
    let _ = fs::remove_file(path);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

fn make_private(path: &Path) {
    let readable_by_others = fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o077 != 0);
    if readable_by_others {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    if let Some(dir) = path.parent() {
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
}

/// Paths of the files Sceneuron keeps in its OBS module config directory.
#[derive(Clone, Debug)]
pub struct Paths {
    pub project: PathBuf,
    pub config: PathBuf,
    /// AI provider settings, including the API key; written readable only by its owner.
    pub ai: PathBuf,
    /// Optional keys as `NAME=value` lines, read but never written.
    pub env: PathBuf,
}

impl Paths {
    pub fn in_dir(dir: &Path) -> Paths {
        Paths {
            project: dir.join("project.json"),
            config: dir.join("server.json"),
            ai: dir.join("ai.json"),
            env: dir.join(".env"),
        }
    }
}
