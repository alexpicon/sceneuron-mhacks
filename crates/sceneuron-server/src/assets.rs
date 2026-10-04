// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! The editor and overlay pages and their scripts, styles, and fonts, embedded at build time.
//! Nothing is read from disk at runtime: a request names an asset by its exact path in
//! [`ASSETS`], and every other path is not found.

/// Pages may load scripts, styles, fonts, connections, and audio only from this server, and
/// images only from `data:` URLs and from `blob:` URLs, which hold the camera preview the editor
/// fetched from this server; nothing may frame them.
pub(crate) const PAGE_POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
     font-src 'self'; connect-src 'self'; media-src 'self'; img-src data: blob:; base-uri 'none'; \
     form-action 'none'; frame-ancestors 'none'";

const HTML: &str = "text/html; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const WOFF2: &str = "font/woff2";
const TTF: &str = "font/ttf";
const OTF: &str = "font/otf";

/// An embedded file served as is.
#[derive(Debug)]
pub(crate) struct Asset {
    /// The file's path under the repository's `web/` directory.
    pub(crate) path: &'static str,
    pub(crate) body: &'static [u8],
    pub(crate) content_type: &'static str,
    /// HTML pages carry the content security policy.
    pub(crate) page: bool,
}

/// Embeds `web/<path>`, with the content type and page flag that its extension implies.
macro_rules! embed {
    ($path:literal) => {
        Asset {
            path: $path,
            body: include_bytes!(concat!("../../../web/", $path)),
            content_type: content_type($path),
            page: ends_with($path, ".html"),
        }
    };
}

pub(crate) static EDITOR_PAGE: Asset = embed!("editor.html");

pub(crate) static OVERLAY_PAGE: Asset = embed!("overlay.html");

/// Every script, stylesheet, and font the pages load, served at `/assets/<path>`.
static ASSETS: &[Asset] = &[
    embed!("editor/ai-text.js"),
    embed!("editor/api.js"),
    embed!("editor/app.js"),
    embed!("editor/camera-text.js"),
    embed!("editor/cards-text.js"),
    embed!("editor/condition-edits.js"),
    embed!("editor/consent-text.js"),
    embed!("editor/constants.js"),
    embed!("editor/css/ai.css"),
    embed!("editor/css/assistant.css"),
    embed!("editor/css/base.css"),
    embed!("editor/css/cards.css"),
    embed!("editor/css/conditions.css"),
    embed!("editor/css/layout.css"),
    embed!("editor/css/notifications.css"),
    embed!("editor/css/setup.css"),
    embed!("editor/css/side.css"),
    embed!("editor/css/subtitles.css"),
    embed!("editor/css/theme.css"),
    embed!("editor/css/voice.css"),
    embed!("editor/css/workspace.css"),
    embed!("editor/deck-edits.js"),
    embed!("editor/dom.js"),
    embed!("editor/drag.js"),
    embed!("editor/language-text.js"),
    embed!("editor/languages.js"),
    embed!("editor/main.js"),
    embed!("editor/mic-text.js"),
    embed!("editor/panels/ai-fields.js"),
    embed!("editor/panels/ai-settings.js"),
    embed!("editor/panels/ai-status.js"),
    embed!("editor/panels/assistant.js"),
    embed!("editor/panels/camera-fields.js"),
    embed!("editor/panels/camera-preview.js"),
    embed!("editor/panels/camera.js"),
    embed!("editor/panels/cards.js"),
    embed!("editor/panels/condition-cards.js"),
    embed!("editor/panels/conditions.js"),
    embed!("editor/panels/deck-cards.js"),
    embed!("editor/panels/deck-voice.js"),
    embed!("editor/panels/elements.js"),
    embed!("editor/panels/events.js"),
    embed!("editor/panels/live.js"),
    embed!("editor/panels/microphones.js"),
    embed!("editor/panels/notifications.js"),
    embed!("editor/panels/packs.js"),
    embed!("editor/panels/placeholder-chips.js"),
    embed!("editor/panels/plan-session.js"),
    embed!("editor/panels/plan-view.js"),
    embed!("editor/panels/preview.js"),
    embed!("editor/panels/properties.js"),
    embed!("editor/panels/property-fields.js"),
    embed!("editor/panels/reactions.js"),
    embed!("editor/panels/room-markup.js"),
    embed!("editor/panels/room.js"),
    embed!("editor/panels/rule-cards.js"),
    embed!("editor/panels/rules.js"),
    embed!("editor/panels/settings.js"),
    embed!("editor/panels/settings-transfer.js"),
    embed!("editor/panels/setup-wizard.js"),
    embed!("editor/panels/status.js"),
    embed!("editor/panels/stream-deck.js"),
    embed!("editor/panels/subtitles-fields.js"),
    embed!("editor/panels/subtitles.js"),
    embed!("editor/panels/tabs.js"),
    embed!("editor/panels/theme.js"),
    embed!("editor/panels/voice-fields.js"),
    embed!("editor/panels/voice-guide.js"),
    embed!("editor/panels/voice.js"),
    embed!("editor/placeholders.js"),
    embed!("editor/plan-text.js"),
    embed!("editor/project.js"),
    embed!("editor/reactions-text.js"),
    embed!("editor/room-text.js"),
    embed!("editor/settings-file.js"),
    embed!("editor/signals-context.js"),
    embed!("editor/signals-text.js"),
    embed!("editor/snapshot.js"),
    embed!("editor/stream-deck-text.js"),
    embed!("editor/subtitles-text.js"),
    embed!("editor/sync.js"),
    embed!("editor/triggers.js"),
    embed!("editor/values.js"),
    embed!("editor/voice-guide-text.js"),
    embed!("editor/voice-text.js"),
    embed!("flood-banner.js"),
    embed!("flood-burst.js"),
    embed!("fonts/AlcarinTengwarVF.woff2"),
    embed!("fonts/nasin-nanpa-4.0.2-UCSUR.otf"),
    embed!("fonts/pIqaD-qolqoS.ttf"),
    embed!("flood.js"),
    embed!("icons.js"),
    embed!("link.js"),
    embed!("overlay.js"),
    embed!("reaction-presets.js"),
    embed!("render-cards.js"),
    embed!("render-clock.js"),
    embed!("render-fit.js"),
    embed!("render-icons.js"),
    embed!("render-results.js"),
    embed!("render-style.js"),
    embed!("render-subtitles.js"),
    embed!("render-themes.js"),
    embed!("render.js"),
    embed!("script-fonts.css"),
    embed!("speech.js"),
    embed!("themes/classic-mac.js"),
    embed!("themes/eighties.js"),
    embed!("themes/graphite.js"),
    embed!("themes/hacker.js"),
    embed!("themes/vaporwave.js"),
    embed!("themes/windows-95.js"),
];

