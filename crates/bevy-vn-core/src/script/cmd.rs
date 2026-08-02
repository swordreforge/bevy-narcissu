#![allow(dead_code)]
//! Script command IR — the wire format for `.vnscript.ron` files.
//!
//! Uses Bevy Scene Notation-style internally-tagged serde for
//! versioned, forward-compatible deserialization.
//!
//! RON format: `(cmd:dialogue,args:(speaker:Some("A"),text:"Hi"))`
//! JSON format: `{"cmd":"dialogue","args":{"speaker":"A","text":"Hi"}}`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Versioning ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptVersion {
    V1,
}

// ── Script container ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnScript {
    pub version: ScriptVersion,
    pub meta: ScriptMeta,
    pub instructions: Vec<ScriptCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScriptMeta {
    pub name: Option<String>,
    /// Explicit next script name (replaces old +10 numeric stepping).
    pub next_script: Option<String>,
}

// ── Helper types ──

/// Foreground sprite position on screen.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FgPosition {
    Left,
    Center,
    Right,
    /// Custom absolute x (0.0–1.0 relative to screen width).
    Custom { x: f32 },
}

/// Transition effect for rendering commands.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    /// Cross-fade over the given duration in seconds.
    Fade { duration: f32 },
    /// Apply instantly, no transition.
    None,
}

/// Kind of full-screen overlay effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenEffectKind {
    Flash,
    Fade,
    ScreenOverlay,
}

/// Comparison operator for IfFlag / condition expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

// ── Script command enum ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum ScriptCmd {
    // ── Control flow ──
    Label {
        name: String,
    },
    Jump {
        label: String,
    },
    Call {
        label: String,
    },
    CallScript {
        script: String,
        label: Option<String>,
    },
    Return,
    /// Condition branch (expression: "flag_key op value").
    Condition {
        expression: String,
        goto_true: String,
        goto_false: Option<String>,
    },
    Halt,

    // ── Dialogue ──
    Dialogue {
        speaker: Option<String>,
        text: String,
        voice: Option<String>,
    },
    ClearDialogue,

    // ── Choices ──
    ChoiceBegin,
    ChoiceOption {
        text: String,
        goto: String,
        affection: Vec<(String, i32)>,
    },
    ChoiceEnd {
        convergence: String,
    },

    // ── Rendering ──
    SetBg {
        image: String,
        transition: Option<Transition>,
    },
    ShowFg {
        char_id: String,
        expression: String,
        position: FgPosition,
        transition: Option<Transition>,
    },
    HideFg {
        char_id: String,
        transition: Option<Transition>,
    },
    ShowFace {
        char_id: String,
        expression: String,
    },
    HideFace {
        char_id: String,
    },
    ShowCg {
        image: String,
        transition: Option<Transition>,
    },
    HideCg {
        transition: Option<Transition>,
    },
    ScrollBg {
        speed_x: f32,
        speed_y: f32,
        time_ms: u64,
    },
    /// Generic sprite overlay (replaces DrawSprite/DrawSpriteEx).
    Sprite {
        id: String,
        image: String,
        x: f32,
        y: f32,
        anchor_x: Option<f32>,
        anchor_y: Option<f32>,
        z: Option<i32>,
    },
    SpriteFade {
        id: String,
        opacity: f32,
        duration_ms: u64,
    },
    SpriteMove {
        id: String,
        x: f32,
        y: f32,
        duration_ms: u64,
    },
    SpriteRemove {
        id: String,
    },

    // ── Audio ──
    PlayBgm {
        id: String,
        volume: Option<f32>,
        fade_ms: Option<u64>,
    },
    StopBgm {
        fade_ms: Option<u64>,
    },
    PlaySe {
        file: String,
        channel: Option<usize>,
        volume: Option<f32>,
    },
    StopSe {
        channel: Option<usize>,
    },
    PlayVoice {
        file: String,
        volume: Option<f32>,
    },
    SetVolume {
        bgm: Option<f32>,
        se: Option<f32>,
        voice: Option<f32>,
    },

    // ── Effects ──
    Wait {
        time_ms: u64,
    },
    ScreenEffect {
        kind: ScreenEffectKind,
        color: Option<String>,
        duration_ms: u64,
    },
    Shake {
        intensity: f32,
        duration_frames: u32,
    },
    ScrollView {
        x: f32,
        y: f32,
        duration_ms: u64,
    },

    // ── State management ──
    SetFlag {
        key: String,
        value: i32,
    },
    SetGlobalFlag {
        flag_id: u32,
        value: i32,
    },
    IfFlag {
        flag_key: String,
        op: ConditionOp,
        value: String,
        goto: String,
    },
    UnlockCg {
        image: String,
    },
    UnlockBgm {
        id: String,
    },
    UnlockScene {
        scene_id: String,
    },

    // ── Meta ──
    SavePoint {
        id: String,
    },
    SetNextScript {
        script: String,
    },
    RouteFlag {
        route_key: String,
    },
    SetMode {
        mode: String,
    },

    // ── Video ──
    PlayMovie {
        file: String,
        blocking: bool,
    },
    StopMovie,
    SpriteVideo {
        id: String,
        file: String,
        x: f32,
        y: f32,
    },
    StopSpriteVideo {
        id: String,
    },

    // ── Extension ──
    /// Game-specific custom command.
    Custom {
        tag: String,
        data: HashMap<String, String>,
    },

    // ── Compatibility ──
    /// Unknown/unrecognized command — safely skipped.
    #[serde(other)]
    Unknown,
}
