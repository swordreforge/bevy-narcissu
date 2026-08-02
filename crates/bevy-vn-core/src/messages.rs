//! Engine-level Event types for cross-plugin communication.
//!
//! All script-execution → subsystem commands flow through these Events.
//! Each subsystem plugin registers its own Event readers.
//! No plugin writes another plugin's resources directly.

use bevy::prelude::*;

// ── Engine control ──

/// Drives script execution. Sent by user input, auto-skip timer, or video EOS.
#[derive(Message, Clone)]
pub struct AdvanceEvent {
    pub source: AdvanceSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceSource {
    UserInput,
    Auto,
    Skip,
    VideoEnd,
}

/// Request a state transition (replaces old direct ScreenTransition writes).
#[derive(Message)]
pub struct TransitionRequest {
    pub target: crate::state::VnAppState,
    pub duration: Option<f32>,
}

#[derive(Message)]
pub struct TransitionComplete {
    pub target: crate::state::VnAppState,
}

// ── Rendering ──

#[derive(Message, Clone)]
pub struct SetBgEvent {
    pub image: String,
    pub transition: Option<crate::script::cmd::Transition>,
}

#[derive(Message, Clone)]
pub struct ShowFgEvent {
    pub char_id: String,
    pub expression: String,
    pub position: crate::script::cmd::FgPosition,
    pub transition: Option<crate::script::cmd::Transition>,
}

#[derive(Message, Clone)]
pub struct HideFgEvent {
    pub char_id: String,
    pub transition: Option<crate::script::cmd::Transition>,
}

#[derive(Message, Clone)]
pub struct ShowFaceEvent {
    pub char_id: String,
    pub expression: String,
}

#[derive(Message, Clone)]
pub struct HideFaceEvent {
    pub char_id: String,
}

#[derive(Message, Clone)]
pub struct ShowCgEvent {
    pub image: String,
    pub transition: Option<crate::script::cmd::Transition>,
}

#[derive(Message, Clone)]
pub struct HideCgEvent {
    pub transition: Option<crate::script::cmd::Transition>,
}

#[derive(Message, Clone)]
pub struct ScrollBgEvent {
    pub speed_x: f32,
    pub speed_y: f32,
    pub time_ms: u64,
}

#[derive(Message, Clone)]
pub struct SpriteEvent {
    pub id: String,
    pub image: String,
    pub x: f32,
    pub y: f32,
    pub anchor_x: Option<f32>,
    pub anchor_y: Option<f32>,
    pub z: Option<i32>,
}

#[derive(Message, Clone)]
pub struct SpriteEffectEvent {
    pub id: String,
    pub effect: SpriteEffectKind,
}

#[derive(Debug, Clone)]
pub enum SpriteEffectKind {
    Fade { opacity: f32, duration_ms: u64 },
    Move { x: f32, y: f32, duration_ms: u64 },
    Remove,
}

#[derive(Message, Clone)]
pub struct ScreenEffectEvent {
    pub kind: crate::script::cmd::ScreenEffectKind,
    pub color: Option<String>,
    pub duration_ms: u64,
}

// ── Audio ──

#[derive(Message, Clone)]
pub struct PlayBgmEvent {
    pub id: String,
    pub volume: Option<f32>,
    pub fade_ms: Option<u64>,
}

#[derive(Message, Clone)]
pub struct StopBgmEvent {
    pub fade_ms: Option<u64>,
}

#[derive(Message, Clone)]
pub struct PlaySeEvent {
    pub file: String,
    pub channel: Option<usize>,
    pub volume: Option<f32>,
}

#[derive(Message, Clone)]
pub struct StopSeEvent {
    pub channel: Option<usize>,
}

#[derive(Message, Clone)]
pub struct PlayVoiceEvent {
    pub file: String,
    pub volume: Option<f32>,
}

#[derive(Message, Clone)]
pub struct SetVolumeEvent {
    pub bgm: Option<f32>,
    pub se: Option<f32>,
    pub voice: Option<f32>,
}

// ── Video ──

#[derive(Message, Clone)]
pub struct PlayMovieEvent {
    pub file: String,
    pub blocking: bool,
}

#[derive(Message, Clone)]
pub struct StopMovieEvent;

#[derive(Message, Clone)]
pub struct SpriteVideoEvent {
    pub id: String,
    pub file: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Message, Clone)]
pub struct StopSpriteVideoEvent {
    pub id: String,
}

// ── State mutations ──

#[derive(Message, Clone)]
pub struct UnlockCgEvent {
    pub image: String,
}

#[derive(Message, Clone)]
pub struct UnlockBgmEvent {
    pub id: String,
}

#[derive(Message, Clone)]
pub struct AffectionChangeEvent {
    pub char_key: String,
    pub delta: i32,
}

#[derive(Message, Clone)]
pub struct BacklogPushEvent {
    pub entry: BacklogEntry,
}

#[derive(Debug, Clone)]
pub struct BacklogEntry {
    pub speaker: Option<String>,
    pub text: String,
    pub voice_file: Option<String>,
}

#[derive(Message, Clone)]
pub struct SavePointEvent {
    pub id: String,
}

// ── UI state ──

#[derive(Message, Clone)]
pub struct DialogueStateEvent {
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Message, Clone)]
pub struct ClearDialogueEvent;

#[derive(Message, Clone)]
pub struct ChoiceStateEvent {
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone)]
pub struct ChoiceOption {
    pub text: String,
    pub goto: String,
    pub affection: Vec<(String, i32)>,
}

#[derive(Message, Clone)]
pub struct ChoiceSelectedEvent {
    pub index: usize,
}

#[derive(Message)]
pub struct ScriptCommandComplete {
    pub cmd: String,
}
