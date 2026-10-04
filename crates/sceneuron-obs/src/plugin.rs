// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Process-wide plugin state: the hub, the local server, the vision bridge, the AI, the shared
//! room, the voice input, and the Stream Deck pairing sent at start.

mod pair_at_start;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use sceneuron_ai::{AiService, Assistant};
use sceneuron_room::RoomClient;
use sceneuron_server::{Host, Hub, HubConfig, Server, ServerConfig};

use crate::ai;
use crate::config::{Paths, PluginConfig, SharedConfig};
use crate::ffi;
use crate::host::ObsHost;
use crate::starter;
use crate::stream_deck::PairAtStart;
use crate::vision::VisionBridge;
use crate::voice::VoiceBridge;

static PLUGIN: RwLock<Option<Arc<Plugin>>> = RwLock::new(None);

/// The running plugin, if loaded.
pub fn current() -> Option<Arc<Plugin>> {
    PLUGIN.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

struct ServerState {
    server: Option<Server>,
    error: Option<String>,
}

pub struct Plugin {
    pub hub: Arc<Hub>,
    pub vision: VisionBridge,
    pub ai: Arc<Assistant>,
    /// Connects to SpacetimeDB only once someone creates or joins a room in the editor.
    pub room: Arc<RoomClient>,
    pub voice: VoiceBridge,
    config: Arc<SharedConfig>,
    server: Mutex<ServerState>,
    urls: Mutex<(String, String)>,
    url_generation: AtomicU64,
    /// The Stream Deck pairing OBS sends once it started (see `pair_at_start`).
    pair_at_start: Mutex<Option<PairAtStart>>,
}

impl Plugin {
    /// Loads the project and config from `config_dir`, then starts the hub and server. A fresh
    /// install starts from the starter setup.
    pub fn start(config_dir: PathBuf) -> Result<Arc<Plugin>, String> {
        let paths = Paths::in_dir(&config_dir);
        let loaded = starter::load_project(&paths);
        if loaded.fresh {
            ffi::info("a fresh install: starting from the starter setup");
        }
        if let Some(warning) = &loaded.warning {
            ffi::warn(warning);
        }
        // Saving over a project that could be neither read nor set aside would destroy it.
        let hub_config = HubConfig {
            project_path: loaded.writable.then(|| paths.project.clone()),
            ..HubConfig::default()
        };
        let hub = Hub::start(loaded.project, hub_config).map_err(|error| error.to_string())?;
        if let Some(warning) = loaded.warning {
            hub.update(|engine, now| engine.note("system", &warning, now));
        }
        let config = Arc::new(SharedConfig::load_or_create(paths.config.clone()));
        let vision = VisionBridge::new(Arc::clone(&hub));
        let ai = ai::start(&paths, &hub, &vision);
        let room = RoomClient::start(Arc::clone(&hub));
        let reached = Arc::downgrade(&room);
        ai.set_room(Box::new(move |step| {
            reached
                .upgrade()
                .map_or_else(|| "the room is closed".to_owned(), |room| room.voice_step(step))
        }));
        let voice = VoiceBridge::default();
        let plugin = Arc::new(Plugin {
            vision,
            ai,
            room,
            voice,
            hub,
            config,
            server: Mutex::new(ServerState {
                server: None,
                error: None,
            }),
            urls: Mutex::new((String::new(), String::new())),
            url_generation: AtomicU64::new(0),
            pair_at_start: Mutex::new(None),
        });
        plugin.start_server();
        plugin.pair_stream_deck_later();
        *PLUGIN.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::clone(&plugin));
        Ok(plugin)
    }

    /// (Re)starts the server with the current config, falling back to a free port if needed.
    ///
    /// The fallback also replaces the editor and overlay tokens: docks and bookmarks still
    /// point at the configured port, so whatever program holds it would otherwise receive a
    /// token that still works here. The control token stays, because control surfaces prove
    /// the server before they send it.
    ///
    /// The config is read only after the old server stopped, and that server may no longer
    /// change the control token, so a revoke it was handling is either in the new server's
    /// token or refused; it is never lost.
    fn start_server(&self) {
        let mut state = self.server.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let serving = self.config.next_server();
        if let Some(old) = state.server.take() {
            old.stop();
        }
        let config = self.config.get();
        let host: Arc<dyn Host> = Arc::new(ObsHost {
            config: Arc::clone(&self.config),
            server: serving,
            hub: Arc::clone(&self.hub),
            preview: Arc::clone(self.vision.preview()),
            voice_owner: self.voice.owner(),
        });
        let attempt = |port, config: &PluginConfig| {
            let server_config = ServerConfig {
                port,
                token: config.token.clone(),
                overlay_token: config.overlay_token.clone(),
                control_token: config.control_token.clone(),
                ..ServerConfig::default()
            };
            Server::start(
                Arc::clone(&self.hub),
                Arc::clone(&host),
                Arc::new(AiService::new(Arc::clone(&self.ai))),
                self.room.clone(),
                server_config,
            )
        };
        let (server, error) = match attempt(config.port, &config) {
            Ok(server) => (Some(server), None),
            Err(error) => {
                let note = format!(
                    "port {} is unavailable ({error}); using a temporary port and a new editor address",
                    config.port
                );
                ffi::warn(&note);
                let config = self.update_config(PluginConfig::regenerate_tokens);
                match attempt(0, &config) {
                    Ok(server) => (Some(server), Some(note)),
                    Err(error) => (None, Some(format!("the editor server could not start: {error}"))),
                }
            }
        };
        if let Some(server) = &server {
            ffi::info(&format!("editor listening on 127.0.0.1:{}", server.port()));
            let urls = (server.editor_url(), server.overlay_url());
            *self.urls.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = urls;
            self.url_generation.fetch_add(1, Ordering::SeqCst);
        }
        state.server = server;
        state.error = error;
    }

    /// Applies a new port and optionally new tokens, then restarts the server.
    pub fn reconfigure(&self, port: u16, regenerate_token: bool) {
        if self.config.get().port == port && !regenerate_token {
            return;
        }
        self.update_config(|config| {
            config.port = port;
            if regenerate_token {
                config.regenerate_tokens();
            }
        });
        self.start_server();
    }

    fn update_config(&self, change: impl FnOnce(&mut PluginConfig)) -> PluginConfig {
        let (config, saved) = self.config.update(change);
        if let Err(error) = saved {
            ffi::warn(&format!("could not save settings: {error}"));
        }
        config
    }

    /// A single-use editor address for opening a browser; empty when the server is down.
    pub fn editor_link(&self) -> String {
        let state = self.server.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.server.as_ref().map(Server::editor_link).unwrap_or_default()
    }

    pub fn port_setting(&self) -> u16 {
        self.config.get().port
    }

    pub fn editor_url(&self) -> String {
        self.urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .0
            .clone()
    }

    pub fn overlay_url(&self) -> String {
        self.urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .1
            .clone()
    }

    /// Changes whenever the overlay URL changes, so overlay sources can refresh cheaply.
    pub fn url_generation(&self) -> u64 {
        self.url_generation.load(Ordering::SeqCst)
    }

    pub fn server_error(&self) -> Option<String> {
        self.server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .error
            .clone()
    }

    /// Stops background work and saves pending changes. Safe to call more than once.
    pub fn stop(&self) {
        self.stop_pairing();
        if let Some(server) = self
            .server
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .server
            .take()
        {
            server.stop();
        }
        // Waits for a model call in flight, so no AI or voice thread outlives the module.
        self.ai.shutdown();
        self.room.shutdown();
        self.vision.shutdown();
        self.hub.shutdown();
    }
}

/// Removes the global plugin and stops it.
pub fn shutdown() {
    let plugin = PLUGIN.write().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    if let Some(plugin) = plugin {
        plugin.stop();
    }
}
