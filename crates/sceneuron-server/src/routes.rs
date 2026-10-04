// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Request checks and dispatch: Host allowlist, access tokens and their scope, one-time
//! editor links, path, method, and body size.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::access::{Access, access};
use crate::api_control::Answer;
use crate::body::{MAX_BODY, too_large};
use crate::connection::Request;
use crate::endpoints::Endpoint;
use crate::head::Head;
use crate::http::{Reply, query_param, split_target};
use crate::server::Context;
use crate::sse::Feed;
use crate::{api, api_ai, api_camera, api_control, api_room, api_subtitles, api_voice, hello, sse};

enum Handled {
    Reply(Reply),
    Events(Feed),
}

/// Answers one request, or streams events to it until the client or the server leaves.
pub(crate) fn handle(context: &Context, mut request: Request) {
    // A panicking handler answers 500, and its connection closes as usual.
    let handled = catch_unwind(AssertUnwindSafe(|| route(context, &mut request)))
        .unwrap_or_else(|_| Handled::Reply(Reply::error(500, "the request failed")));
    match handled {
        Handled::Reply(reply) => request.connection.send(&reply),
        Handled::Events(feed) => sse::serve(context, request.connection, request.head.version, feed),
    }
}

fn route(context: &Context, request: &mut Request) -> Handled {
    if !host_allowed(context, &request.head) {
        let message = "the Host header must be 127.0.0.1 or localhost with this server's port";
        return Handled::Reply(Reply::error(403, message));
    }
    let target = request.head.target.clone();
    let (path, query) = split_target(&target);
    if path == "/editor"
        && let Some(code) = query_param(query, "code")
    {
        return Handled::Reply(redeem_link(context, &code));
    }
    let access = access(context, &request.head, query);
    let endpoint = Endpoint::find(path);
    // Checked before a missing path's 404, so a request without a token learns nothing about
    // which paths exist.
    if path.starts_with("/api/") && access == Access::None && !endpoint.is_some_and(|found| found.allows(access)) {
        return Handled::Reply(Reply::error(401, "a valid access token is required"));
    }
    let Some(endpoint) = endpoint else {
        return Handled::Reply(Reply::error(404, "not found"));
    };
    if !endpoint.allows(access) {
        let message = match access {
            Access::Control { .. } => {
                "the control token only controls cards, elements, effects, floods, automation, and push to talk"
            }
            _ => "the overlay token can only read the overlay state",
        };
        return Handled::Reply(Reply::error(403, message));
    }
    let methods = endpoint.methods();
    if !methods.contains(&request.head.method.as_str()) {
        let reply = Reply::error(405, "method not allowed").with_header("Allow", methods.join(", "));
        return Handled::Reply(reply);
    }
    // Refused before any of the body is read; the connection closes after the reply.
    if request.head.content_length > MAX_BODY as u64 {
        return Handled::Reply(too_large());
    }
    let hub = &context.hub;
    let host = context.host.as_ref();
    let assistant = context.assistant.as_ref();
    let answer = Answer::for_access(access);
    let result = match endpoint {
        Endpoint::Root if query.is_empty() => Ok(Reply::redirect("/editor".to_owned())),
        Endpoint::Root => Ok(Reply::redirect(format!("/editor?{query}"))),
        Endpoint::File(asset) => Ok(Reply::asset(asset)),
        Endpoint::State => Ok(api::state(hub)),
        Endpoint::Project if request.head.method == "PUT" => api::replace_project(hub, request),
        Endpoint::Project => Ok(api::project(hub)),
        Endpoint::Override => api::set_override(hub, request),
        Endpoint::Release => api::release(hub, request, answer),
        Endpoint::Effect => api::effect(hub, request, answer),
        Endpoint::Flood => api::flood(hub, request, answer),
        Endpoint::Automation => api::automation(hub, request, answer),
        Endpoint::Simulate => api::simulate(hub, request),
        Endpoint::Sources => Ok(api::sources(host)),
        Endpoint::Attach => api::attach(host, request),
        Endpoint::CameraPreview => Ok(api_camera::preview(host)),
        Endpoint::CameraAnalysis => Ok(api_camera::analysis(host)),
        Endpoint::AudioSources => Ok(api_camera::audio_sources(host)),
        Endpoint::AttachVoice => api_camera::attach_voice(host, request),
        Endpoint::AddOverlay => api::add_overlay(host),
        Endpoint::Setup => api_camera::set_up(context),
        Endpoint::OverlayUrl => Ok(Reply::json(200, &serde_json::json!({ "url": context.overlay_url() }))),
        Endpoint::Hello => Ok(hello::hello(context, query_param(query, "nonce"))),
        Endpoint::AiSettings if request.head.method == "PUT" => api_ai::update_settings(assistant, request),
        Endpoint::AiSettings => Ok(api_ai::settings(assistant)),
        Endpoint::AiPlan => api_ai::plan(hub, assistant, request),
        Endpoint::AiAnalyze => api_ai::analyze(assistant),
        Endpoint::Packs => Ok(api_control::packs(hub)),
        Endpoint::Card => api_control::card(hub, request, answer),
        Endpoint::ControlElement => api_control::element(hub, request, answer),
        Endpoint::ControlState => Ok(api_control::state(hub)),
        Endpoint::Pairing if request.head.method == "POST" => Ok(api_control::issue_code(context)),
        Endpoint::Pairing => Ok(api_control::pairings(context)),
        Endpoint::Pair => api_control::pair(context, request),
        Endpoint::Revoke => api_control::revoke(context),
        Endpoint::Room if request.head.method == "POST" => api_room::act(context.room.as_ref(), request, access),
        Endpoint::Room => api_room::state(context.room.as_ref(), access),
        Endpoint::Voice => Ok(api_voice::voice(hub)),
        Endpoint::VoiceTalk => api_voice::talk(hub, request, answer),
        Endpoint::VoiceGuide => Ok(api_voice::guide(hub)),
        Endpoint::VoiceCommand => api_voice::command(hub, request),
        Endpoint::Speech => Ok(api_voice::speech(assistant, path)),
        Endpoint::Subtitles => Ok(api_subtitles::subtitles(hub)),
        Endpoint::SubtitlesSimulate => api_subtitles::simulate(hub, request),
        Endpoint::Events => return Handled::Events(Feed::State),
        Endpoint::ControlEvents => {
            let generation = match access {
                Access::Control { generation } => Some(generation),
                _ => None,
            };
            return Handled::Events(Feed::Control { generation });
        }
    };
    Handled::Reply(result.unwrap_or_else(|error| error))
}

/// Accepts only the loopback names of this port, so a page on another site that
/// rebinds its DNS name to 127.0.0.1 cannot reach the API.
fn host_allowed(context: &Context, head: &Head) -> bool {
    let mut hosts = head.header_values("Host");
    match (hosts.next(), hosts.next()) {
        (Some(host), None) => context.allowed_hosts.iter().any(|allowed| allowed == host),
        _ => false,
    }
}

/// Swaps a one-time editor code for the editor address with its token.
fn redeem_link(context: &Context, code: &str) -> Reply {
    if context.links.redeem(code) {
        Reply::redirect(format!("/editor#token={}", context.token))
    } else {
        let message = "this editor link was already used or has expired; open the editor again from OBS";
        Reply::error(403, message)
    }
}
