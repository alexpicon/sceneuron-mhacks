// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Command-line options.

use std::ffi::OsString;
use std::path::PathBuf;

pub(crate) const DEFAULT_PORT: u16 = 4466;

pub(crate) const USAGE: &str = "\
Usage: sceneuron-dev [--port N] [--token T] [--control-token T] [--project PATH]
                     [--demo-presence] [--stream-deck-links PATH]
                     [--demo-ai | --ai [--ai-settings PATH] [--env-file PATH]]
                     [--voice-wav PATH [--voice-url URL]]

Runs the Sceneuron editor, overlay, and engine without OBS.

Options:
  --port N          Port on 127.0.0.1 (default 4466; 0 picks a free port)
  --token T         Access token (default: a new random token)
  --control-token T Control token for Stream Decks and other control surfaces
                    (default: a new random token, kept only while running)
  --project PATH    Load and save the project here (default: keep it in memory).
                    A new file starts from the starter setup, and so does a new
                    --ai-settings file with it
  --demo-presence   Simulate a camera: present for 8 s, away for 8 s, repeating
  --stream-deck-links PATH
                    Simulate Sceneuron's Stream Deck plugin installed: Set up OBS
                    for me writes each pairing link, code included, to PATH (a
                    new line each, the file readable only by you) instead of
                    opening it. Without it, the plugin is not installed
  --demo-ai         Simulate the AI: keyword-based plans and a scripted camera for
                    scene understanding; nothing leaves this computer
  --ai              Use the real AI: the assistant calls the configured models
                    (there is no camera here, so scene understanding stays idle)
  --ai-settings PATH  AI settings file for --ai (default .sceneuron/ai.json)
  --env-file PATH   Optional keys for --ai and --voice-wav as NAME=value lines
                    (default .env); see .env.example. Every key is optional.
  --voice-wav PATH  Speak a WAV file to voice control once, as if push to talk
                    were held while it plays. Its audio, and the wake word and
                    the names and trigger phrases of cards that voice can raise
                    (as keyterms), go to ElevenLabs speech to text, which needs
                    SCENEURON_ELEVENLABS_API_KEY and spends credits. With --ai,
                    voice control or subtitles must be on in the AI settings;
                    with subtitles on, the words become subtitles too, and
                    final lines go to the AI provider when they have targets
  --voice-url URL   Speech-to-text WebSocket for --voice-wav (default: ElevenLabs);
                    ws:// only for a test server on this computer
  -h, --help        Show this help
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub(crate) port: u16,
    pub(crate) token: Option<String>,
    pub(crate) control_token: Option<String>,
    pub(crate) project: Option<PathBuf>,
    pub(crate) demo_presence: bool,
    /// Where pairing links for the simulated Stream Deck plugin go; `None` when it is not installed.
    pub(crate) stream_deck_links: Option<PathBuf>,
    pub(crate) demo_ai: bool,
    pub(crate) ai: bool,
    pub(crate) ai_settings: PathBuf,
    pub(crate) env_file: PathBuf,
    pub(crate) voice_wav: Option<PathBuf>,
    pub(crate) voice_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Run(Options),
    Help,
}

/// Parses the arguments after the program name; a repeated option keeps its last value.
pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut options = Options {
        port: DEFAULT_PORT,
        token: None,
        control_token: None,
        project: None,
        demo_presence: false,
        stream_deck_links: None,
        demo_ai: false,
        ai: false,
        ai_settings: PathBuf::from(".sceneuron/ai.json"),
        env_file: PathBuf::from(".env"),
        voice_wav: None,
        voice_url: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let text = arg.to_str().ok_or_else(|| format!("unexpected argument {arg:?}"))?;
        let (flag, inline) = match text.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(OsString::from(value))),
            _ => (text, None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag {
            "-h" | "--help" => return Ok(Command::Help),
            "--port" => options.port = parse_port(&value()?)?,
            "--token" => options.token = Some(utf8(flag, value()?)?),
            "--control-token" => options.control_token = Some(utf8(flag, value()?)?),
            "--project" => options.project = Some(PathBuf::from(value()?)),
            "--demo-presence" if inline.is_none() => options.demo_presence = true,
            "--demo-presence" => return Err("--demo-presence takes no value".to_owned()),
            "--stream-deck-links" => options.stream_deck_links = Some(PathBuf::from(value()?)),
            "--demo-ai" if inline.is_none() => options.demo_ai = true,
            "--demo-ai" => return Err("--demo-ai takes no value".to_owned()),
            "--ai" if inline.is_none() => options.ai = true,
            "--ai" => return Err("--ai takes no value".to_owned()),
            "--ai-settings" => options.ai_settings = PathBuf::from(value()?),
            "--env-file" => options.env_file = PathBuf::from(value()?),
            "--voice-wav" => options.voice_wav = Some(PathBuf::from(value()?)),
            "--voice-url" => options.voice_url = Some(utf8(flag, value()?)?),
            _ => return Err(format!("unknown option {text:?}")),
        }
    }
    if options.ai && options.demo_ai {
        return Err("--ai and --demo-ai cannot be used together".to_owned());
    }
    if options.voice_url.is_some() && options.voice_wav.is_none() {
        return Err("--voice-url needs --voice-wav".to_owned());
    }
    if options.voice_wav.is_some() && options.demo_ai {
        return Err("--voice-wav cannot be used with --demo-ai, which simulates voice".to_owned());
    }
    Ok(Command::Run(options))
}

