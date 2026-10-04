// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Where the rooms live: a SpacetimeDB server's address and the database that holds them.

/// The server `spacetime start` runs on this computer.
pub const DEFAULT_SERVER: &str = "http://127.0.0.1:3000";

/// The database name `room/module` is published under.
pub const DEFAULT_DATABASE: &str = "sceneuron-room";

const MAX_SERVER_CHARS: usize = 200;
const MAX_DATABASE_CHARS: usize = 64;

/// A SpacetimeDB server and database, checked so the client can always use them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// `http`, `https`, `ws`, or `wss`, a host with an optional port, and an optional path.
    pub server: String,
    /// Lowercase letters, digits, and dashes, as SpacetimeDB names databases.
    pub database: String,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_owned(),
            database: DEFAULT_DATABASE.to_owned(),
        }
    }
}

impl Target {
    /// The target with these fields, taking the defaults for missing or blank ones.
    pub fn new(server: Option<&str>, database: Option<&str>) -> Result<Target, String> {
        let server = match server.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => check_server(text)?,
            None => DEFAULT_SERVER.to_owned(),
        };
        let database = match database.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => check_database(text)?,
            None => DEFAULT_DATABASE.to_owned(),
        };
        Ok(Target { server, database })
    }
}

fn check_server(text: &str) -> Result<String, String> {
    let refused = || {
        format!(
            "the server must be an address such as {DEFAULT_SERVER}, with http, https, ws, or wss, \
             and no spaces, user name, query, or fragment; got {text:?}"
        )
    };
    if text.chars().count() > MAX_SERVER_CHARS {
        return Err(format!(
            "the server address must be at most {MAX_SERVER_CHARS} characters"
        ));
    }
    let (scheme, rest) = text.split_once("://").ok_or_else(refused)?;
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https" | "ws" | "wss") {
        return Err(refused());
    }
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let allowed = |c: char| c.is_ascii_alphanumeric() || "-._~:[]".contains(c);
    let path_allowed = |c: char| c.is_ascii_alphanumeric() || "-._~/".contains(c);
    if authority.is_empty() || !authority.chars().all(allowed) || !path.chars().all(path_allowed) {
        return Err(refused());
    }
    let path = path.trim_end_matches('/');
    Ok(if path.is_empty() {
        format!("{scheme}://{authority}")
    } else {
        format!("{scheme}://{authority}/{path}")
    })
}

fn check_database(text: &str) -> Result<String, String> {
    let valid = text.len() <= MAX_DATABASE_CHARS
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !text.starts_with('-')
        && !text.ends_with('-');
    if valid {
        Ok(text.to_owned())
    } else {
        Err(format!(
            "the database must be 1 to {MAX_DATABASE_CHARS} lowercase letters, digits, and dashes, \
             such as {DEFAULT_DATABASE}; got {text:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_fields_take_the_defaults() {
        assert_eq!(Target::new(None, None), Ok(Target::default()));
        assert_eq!(Target::new(Some("  "), Some("")), Ok(Target::default()));
    }

    #[test]
    fn servers_keep_their_scheme_host_port_and_path() {
        let cases = [
            ("http://127.0.0.1:3000/", "http://127.0.0.1:3000"),
            ("HTTPS://maincloud.spacetimedb.com", "https://maincloud.spacetimedb.com"),
            ("ws://[::1]:3000", "ws://[::1]:3000"),
            ("wss://example.com/db/", "wss://example.com/db"),
        ];
        for (given, expected) in cases {
            assert_eq!(Target::new(Some(given), None).unwrap().server, expected, "{given}");
        }
    }

    #[test]
    fn servers_the_client_cannot_use_are_refused() {
        for server in [
            "127.0.0.1:3000",
            "ftp://example.com",
            "http://",
            "http://user@example.com",
            "http://example.com/?db=x",
            "http://example.com/#x",
            "http://exa mple.com",
            &format!("http://{}.com", "a".repeat(200)),
        ] {
            assert!(Target::new(Some(server), None).is_err(), "{server}");
        }
    }

    #[test]
    fn databases_follow_spacetimedb_names() {
        assert_eq!(Target::new(None, Some("my-room-2")).unwrap().database, "my-room-2");
        for database in ["Rooms", "-rooms", "rooms-", "room_s", "rooms!", &"a".repeat(65)] {
            assert!(Target::new(None, Some(database)).is_err(), "{database}");
        }
    }
}
