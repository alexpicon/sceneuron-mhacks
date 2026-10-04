// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises plugin settings: first-run token creation, persistence, repair of bad values,
//! token regeneration, private, atomic writes, and the shared config that keeps a revoked
//! control token revoked across restarts and refuses a revoke from a replaced server.

use std::fs;
use std::path::PathBuf;

use sceneuron::config::{DEFAULT_PORT, PluginConfig, SharedConfig};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sceneuron-obs-config-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir.join("server.json")
}

#[test]
fn first_run_creates_and_saves_a_token() {
    let path = scratch("first");
    let config = PluginConfig::load_or_create(&path);
    assert_eq!(config.port, DEFAULT_PORT);
    assert_eq!((config.token.len(), config.overlay_token.len()), (32, 32));
    assert_eq!(config.control_token.len(), 32);
    assert_ne!(config.token, config.overlay_token);
    assert!(config.control_token != config.token && config.control_token != config.overlay_token);
    assert_eq!(PluginConfig::load_or_create(&path), config);
}

#[test]
fn keeps_a_valid_saved_config() {
    let path = scratch("keep");
    let saved = PluginConfig {
        port: 5000,
        token: "ab".repeat(16),
        overlay_token: "cd".repeat(16),
        control_token: "ef".repeat(16),
    };
    saved.save(&path).unwrap();
    assert_eq!(PluginConfig::load_or_create(&path), saved);
    assert!(!path.with_file_name("server.json.tmp").exists());
}

#[test]
fn repairs_a_weak_token_or_privileged_port() {
    let path = scratch("repair");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, r#"{"port": 80, "token": "short"}"#).unwrap();
    let config = PluginConfig::load_or_create(&path);
    assert_eq!(config.port, DEFAULT_PORT);
    assert_eq!(config.token.len(), 32);
}

#[test]
fn replaces_a_token_the_server_would_reject() {
    let path = scratch("long");
    let long = PluginConfig {
        port: 5000,
        token: "a".repeat(129),
        overlay_token: "b".repeat(32),
        control_token: "c".repeat(32),
    };
    long.save(&path).unwrap();
    let config = PluginConfig::load_or_create(&path);
    assert_eq!((config.port, config.token.len()), (5000, 32));
}

#[test]
fn unreadable_file_falls_back_to_defaults() {
    let path = scratch("garbage");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "not json").unwrap();
    assert_eq!(PluginConfig::load_or_create(&path).port, DEFAULT_PORT);
}

#[test]
fn a_missing_or_reused_overlay_token_is_replaced() {
    let path = scratch("overlay");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let token = "ab".repeat(16);
    fs::write(
        &path,
        format!(r#"{{"port": 5000, "token": "{token}", "overlay_token": "{token}"}}"#),
    )
    .unwrap();
    let config = PluginConfig::load_or_create(&path);
    assert_eq!(config.token, token);
    assert_ne!(config.overlay_token, token);
    assert_eq!(config.overlay_token.len(), 32);
}

#[test]
fn a_missing_or_reused_control_token_is_replaced() {
    let path = scratch("control");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let (token, overlay) = ("ab".repeat(16), "cd".repeat(16));
    for control in [String::new(), "short".to_owned(), token.clone(), overlay.clone()] {
        let text = format!(r#"{{"token": "{token}", "overlay_token": "{overlay}", "control_token": "{control}"}}"#);
        fs::write(&path, text).unwrap();
        let config = PluginConfig::load_or_create(&path);
        assert_eq!(
            (config.token.as_str(), config.overlay_token.as_str()),
            (token.as_str(), overlay.as_str())
        );
        assert_eq!(config.control_token.len(), 32, "{control:?}");
        assert!(config.control_token != token && config.control_token != overlay);
        assert_eq!(PluginConfig::load_or_create(&path), config, "the new token was saved");
    }
}

#[test]
fn regenerating_replaces_both_tokens() {
    let mut config = PluginConfig::load_or_create(&scratch("regenerate"));
    let before = config.clone();
    config.regenerate_tokens();
    assert_ne!(config.token, before.token);
    assert_ne!(config.overlay_token, before.overlay_token);
    assert_eq!(config.port, before.port);
    assert_eq!(
        config.control_token, before.control_token,
        "paired Stream Decks keep working"
    );
}

#[test]
fn a_saved_control_token_survives_a_restart() {
    let path = scratch("shared");
    let shared = SharedConfig::load_or_create(path.clone());
    let before = shared.get();
    let token = "0123456789abcdef".repeat(2);
    shared.set_control_token(shared.next_server(), &token).unwrap();
    let (updated, saved) = shared.update(|config| config.port = 5001);
    saved.unwrap();
    assert_eq!((updated.port, updated.control_token.as_str()), (5001, token.as_str()));
    let restarted = PluginConfig::load_or_create(&path);
    assert_eq!(restarted.control_token, token);
    assert_eq!(restarted.port, 5001);
    assert_eq!(
        (restarted.token, restarted.overlay_token),
        (before.token, before.overlay_token)
    );
}

#[test]
fn a_control_token_that_cannot_be_saved_is_not_used() {
    let blocker = scratch("unsaved").with_file_name("blocker");
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, "a file where a directory should be").unwrap();
    let shared = SharedConfig::load_or_create(blocker.join("server.json"));
    let before = shared.get();
    let refused = shared.set_control_token(shared.next_server(), &"fe".repeat(16));
    assert!(refused.is_err_and(|error| error.starts_with("could not save settings")));
    assert_eq!(shared.get(), before);
    let (updated, saved) = shared.update(|config| config.port = 5002);
    assert!(saved.is_err());
    assert_eq!(updated.port, 5002, "other changes still apply until a restart");
}

#[test]
fn a_replaced_server_cannot_change_the_control_token() {
    let path = scratch("replaced");
    let shared = SharedConfig::load_or_create(path.clone());
    let old = shared.next_server();
    let new = shared.next_server();
    let before = shared.get();
    let refused = shared.set_control_token(old, &"fe".repeat(16));
    assert!(refused.is_err_and(|error| error.contains("restarting")));
    assert_eq!(shared.get(), before);
    assert_eq!(PluginConfig::load_or_create(&path), before, "nothing was saved");
    shared.set_control_token(new, &"fe".repeat(16)).unwrap();
    assert_eq!(shared.get().control_token, "fe".repeat(16));
}

#[test]
fn the_file_and_its_directory_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let path = scratch("private");
    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    PluginConfig::load_or_create(&path);
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    PluginConfig::load_or_create(&path);
    assert_eq!(mode(&path), 0o600, "an existing readable file is made private");
}
