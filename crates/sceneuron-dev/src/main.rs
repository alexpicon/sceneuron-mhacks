// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Runs the Sceneuron editor, overlay, and engine without OBS, for development and tests.

mod args;
mod demo;
mod demo_ai;
mod demo_devices;
mod demo_picture;
mod dev_host;
mod on_air;
mod quit;
mod real_ai;
mod voice_wav;

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use sceneuron_ai::place_starter_settings;
use sceneuron_core::{Project, default_project, load_or_starter};
use sceneuron_room::RoomClient;
use sceneuron_server::{Assistant, Hub, HubConfig, NullAssistant, Server, ServerConfig, generate_token};

use crate::args::{Command, Options, USAGE};
use crate::dev_host::DevHost;
use crate::on_air::OnAir;

fn main() -> ExitCode {
    let options = match args::parse(std::env::args_os().skip(1)) {
        Ok(Command::Run(options)) => options,
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("sceneuron-dev: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sceneuron-dev: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: Options) -> Result<(), String> {
    quit::install();
    let (project, fresh) = options
        .project
        .as_deref()
        .map_or_else(|| (default_project(), false), load_project);
    // A fresh install, like the plugin's first start, gets the starter AI settings too.
    let starter_ai = fresh && options.ai && place_starter_ai(&options.ai_settings);
    let config = HubConfig {
        project_path: options.project.clone(),
        ..HubConfig::default()
    };
    let hub = Hub::start(project, config).map_err(|error| format!("cannot start: {error}"))?;
    let server_config = ServerConfig {
        port: options.port,
        token: options.token.unwrap_or_else(generate_token),
        control_token: options.control_token.unwrap_or_else(generate_token),
        ..ServerConfig::default()
    };
    let mut real_ai = None;
    let (demo_ai, assistant): (_, Arc<dyn Assistant>) = if options.demo_ai {
        let (demo_ai, assistant) =
            demo_ai::start(Arc::clone(&hub)).map_err(|error| format!("cannot start the AI demo: {error}"))?;
        (Some(demo_ai), assistant)
    } else if options.ai {
        let voice_url = options.voice_url.as_deref();
        let (ai, service, warnings) = real_ai::start(&hub, &options.ai_settings, &options.env_file, voice_url);
        for warning in warnings {
            eprintln!("sceneuron-dev: {warning}");
        }
        real_ai = Some(ai);
        (None, service)
    } else {
        (None, Arc::new(NullAssistant))
    };
    // Connects to SpacetimeDB only once someone creates or joins a room in the editor.
    let room = RoomClient::start(Arc::clone(&hub));
    if let Some(ai) = &real_ai {
        ai.follow_room(&room);
    }
    if let Some(demo) = &demo_ai {
        demo.follow_room(&room);
    }
    let demo_devices = options.demo_presence.then(|| Arc::clone(&hub));
    let host = Arc::new(DevHost::new(demo_devices, options.stream_deck_links.clone()));
    let server = Server::start(Arc::clone(&hub), host, assistant, room.clone(), server_config)
        .map_err(|error| error.to_string())?;
    println!("Sceneuron development server");
    println!("Editor:  {}", server.editor_url());
    println!("Overlay: {}", server.overlay_url());
    match &options.project {
        Some(path) if fresh => println!("Project: {} (new: the starter setup)", path.display()),
        Some(path) => println!("Project: {}", path.display()),
        None => println!("Project: in memory only (use --project PATH to save)"),
    }
    let demo = if options.demo_presence {
        let demo = demo::start(Arc::clone(&hub)).map_err(|error| format!("cannot start the demo: {error}"))?;
        println!("Demo presence: in view for 8 s, away for 8 s, repeating");
        println!("Demo camera: a drawn picture for the analyzed simulated camera, and microphones that hear nothing");
        Some(demo)
    } else {
        None
    };
    if let Some(path) = &options.stream_deck_links {
        println!(
            "Stream Deck: the plugin counts as installed; pairing links go to {}",
            path.display()
        );
    }
    if demo_ai.is_some() {
        println!("Demo AI: simulated assistant and scene understanding from a scripted camera;");
        println!("         no camera image, API key, or request leaves this computer");
    }
    if real_ai.is_some() {
        println!(
            "AI: the assistant calls the models in {}",
            options.ai_settings.display()
        );
    }
    if starter_ai {
        println!("    (new: the starter settings, with no key and no consent)");
    }
    let voice = match &options.voice_wav {
        Some(path) => {
            let url = options.voice_url.as_deref();
            let assistant_voice = real_ai.as_ref().map(real_ai::RealAi::voice);
            let (voice, seconds) = voice_wav::start(&hub, path, url, &options.env_file, assistant_voice)?;
            let to = url.unwrap_or("ElevenLabs speech to text");
            println!(
                "Voice: speaking {} ({seconds:.1} s) to {to} as push to talk",
                path.display()
            );
            Some(voice)
        }
        None => None,
    };
    let on_air = OnAir::start(Arc::clone(&hub));
    quit::read_lines(move |line| {
        if let Some(said) = on_air.command(line) {
            println!("{said}");
        }
    });
    println!("{}", on_air::HELP);
    println!("Type q and press Enter, or press Ctrl-C, to quit.");
    quit::wait();
    if let Some(demo) = demo {
        demo.stop();
    }
    if let Some(demo_ai) = demo_ai {
        demo_ai.stop();
    }
    if let Some(voice) = voice {
        voice.stop();
    }
    if let Some(real_ai) = real_ai {
        real_ai.stop();
    }
    server.stop();
    room.shutdown();
    hub.shutdown();
    println!("Stopped.");
    Ok(())
}

/// Loads the saved project, the starter project when there is none yet, or the default one when it
/// is unreadable; the flag says whether there was none: a fresh install.
fn load_project(path: &Path) -> (Project, bool) {
    let loaded = load_or_starter(path);
    if let Some(warning) = loaded.warning {
        eprintln!("sceneuron-dev: {warning}");
    }
    (loaded.project, loaded.fresh)
}

/// Puts the starter AI settings at `path` unless a file is there; returns whether it did.
fn place_starter_ai(path: &Path) -> bool {
    place_starter_settings(path).unwrap_or_else(|error| {
        eprintln!(
            "sceneuron-dev: the starter AI settings could not be written to {}: {error}",
            path.display()
        );
        false
    })
}