/// Finds a file under `/assets/` by its exact path. Paths are never decoded or
/// normalized, so `..` segments, percent escapes, and other spellings match nothing.
pub(crate) fn find(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|asset| asset.path == path)
}

/// The content type for a file's extension. Any other extension fails the build.
const fn content_type(path: &str) -> &'static str {
    if ends_with(path, ".js") {
        JAVASCRIPT
    } else if ends_with(path, ".css") {
        CSS
    } else if ends_with(path, ".html") {
        HTML
    } else if ends_with(path, ".woff2") {
        WOFF2
    } else if ends_with(path, ".ttf") {
        TTF
    } else if ends_with(path, ".otf") {
        OTF
    } else {
        panic!("embedded files must be .js, .css, .html, .woff2, .ttf, or .otf")
    }
}

const fn ends_with(text: &str, suffix: &str) -> bool {
    let (text, suffix) = (text.as_bytes(), suffix.as_bytes());
    if suffix.len() > text.len() {
        return false;
    }
    let start = text.len() - suffix.len();
    let mut index = 0;
    while index < suffix.len() {
        if text[start + index] != suffix[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn asset_paths_are_unique_relative_and_plain() {
        let mut seen = HashSet::new();
        for asset in ASSETS {
            assert!(seen.insert(asset.path), "{} is listed twice", asset.path);
            let plain = |segment: &str| !segment.is_empty() && segment != "." && segment != "..";
            assert!(asset.path.split('/').all(plain), "{}", asset.path);
            assert!(!asset.path.contains(['\\', '%', '?', '#']), "{}", asset.path);
            assert!(!asset.page, "{} is a page, which is served at its own path", asset.path);
            assert!(find(asset.path).is_some_and(|found| std::ptr::eq(found, asset)));
        }
    }

    #[test]
    fn extensions_choose_the_content_type() {
        assert_eq!(content_type("a.js"), JAVASCRIPT);
        assert_eq!(content_type("dir/a.css"), CSS);
        assert_eq!(content_type("a.html"), HTML);
        assert_eq!(content_type("fonts/a.woff2"), WOFF2);
        assert_eq!(content_type("fonts/a.ttf"), TTF);
        assert_eq!(content_type("fonts/a.otf"), OTF);
        assert!(ends_with("editor/main.js", ".js"));
        assert!(!ends_with("main.json", ".js"));
        assert!(!ends_with("js", ".js"));
        assert!(EDITOR_PAGE.page && OVERLAY_PAGE.page);
        assert_eq!(EDITOR_PAGE.content_type, HTML);
    }
}
