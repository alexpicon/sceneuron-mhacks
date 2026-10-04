// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Optional API keys from the environment and `.env` files.
//!
//! Every key is optional. Without any, Sceneuron still tracks presence, runs rules, and keeps the
//! editor and manual controls working; a key only turns on the cloud feature that needs it. A key
//! the streamer saves in the editor wins over one found here, and keys found here are never written
//! to `ai.json`. Names carry a `SCENEURON_` prefix so a key set for another program is never
//! picked up and spent by accident.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::provider_kind::ProviderKind;

/// Key for the Gemini API.
pub const GEMINI_KEY_VAR: &str = "SCENEURON_GEMINI_API_KEY";
/// Key for an OpenAI-compatible endpoint, such as NVIDIA NIM.
pub const OPENAI_KEY_VAR: &str = "SCENEURON_OPENAI_API_KEY";
/// Key for TypeSafe's Jev model, which gives quick judgments from on-device facts.
pub const JEV_KEY_VAR: &str = "SCENEURON_JEV_API_KEY";
/// Key for ElevenLabs, which transcribes voice commands and speaks cards.
pub const ELEVENLABS_KEY_VAR: &str = "SCENEURON_ELEVENLABS_API_KEY";
/// Path of a `.env` file to read instead of the default one.
pub const ENV_FILE_VAR: &str = "SCENEURON_ENV_FILE";

/// Keys found outside the editor; empty strings mean none.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Credentials {
    pub gemini: String,
    pub openai: String,
    pub jev: String,
    pub elevenlabs: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = |key: &str| if key.is_empty() { "" } else { "<set>" };
        f.debug_struct("Credentials")
            .field("gemini", &shown(&self.gemini))
            .field("openai", &shown(&self.openai))
            .field("jev", &shown(&self.jev))
            .field("elevenlabs", &shown(&self.elevenlabs))
            .finish()
    }
}

impl Credentials {
    /// Reads `file` if it exists, then the process environment, which wins over the file.
    /// Warnings say what could not be used; a missing file is not one.
    pub fn load(file: Option<&Path>) -> (Credentials, Vec<String>) {
        let mut warnings = Vec::new();
        let mut values = match file {
            Some(path) => read_env_file(path, &mut warnings),
            None => BTreeMap::new(),
        };
        for name in [GEMINI_KEY_VAR, OPENAI_KEY_VAR, JEV_KEY_VAR, ELEVENLABS_KEY_VAR] {
            if let Ok(value) = std::env::var(name)
                && !value.trim().is_empty()
            {
                values.insert(name.to_owned(), value.trim().to_owned());
            }
        }
        (Credentials::from_values(&values), warnings)
    }

    /// The keys among `values`, by variable name.
    pub fn from_values(values: &BTreeMap<String, String>) -> Credentials {
        let get = |name: &str| values.get(name).cloned().unwrap_or_default();
        Credentials {
            gemini: get(GEMINI_KEY_VAR),
            openai: get(OPENAI_KEY_VAR),
            jev: get(JEV_KEY_VAR),
            elevenlabs: get(ELEVENLABS_KEY_VAR),
        }
    }

    /// The key for a vision and planner provider, or empty.
    pub fn for_provider(&self, provider: ProviderKind) -> &str {
        match provider {
            ProviderKind::Gemini => &self.gemini,
            ProviderKind::OpenAiCompatible => &self.openai,
        }
    }
}

/// The file to read: the one `SCENEURON_ENV_FILE` names, or else `default`.
pub fn env_file_path(default: &Path) -> std::path::PathBuf {
    match std::env::var(ENV_FILE_VAR) {
        Ok(path) if !path.trim().is_empty() => std::path::PathBuf::from(path.trim()),
        _ => default.to_path_buf(),
    }
}

fn read_env_file(path: &Path, warnings: &mut Vec<String>) -> BTreeMap<String, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return BTreeMap::new(),
        Err(error) => {
            warnings.push(format!("cannot read {}: {error}", path.display()));
            return BTreeMap::new();
        }
    };
    if readable_by_others(path) {
        warnings.push(format!(
            "{} can be read by other users; restrict it to its owner (chmod 600)",
            path.display()
        ));
    }
    let (values, bad_lines) = parse_env(&text);
    if !bad_lines.is_empty() {
        let lines: Vec<String> = bad_lines.iter().map(usize::to_string).collect();
        warnings.push(format!(
            "{}: ignored lines {} that are not NAME=value",
            path.display(),
            lines.join(", ")
        ));
    }
    values
}

/// Parses `NAME=value` lines; blank lines and `#` comments are skipped, `export ` is allowed,
/// and a value may be quoted. Returns the values and the numbers of lines that did not parse.
pub fn parse_env(text: &str) -> (BTreeMap<String, String>, Vec<usize>) {
    let mut values = BTreeMap::new();
    let mut bad = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            bad.push(index + 1);
            continue;
        };
        let name = name.trim();
        let valid_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid_name {
            bad.push(index + 1);
            continue;
        }
        values.insert(name.to_owned(), unquote(value.trim()));
    }
    (values, bad)
}

/// Strips matching quotes, or an unquoted value's trailing ` # comment`.
fn unquote(value: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote).and_then(|rest| rest.strip_suffix(quote)) {
            return inner.to_owned();
        }
    }
    match value.find(" #") {
        Some(comment) => value[..comment].trim_end().to_owned(),
        None => value.to_owned(),
    }
}

fn readable_by_others(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o077 != 0)
}
