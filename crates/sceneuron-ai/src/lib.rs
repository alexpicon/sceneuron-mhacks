// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Sceneuron's AI: vision-language model clients, the scheduling policy that decides when a
//! camera frame is worth a call, the director thread that turns frames into scene observations,
//! and the assistant that turns the streamer's words into edit plans.
//!
//! Models are reached over HTTPS: Google's Gemini API by default, or any OpenAI-compatible
//! endpoint such as NVIDIA NIM, Ollama, llama.cpp, or vLLM. Answers are always JSON that matches
//! a schema, and nothing is sent until the streamer turns scene understanding on and consents.
//! The same providers translate live subtitles, and turn voice commands that Sceneuron does not
//! know by itself into actions. Nothing here depends on OBS.

mod assistant;
mod commands;
mod cost;
mod credentials;
mod director;
mod elevenlabs;
mod gate;
mod gemini;
mod http;
#[cfg(feature = "hub")]
mod hub;
#[cfg(feature = "hub")]
mod hub_commands;
#[cfg(feature = "hub")]
mod hub_subtitles;
mod image;
mod jev;
mod json_text;
mod judge;
mod mock;
mod openai;
mod plan_schema;
mod planner;
mod provider;
mod provider_kind;
mod retired;
mod settings;
mod settings_file;
mod settings_update;
mod settings_view;
mod speaker;
mod speech_cache;
mod starter;
mod subtitle_settings;
mod subtitle_status;
mod translator;
mod tts;
mod voice_cost;
mod voice_link;
mod voice_settings;

pub use assistant::{Assistant, PlanOutcome, RoomHandler};
pub use commands::{
    CommandAnswer, MAX_ACTIONS, VOICE_PROMPT, check_edits, command_request, command_schema, parse_commands,
};
pub use cost::{Prices, cost_usd, gemini_prices, prices_for};
pub use credentials::{
    Credentials, ELEVENLABS_KEY_VAR, ENV_FILE_VAR, GEMINI_KEY_VAR, JEV_KEY_VAR, OPENAI_KEY_VAR, env_file_path,
    parse_env,
};
pub use director::{Director, Sink};
pub use elevenlabs::ElevenLabs;
pub use gate::{Decision, Gate, Reason};
#[cfg(feature = "hub")]
pub use hub::{AiService, HubSink, follow_hub, follow_voice};
pub use image::{Frame, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH, Thumbnail, encode_jpeg};
pub use jev::{JevClient, QuickJudgment, parse_quick, quick_request, quick_verdict};
pub use judge::{JUDGE_PROMPT, judge_request, parse_observation};
pub use mock::MockProvider;
pub use plan_schema::plan_schema;
pub use planner::{PLANNER_PROMPT, PlanMode, parse_plan, plan_request};
pub use provider::{AiError, Connector, ModelReply, ModelRequest, Provider, Usage, connect, default_connector};
pub use provider_kind::{DEFAULT_MODEL, DEFAULT_PLANNER_MODEL, DEFAULT_TRANSLATION_MODEL, DefaultModels, ProviderKind};
pub use settings::{AiSettings, DEFAULT_JEV_BASE_URL, DEFAULT_JEV_MODEL, MAX_IMAGE_WIDTH, MIN_IMAGE_WIDTH};
pub use settings_file::{load_settings, save_settings};
pub use settings_update::AiSettingsUpdate;
pub use settings_view::AiSettingsView;
pub use speech_cache::{MAX_SPEECH_FILES, SpeechCache};
pub use starter::place_starter_settings;
pub use subtitle_settings::{SubtitleSettings, SubtitleSettingsUpdate, SubtitleSettingsView};
pub use translator::{TRANSLATE_PROMPT, parse_translations, translation_request, translation_schema};
pub use tts::{
    AccountVoice, AccountVoices, DEFAULT_ELEVENLABS_URL, SPEECH_CONTENT_TYPE, SPEECH_FORMAT, TextToSpeech,
    missing_voice,
};
pub use voice_cost::{
    VoiceUsage, keyterm_rate_per_hour, stt_price_per_hour, stt_rate_per_hour, tts_price_per_1k_chars,
    tts_rate_per_1k_chars, voice_cost_usd,
};
pub use voice_link::{VoiceLink, voice_terms};
pub use voice_settings::{
    DEFAULT_STT_MODEL, DEFAULT_TTS_MODEL, DEFAULT_VOICE_ID, VoiceSettings, VoiceSettingsUpdate, VoiceSettingsView,
};
