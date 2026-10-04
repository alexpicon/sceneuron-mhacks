// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Where a target's WebSocket is: the server's address mapped to `ws` or `wss`, its host and port
//! for connecting, and the database's subscribe path, `/v1/database/{name}/subscribe`.

use crate::target::Target;

/// Where a target's subscribe endpoint is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Endpoint {
    pub(super) url: String,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) secure: bool,
}

/// The subscribe endpoint of `target`; [`Target`] has already checked the server's form.
pub(super) fn endpoint(target: &Target) -> Result<Endpoint, String> {
    let bad = || format!("cannot read the server address {:?}", target.server);
    let (scheme, rest) = target.server.split_once("://").ok_or_else(bad)?;
    let secure = matches!(scheme, "https" | "wss");
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, ""), |(authority, path)| (authority, path));
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => {
            let (host, after) = bracketed.split_once(']').ok_or_else(bad)?;
            (host, after.strip_prefix(':'))
        }
        None => match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    let port = match port {
        Some(port) => port.parse().map_err(|_| bad())?,
        None if secure => 443,
        None => 80,
    };
    if host.is_empty() {
        return Err(bad());
    }
    let path = if path.is_empty() {
        String::new()
    } else {
        format!("/{path}")
    };
    let socket_scheme = if secure { "wss" } else { "ws" };
    Ok(Endpoint {
        url: format!(
            "{socket_scheme}://{authority}{path}/v1/database/{}/subscribe",
            target.database
        ),
        host: host.to_owned(),
        port,
        secure,
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint_of(server: &str) -> Endpoint {
        endpoint(&Target::new(Some(server), Some("rooms")).unwrap()).unwrap()
    }

    #[test]
    fn plain_and_secure_servers_map_to_ws_and_wss() {
        let local = endpoint_of("http://127.0.0.1:3000");
        assert_eq!(local.url, "ws://127.0.0.1:3000/v1/database/rooms/subscribe");
        assert_eq!(
            (local.host.as_str(), local.port, local.secure),
            ("127.0.0.1", 3000, false)
        );
        let cloud = endpoint_of("https://maincloud.spacetimedb.com");
        assert_eq!(cloud.url, "wss://maincloud.spacetimedb.com/v1/database/rooms/subscribe");
        assert_eq!((cloud.port, cloud.secure), (443, true));
        assert_eq!(endpoint_of("ws://example.com").port, 80);
        assert!(endpoint_of("wss://example.com:8443").secure);
    }

    #[test]
    fn ipv6_hosts_and_paths_are_kept() {
        let v6 = endpoint_of("ws://[::1]:3000/proxy/stdb");
        assert_eq!(v6.url, "ws://[::1]:3000/proxy/stdb/v1/database/rooms/subscribe");
        assert_eq!((v6.host.as_str(), v6.port), ("::1", 3000));
    }

    #[test]
    fn ports_that_are_not_numbers_are_refused() {
        let target = Target::new(Some("http://example.com:http"), None).unwrap();
        assert!(endpoint(&target).is_err());
    }
}
