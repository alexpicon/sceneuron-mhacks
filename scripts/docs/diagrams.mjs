// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

import { writeDiagrams } from "./style.mjs";

const n = (title, ...body) => ({ title, body, tone: "amber" });
const withTone = (tone, title, ...body) => ({ title, body, tone });
const flow = (name, title, subtitle, items, note, tone = "amber") =>
  ({ name, title, subtitle, items, note, tone, kind: "flow" });
const grid = (name, title, subtitle, items, note, tone = "violet") =>
  ({ name, title, subtitle, items, note, tone, kind: "grid" });
const lanes = (name, title, subtitle, items, note, tone = "amber") =>
  ({ name, title, subtitle, items, note, tone, kind: "lanes" });
const hub = (name, title, subtitle, center, items, note, tone = "amber") =>
  ({ name, title, subtitle, center, items, note, tone, kind: "hub" });

writeDiagrams([
  grid("docs-map", "Find the right guide", "One subject per page, from first use to owner review.", [
    withTone("amber", "Product", "Professional call flows", "and creative use"),
    withTone("violet", "Install", "Build both plugins", "and pair them"),
    withTone("cyan", "Design", "State, trust, and", "component boundaries"),
    withTone("green", "Develop", "Locked builds and", "local checks"),
    withTone("rose", "API", "Token scopes and", "control routes"),
    withTone("amber", "Verify", "App logs, hardware,", "and owner acceptance"),
  ], "The pitch, backlog, and provenance pages complete the map."),

  flow("professional-flow", "One clear signal in the call", "The primary path serves meetings, planning, and classes.", [
    n("A person", "Needs to signal", "without interrupting"),
    withTone("violet", "Sceneuron", "Raises the chosen", "card or caption"),
    withTone("cyan", "OBS output", "Composes the", "person and overlay"),
    withTone("green", "The call", "Teammates see one", "video feed"),
  ], "Streamers can use the same canvas for creative scenes."),

  flow("card-lifecycle", "One card, every control", "The card state is shared across inputs and displays.", [
    withTone("violet", "Configure", "Choose a deck,", "card, and rules"),
    n("Raise", "Key, editor, hotkey,", "voice, or camera"),
    withTone("cyan", "Show", "Overlay, editor,", "and key agree"),
    withTone("green", "Lower", "Press again or", "let time expire"),
  ], "The input changes live state; it does not silently edit the layout."),

  flow("provenance", "From plan to test", "The project owner's account of roles in this build.", [
    withTone("violet", "Alexander + Fable", "PLAN.md in UltraCode", "about two hours"),
    n("GPT-6-Astra", "Ultra effort and", "ultrafast build"),
    withTone("green", "Owner test", "Installed apps", "before a commit"),
  ], "The plan was complete before the implementation began."),

  hub("system-map", "One engine, several hosts", "Each component has one job and one path to shared state.",
    { title: "Core engine", body: "Project, rules, cards, state" }, [
      withTone("violet", "OBS host", "Sources, filters, hotkeys"),
      withTone("cyan", "Browser views", "Editor and overlay"),
      withTone("green", "Stream Deck", "Limited paired controls"),
      withTone("rose", "Room client", "SpacetimeDB updates"),
    ], "Optional AI and voice services attach through the host.", "amber"),

  flow("state-flow", "A change reaches every view", "Inputs converge before the UI or a device redraws.", [
    withTone("violet", "Input", "Key, editor, voice,", "camera, or hotkey"),
    n("Engine", "Apply one state", "change"),
    withTone("cyan", "Events", "Publish the new", "limited views"),
    withTone("green", "Displays", "Overlay and keys", "show the result"),
  ], "A saved project is separate from a momentary live action."),

  lanes("data-boundaries", "Three scopes, three jobs", "A display and a key never receive editor rights.", [
    withTone("violet", "Editor token", "Full local API", "Project and settings", "Held by trusted editor"),
    withTone("cyan", "Overlay token", "Read state and events", "Play prepared audio", "No project edits"),
    withTone("amber", "Control token", "Cards and selected acts", "Limited control view", "Header only"),
  ], "The HTTP server listens on 127.0.0.1 and checks Host."),

  flow("room-vote", "Votes stay hidden until reveal", "The room module separates public progress from private values.", [
    withTone("violet", "Join", "Two people enter", "the same room"),
    n("Vote", "Others see that a", "ballot exists"),
    withTone("cyan", "Reveal", "Names, values, and", "average appear"),
    withTone("green", "Next round", "Lower cards and", "clear votes"),
  ], "The private vote table is not exposed before reveal."),

  lanes("build-map", "Three build products", "Locked inputs lead to independent install artifacts.", [
    withTone("amber", "Rust workspace", "OBS dynamic library", "Notices and bundle", "sceneuron.plugin"),
    withTone("violet", "Node package", "Stream Deck plugin", "Manifest and profiles", ".streamDeckPlugin"),
    withTone("cyan", "Room module", "Separate Cargo lock", "WebAssembly module", "SpacetimeDB"),
  ], "Run builds from the repository root; keep temporary work in ~/src/tmp."),

  flow("test-ladder", "Confidence grows in layers", "A passing test suite does not replace a physical call.", [
    withTone("violet", "Unit tests", "Rust, JavaScript,", "and Stream Deck"),
    withTone("cyan", "Local flows", "Browser, room,", "fake device"),
    n("Installed apps", "OBS loads and the", "Deck pairs"),
    withTone("green", "Owner", "Camera, mic, keys,", "and call output"),
  ], "Re-run affected checks after every source change."),

  lanes("install-paths", "Two installs, one system", "Build each plugin, then connect them on this Mac.", [
    withTone("amber", "OBS path", "Release library", "Signed user bundle", "Local editor server"),
    withTone("violet", "Stream Deck path", "Locked npm build", "Packaged plugin", "Device profile"),
  ], "Pair once after both apps are running."),

  flow("pairing", "Pair without sharing editor access", "A one-time code becomes a limited local control token.", [
    n("OBS editor", "Issues a short-lived", "pairing code"),
    withTone("violet", "Deck app", "Trades the code", "on loopback"),
    withTone("cyan", "Control token", "Stored by the local", "plugin only"),
    withTone("green", "Live keys", "Raise cards and", "follow events"),
  ], "Revocation ends old streams and requires a fresh pair."),

  lanes("api-scopes", "Local API access", "The path is open only to the scope it needs.", [
    withTone("violet", "Full", "Editor and project", "AI settings and setup", "All API routes"),
    withTone("cyan", "Read only", "Overlay state", "Subtitles and events", "Prepared speech audio"),
    withTone("amber", "Limited control", "Cards and elements", "Room reveal and next", "Push to talk hold"),
  ], "Control tokens are accepted in X-Sceneuron-Token, never a URL."),

  flow("api-pairing", "Short code, lasting boundary", "A control surface proves the server before it sends a token.", [
    n("Issue", "Editor requests a", "one-time code"),
    withTone("violet", "Trade", "Local app receives", "control access"),
    withTone("cyan", "Follow", "Limited events keep", "keys current"),
    withTone("rose", "Revoke", "Old token and its", "streams stop"),
  ], "An unpaired browser page cannot trade a code."),

  flow("verification", "From source to installed proof", "Each layer answers a different question.", [
    withTone("violet", "Source", "Format, tests,", "lint, licenses"),
    withTone("cyan", "Packages", "Validate the two", "plugin artifacts"),
    n("Apps", "OBS loads; Deck", "starts and pairs"),
    withTone("green", "Person", "Uses camera, mic,", "keys, and video"),
  ], "Only the owner accepts the final commit candidate."),

  grid("manual-gate", "The hands-on gate", "A person checks what simulation cannot prove.", [
    withTone("amber", "Camera", "Preview and chosen", "source in OBS"),
    withTone("violet", "Microphone", "Consent and push", "to talk behavior"),
    withTone("cyan", "Physical Deck", "Profile, key press,", "and pairing"),
    withTone("green", "Call output", "Cards and captions", "remain readable"),
  ], "The apps stay installed for the owner's test."),

  flow("release-gates", "A commit follows a tested build", "Current checks lead to a person, then to a release decision.", [
    withTone("violet", "Automated", "All affected suites", "pass again"),
    withTone("cyan", "Installed", "OBS and Deck load", "and pair"),
    n("Owner test", "Use real input and", "video output"),
    withTone("green", "Commit", "Review files only", "after acceptance"),
  ], "Provider live tests and public distribution have separate gates."),

  flow("decision", "One accountable decision", "Evidence stays distinct from the owner's judgment.", [
    withTone("cyan", "Evidence", "Builds, tests,", "and app logs"),
    n("Review", "Physical workflow", "and open limits"),
    withTone("green", "Accept", "Commit or return", "to fixes"),
  ], "Passing automation is necessary, but not the final approval."),

  flow("demo-story", "A professional-first demo", "Show a useful call before the creative extension.", [
    n("Meeting", "Quiet signal from", "one key"),
    withTone("violet", "Estimate", "Hidden votes then", "a shared reveal"),
    withTone("cyan", "Caption", "Readable speech", "in the video"),
    withTone("green", "Creative", "Theme and reaction", "for a stream"),
  ], "One OBS output carries every step."),

  hub("demo-controls", "Different inputs, one picture", "The audience sees the composed result, not the control surface.",
    { title: "Live state", body: "One project in the engine" }, [
      withTone("violet", "Editor", "Layout and manual acts"),
      withTone("cyan", "Stream Deck", "Cards, room, and voice"),
      withTone("green", "Hotkeys", "Fast OBS controls"),
      withTone("rose", "Camera / voice", "Optional signals"),
    ], "OBS puts the current overlay into the call video.", "amber"),

  flow("readme-hero", "A clearer way to take part", "Professional calls get visible signals without interruption.", [
    n("Need", "A question, vote,", "or short caption"),
    withTone("violet", "Control", "Key, editor,", "hotkey, or voice"),
    withTone("cyan", "Compose", "Sceneuron adds it", "inside OBS"),
    withTone("green", "Share", "The call sees one", "video output"),
  ], "Meeting and classroom use comes first; streaming is a creative extension."),

  hub("readme-system", "One Sceneuron, many surfaces", "Shared live state keeps the browser and devices in sync.",
    { title: "Sceneuron", body: "Cards, rules, current view" }, [
      withTone("violet", "OBS", "Camera, audio, output"),
      withTone("cyan", "Browser", "Edit and display"),
      withTone("green", "Stream Deck", "Paired key actions"),
      withTone("rose", "Shared room", "Private then revealed"),
    ], "Optional AI and voice require their own settings and consent.", "amber"),

  flow("readme-checks", "From build to owner test", "Each step protects the final call workflow.", [
    withTone("violet", "Locked inputs", "Cargo and npm", "versions fixed"),
    withTone("cyan", "Suites", "Rust, browser,", "room, and Deck"),
    n("Two plugins", "OBS loads; Deck", "pairs on loopback"),
    withTone("green", "Owner", "Tests the real", "camera and keys"),
  ], "The commit waits until the owner accepts the installed result."),
]);