fn parse_port(value: &OsString) -> Result<u16, String> {
    value
        .to_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| format!("--port needs a number from 0 to 65535, got {value:?}"))
}

fn utf8(flag: &str, value: OsString) -> Result<String, String> {
    value
        .into_string()
        .map_err(|value| format!("{flag} must be text, got {value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(OsString::from))
    }

    fn options(args: &[&str]) -> Options {
        match run(args) {
            Ok(Command::Run(options)) => options,
            other => panic!("{args:?} gave {other:?}"),
        }
    }

    #[test]
    fn defaults_apply_without_arguments() {
        let parsed = options(&[]);
        assert_eq!(parsed.port, DEFAULT_PORT);
        assert_eq!(parsed.token, None);
        assert_eq!(parsed.control_token, None);
        assert_eq!(parsed.project, None);
        assert!(!parsed.demo_presence);
        assert_eq!(parsed.stream_deck_links, None);
        assert!(!parsed.demo_ai);
        assert!(!parsed.ai);
        assert_eq!(parsed.ai_settings, PathBuf::from(".sceneuron/ai.json"));
        assert_eq!(parsed.env_file, PathBuf::from(".env"));
        assert_eq!((parsed.voice_wav, parsed.voice_url), (None, None));
    }

    #[test]
    fn real_ai_options_are_read_and_exclude_the_demo() {
        let parsed = options(&["--ai", "--ai-settings", "a.json", "--env-file=keys.env"]);
        assert!(parsed.ai);
        assert_eq!(parsed.ai_settings, PathBuf::from("a.json"));
        assert_eq!(parsed.env_file, PathBuf::from("keys.env"));
        assert!(
            run(&["--ai", "--demo-ai"])
                .unwrap_err()
                .contains("cannot be used together")
        );
        assert!(run(&["--ai=yes"]).is_err());
    }

    #[test]
    fn every_option_is_read_in_both_forms() {
        let spaced = options(&[
            "--port",
            "0",
            "--token",
            "t",
            "--control-token",
            "c",
            "--project",
            "p.json",
            "--demo-presence",
            "--stream-deck-links",
            "links.txt",
            "--demo-ai",
        ]);
        let joined = options(&[
            "--port=0",
            "--token=t",
            "--control-token=c",
            "--project=p.json",
            "--demo-presence",
            "--stream-deck-links=links.txt",
            "--demo-ai",
        ]);
        assert_eq!(spaced, joined);
        assert_eq!(spaced.port, 0);
        assert_eq!(spaced.token.as_deref(), Some("t"));
        assert_eq!(spaced.control_token.as_deref(), Some("c"));
        assert_eq!(spaced.project, Some(PathBuf::from("p.json")));
        assert!(spaced.demo_presence);
        assert_eq!(spaced.stream_deck_links, Some(PathBuf::from("links.txt")));
        assert!(spaced.demo_ai);
        assert!(options(&["--demo-ai"]).demo_ai && !options(&["--demo-ai"]).demo_presence);
        assert_eq!(options(&["--port", "1", "--port", "2"]).port, 2);
    }

    #[test]
    fn voice_options_are_read_and_the_url_needs_a_file() {
        let parsed = options(&["--voice-wav", "say.wav", "--voice-url=ws://127.0.0.1:9/v1"]);
        assert_eq!(parsed.voice_wav, Some(PathBuf::from("say.wav")));
        assert_eq!(parsed.voice_url.as_deref(), Some("ws://127.0.0.1:9/v1"));
        assert_eq!(options(&["--voice-wav=a.wav"]).voice_url, None);
        assert!(
            run(&["--voice-url", "wss://example.com"])
                .unwrap_err()
                .contains("needs --voice-wav")
        );
        assert!(run(&["--voice-wav"]).is_err() && run(&["--voice-wav", "a.wav", "--voice-url"]).is_err());
    }

    #[test]
    fn help_wins() {
        assert_eq!(run(&["--port", "1", "--help"]), Ok(Command::Help));
        assert_eq!(run(&["-h"]), Ok(Command::Help));
    }

    #[test]
    fn mistakes_are_reported() {
        for args in [
            &["--port"][..],
            &["--port", "65536"],
            &["--port", "-1"],
            &["--port", "web"],
            &["--token"],
            &["--control-token"],
            &["--project"],
            &["--stream-deck-links"],
            &["--demo-presence=yes"],
            &["--demo-ai=yes"],
            &["--verbose"],
            &["serve"],
        ] {
            assert!(run(args).is_err(), "{args:?}");
        }
    }
}
