// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! A stand-in for OBS's program: the scene "Scene 01", off air, until a line on standard input
//! starts or stops streaming or recording, or switches the scene, as OBS would report them.

use std::sync::{Arc, Mutex, PoisonError};

use sceneuron_server::Hub;

/// The simulated scene's name at the start.
const SCENE: &str = "Scene 01";

pub(crate) const HELP: &str = "Type stream, record, stop, or scene NAME and press Enter to simulate OBS.";

/// What OBS would report.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Program {
    scene: String,
    streaming: bool,
    recording: bool,
}

/// What a line asks of the simulated OBS.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    Stream,
    Record,
    Stop,
    Scene(String),
}

pub(crate) struct OnAir {
    hub: Arc<Hub>,
    program: Mutex<Program>,
}

impl OnAir {
    /// Reports the simulated scene, off air, to the hub.
    pub(crate) fn start(hub: Arc<Hub>) -> OnAir {
        let program = Program {
            scene: SCENE.to_owned(),
            streaming: false,
            recording: false,
        };
        let on_air = OnAir {
            hub,
            program: Mutex::new(program.clone()),
        };
        on_air.report(&program);
        on_air
    }

    /// Applies a line typed on standard input; returns what changed, or `None` for a line that is
    /// not a command.
    pub(crate) fn command(&self, line: &str) -> Option<String> {
        let command = parse(line)?;
        let mut program = self.program.lock().unwrap_or_else(PoisonError::into_inner);
        let said = match command {
            Command::Stream => {
                program.streaming = true;
                "streaming"
            }
            Command::Record => {
                program.recording = true;
                "recording"
            }
            Command::Stop => {
                (program.streaming, program.recording) = (false, false);
                "off air"
            }
            Command::Scene(name) => {
                program.scene = name;
                "switched scenes"
            }
        };
        self.report(&program);
        Some(format!("OBS (simulated): {said}, in {:?}", program.scene))
    }

    fn report(&self, program: &Program) {
        self.hub
            .update(|engine, now| engine.set_on_air(&program.scene, program.streaming, program.recording, now));
    }
}

fn parse(line: &str) -> Option<Command> {
    let line = line.trim();
    let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    match (word.to_ascii_lowercase().as_str(), rest.trim()) {
        ("stream", "") => Some(Command::Stream),
        ("record", "") => Some(Command::Record),
        ("stop", "") => Some(Command::Stop),
        ("scene", name) if !name.is_empty() => Some(Command::Scene(name.to_owned())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_name_commands_in_any_case_and_scenes_keep_their_words() {
        assert_eq!(parse("stream"), Some(Command::Stream));
        assert_eq!(parse(" RECORD \n"), Some(Command::Record));
        assert_eq!(parse("Stop"), Some(Command::Stop));
        assert_eq!(
            parse("scene  Interview Cam "),
            Some(Command::Scene("Interview Cam".to_owned()))
        );
    }

    #[test]
    fn anything_else_is_not_a_command() {
        for line in ["", "q", "streaming", "stream now", "scene", "scene   ", "stop it"] {
            assert_eq!(parse(line), None, "{line:?}");
        }
    }
}
