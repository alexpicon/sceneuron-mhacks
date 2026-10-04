// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Exercises the embedded assets over real loopback connections: every script and stylesheet
//! under `web/` is served at `/assets/<path>` with its content type, and so is every font the
//! script fonts stylesheet names, the pages load exactly those files, and unknown, traversal,
//! percent-encoded, and query-suffixed paths are handled without ever reaching the file system.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use common::{Reply, Req, TestServer};

const JAVASCRIPT: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

/// Every `.js` and `.css` file under `web/`, except the Node tests, as a `/`-separated path.
fn web_assets() -> BTreeSet<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let path = format!("{prefix}{name}");
            if entry.file_type().unwrap().is_dir() {
                if path != "tests" {
                    walk(&entry.path(), &format!("{path}/"), found);
                }
            } else if name.ends_with(".js") || name.ends_with(".css") {
                found.insert(path);
            }
        }
    }
    let mut found = BTreeSet::new();
    walk(&web_dir(), "", &mut found);
    found
}

fn web_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web")
}

/// The `/assets/` paths a page loads through `src` and `href` attributes.
fn page_assets(page: &str) -> BTreeSet<String> {
    page.split(['"', '\''])
        .filter_map(|value| value.strip_prefix("/assets/"))
        .map(str::to_owned)
        .collect()
}

fn assert_not_found(reply: &Reply, path: &str) {
    assert_eq!(reply.status, 404, "{path}");
    assert_eq!(reply.header("Content-Type"), Some("application/json"), "{path}");
    assert_eq!(reply.error(), "not found", "{path}");
}

#[test]
fn every_web_script_and_stylesheet_is_served_with_its_type() {
    let server = TestServer::start();
    let assets = web_assets();
    assert!(assets.len() >= 20, "found only {assets:?}");
    for path in &assets {
        let reply = Req::new("GET", &format!("/assets/{path}")).send(server.port);
        assert_eq!(reply.status, 200, "{path}");
        let content_type = if path.ends_with(".js") { JAVASCRIPT } else { CSS };
        assert_eq!(reply.header("Content-Type"), Some(content_type), "{path}");
        assert_eq!(reply.header("X-Content-Type-Options"), Some("nosniff"), "{path}");
        assert_eq!(reply.header("Cache-Control"), Some("no-store"), "{path}");
        assert_eq!(reply.header("Referrer-Policy"), Some("no-referrer"), "{path}");
        assert_eq!(reply.header("Content-Security-Policy"), None, "{path}");
        assert_eq!(reply.body, fs::read(web_dir().join(path)).unwrap(), "{path}");
    }
}

#[test]
fn the_pages_load_every_asset_and_nothing_else() {
    let editor = page_assets(include_str!("../../../web/editor.html"));
    let overlay = page_assets(include_str!("../../../web/overlay.html"));
    assert!(overlay.contains("render-style.js") && overlay.contains("render.js") && overlay.contains("overlay.js"));
    let loaded: BTreeSet<String> = editor.union(&overlay).cloned().collect();
    assert_eq!(loaded, web_assets());
}

#[test]
fn every_font_the_script_fonts_name_is_served_with_its_type_and_nothing_else_of_theirs() {
    let server = TestServer::start();
    let css = include_str!("../../../web/script-fonts.css");
    let named: BTreeSet<String> = css
        .split("url(\"/assets/")
        .skip(1)
        .map(|rest| rest[..rest.find('"').unwrap()].to_owned())
        .collect();
    let on_disk: BTreeSet<String> = fs::read_dir(web_dir().join("fonts"))
        .unwrap()
        .map(|entry| format!("fonts/{}", entry.unwrap().file_name().into_string().unwrap()))
        .filter(|path| [".woff2", ".ttf", ".otf"].iter().any(|kind| path.ends_with(kind)))
        .collect();
    assert_eq!(named, on_disk);
    for path in &named {
        let reply = Req::new("GET", &format!("/assets/{path}")).send(server.port);
        assert_eq!(reply.status, 200, "{path}");
        let kind = match path.rsplit('.').next() {
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            _ => "font/otf",
        };
        assert_eq!(reply.header("Content-Type"), Some(kind), "{path}");
        assert_eq!(reply.header("X-Content-Type-Options"), Some("nosniff"), "{path}");
        assert_eq!(reply.body, fs::read(web_dir().join(path)).unwrap(), "{path}");
    }
    for path in ["fonts/README.md", "fonts/LICENSE-AlcarinTengwar.txt", "fonts/", "fonts"] {
        assert_not_found(&Req::new("GET", &format!("/assets/{path}")).send(server.port), path);
    }
}

#[test]
fn unknown_and_lookalike_asset_paths_are_not_found() {
    let server = TestServer::start();
    let paths = [
        "/assets/",
        "/assets/missing.js",
        "/assets/editor.js",
        "/assets/editor.css",
        "/assets/editor.html",
        "/assets/overlay.html",
        "/assets/editor",
        "/assets/editor/",
        "/assets/editor/panels",
        "/assets/editor/css/",
        "/assets/tests/fake-dom.mjs",
        "/assets/tests/render.test.mjs",
        "/assets/RENDER.JS",
        "/assets/render.js/",
        "/assets/render.js.map",
        "/assets/render.js;",
        "/assets/render.js#top",
        "/assets//render.js",
        "/assets/./render.js",
        "/render.js",
        "/web/render.js",
        "/assets",
    ];
    for path in paths {
        assert_not_found(&Req::new("GET", path).send(server.port), path);
    }
}

#[test]
fn traversal_and_encoded_paths_are_not_found() {
    let server = TestServer::start();
    let paths = [
        "/assets/../Cargo.toml",
        "/assets/../../Cargo.toml",
        "/assets/../../../Cargo.toml",
        "/assets/../web/render.js",
        "/assets/../assets/render.js",
        "/assets/editor/../render.js",
        "/assets/editor/panels/../../render.js",
        "/assets/..%2fCargo.toml",
        "/assets/%2e%2e/Cargo.toml",
        "/assets/%2E%2E%2F%2E%2E%2FCargo.toml",
        "/assets/render%2Ejs",
        "/assets/%72ender.js",
        "/assets/editor%2Fmain.js",
        "/assets/editor%2fmain.js",
        "/assets/editor\\main.js",
        "/assets/render.js%00",
        "/assets/render.js%00.css",
        "/assets/%00",
    ];
    for path in paths {
        assert_not_found(&Req::new("GET", path).send(server.port), path);
    }
}

#[test]
fn a_query_string_never_changes_the_asset() {
    let server = TestServer::start();
    for path in ["render.js", "editor/main.js", "editor/css/base.css"] {
        let body = fs::read(web_dir().join(path)).unwrap();
        for query in [
            "?v=1",
            "?",
            "?../../Cargo.toml",
            "?token=abc&x=%2e%2e",
            "?/assets/missing.js",
        ] {
            let reply = Req::new("GET", &format!("/assets/{path}{query}")).send(server.port);
            assert_eq!(reply.status, 200, "{path}{query}");
            assert_eq!(reply.body, body, "{path}{query}");
        }
    }
    let reply = Req::new("GET", "/assets/missing.js?render.js").send(server.port);
    assert_not_found(&reply, "/assets/missing.js?render.js");
}

#[test]
fn assets_answer_only_get() {
    let server = TestServer::start();
    for method in ["POST", "PUT", "DELETE", "HEAD", "OPTIONS"] {
        let reply = Req::new(method, "/assets/editor/main.js").send(server.port);
        assert_eq!(reply.status, 405, "{method}");
        assert_eq!(reply.header("Allow"), Some("GET"), "{method}");
    }
}
