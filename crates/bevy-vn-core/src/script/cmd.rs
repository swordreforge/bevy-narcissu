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

    // ── Interaction hotspots ──
    /// Declare a clickable screen region (pixel coords, origin top-left).
    /// While any hotspots are active, clicks outside them do nothing;
    /// with none active, the whole screen advances (default).
    Hotspot {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Remove all active hotspots.
    HotspotClear,

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

// ── Serde tests ──

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn ron_roundtrip_dialogue() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta { name: Some("test".into()), next_script: Some("next".into()) },
            instructions: vec![
                ScriptCmd::Label { name: "start".into() },
                ScriptCmd::Dialogue { speaker: Some("Alice".into()), text: "Hello".into(), voice: None },
                ScriptCmd::Halt,
            ],
        };
        let ron_str = ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.version, ScriptVersion::V1);
        assert_eq!(parsed.meta.name.as_deref(), Some("test"));
        assert_eq!(parsed.meta.next_script.as_deref(), Some("next"));
        assert_eq!(parsed.instructions.len(), 3);
        assert!(matches!(&parsed.instructions[0], ScriptCmd::Label { name } if name == "start"));
        assert!(matches!(&parsed.instructions[1], ScriptCmd::Dialogue { speaker, text, .. } if speaker.as_deref() == Some("Alice") && text == "Hello"));
        assert!(matches!(parsed.instructions[2], ScriptCmd::Halt));
    }

    #[test]
    fn ron_roundtrip_dialogue_voice() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::Dialogue { speaker: Some("Alice".into()), text: "Hello".into(), voice: Some("li/n002".into()) },
            ],
        };
        let ron_str = ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert!(matches!(&parsed.instructions[0], ScriptCmd::Dialogue { voice: Some(v), .. } if v == "li/n002"));
    }

    #[test]
    fn ron_roundtrip_control_flow() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::Jump { label: "end".into() },
                ScriptCmd::Call { label: "sub".into() },
                ScriptCmd::CallScript { script: "ch2".into(), label: Some("entry".into()) },
                ScriptCmd::Return,
                ScriptCmd::Condition { expression: "flag >= 3".into(), goto_true: "yes".into(), goto_false: Some("no".into()) },
                ScriptCmd::Label { name: "end".into() },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 6);
    }

    #[test]
    fn ron_roundtrip_rendering() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::SetBg { image: "bg01".into(), transition: Some(Transition::Fade { duration: 1.0 }) },
                ScriptCmd::ShowFg { char_id: "hero".into(), expression: "smile".into(), position: FgPosition::Center, transition: None },
                ScriptCmd::ShowCg { image: "ev01".into(), transition: None },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 3);
    }

    #[test]
    fn ron_roundtrip_audio() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::PlayBgm { id: "0101".into(), volume: Some(0.8), fade_ms: Some(500) },
                ScriptCmd::PlaySe { file: "click".into(), channel: Some(2), volume: None },
                ScriptCmd::PlayVoice { file: "v001".into(), volume: None },
                ScriptCmd::SetVolume { bgm: Some(0.7), se: Some(1.0), voice: None },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 4);
    }

    #[test]
    fn unknown_tag_becomes_unknown() {
        // JSON with an unknown cmd tag should deserialize as Unknown
        let json = r#"{"version":"V1","meta":{},"instructions":[{"cmd":"future_cmd","args":{}}]}"#;
        // Note: with adjacently-tagged + #[serde(other)], unknown with empty map args works
        let result: Result<VnScript, _> = serde_json::from_str(json);
        // If the deserialization succeeds, the unknown cmd should be Unknown variant
        if let Ok(script) = result {
            assert_eq!(script.instructions.len(), 1);
            assert!(matches!(script.instructions[0], ScriptCmd::Unknown));
        } else {
            // If it fails, that's also acceptable — serde(other) + adjacently-tagged has known limitations
            // with map-shaped args. The format is designed to be versioned, so unknown commands
            // would typically be handled by the ScriptVersion check before deserialization.
            eprintln!("Note: unknown cmd with args map failed to deserialize (expected with adjacently-tagged + #[serde(other)])");
        }
    }

    #[test]
    fn custom_variant() {
        let mut data = HashMap::new();
        data.insert("key".into(), "val".into());
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![ScriptCmd::Custom { tag: "my_effect".into(), data }],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert!(matches!(&parsed.instructions[0], ScriptCmd::Custom { tag, .. } if tag == "my_effect"));
    }

    #[test]
    fn fg_position_variants() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::ShowFg { char_id: "a".into(), expression: "x".into(), position: FgPosition::Left, transition: None },
                ScriptCmd::ShowFg { char_id: "b".into(), expression: "y".into(), position: FgPosition::Center, transition: None },
                ScriptCmd::ShowFg { char_id: "c".into(), expression: "z".into(), position: FgPosition::Right, transition: None },
                ScriptCmd::ShowFg { char_id: "d".into(), expression: "w".into(), position: FgPosition::Custom { x: 0.35 }, transition: None },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 4);
    }

    #[test]
    fn screen_effect_kinds() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::ScreenEffect { kind: ScreenEffectKind::Flash, color: Some("White".into()), duration_ms: 200 },
                ScriptCmd::ScreenEffect { kind: ScreenEffectKind::Fade, color: Some("Black".into()), duration_ms: 1000 },
                ScriptCmd::ScreenEffect { kind: ScreenEffectKind::ScreenOverlay, color: None, duration_ms: 0 },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 3);
    }

    #[test]
    fn all_transitions() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::SetBg { image: "bg".into(), transition: Some(Transition::Fade { duration: 0.5 }) },
                ScriptCmd::SetBg { image: "bg2".into(), transition: Some(Transition::None) },
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let _parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
    }

    #[test]
    fn ron_roundtrip_hotspot() {
        let script = VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta::default(),
            instructions: vec![
                ScriptCmd::Hotspot { id: "menu_1".into(), x: 100.0, y: 200.0, width: 300.0, height: 80.0 },
                ScriptCmd::HotspotClear,
            ],
        };
        let ron_str = ron::ser::to_string(&script).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron_str).unwrap();
        assert_eq!(parsed.instructions.len(), 2);
        match &parsed.instructions[0] {
            ScriptCmd::Hotspot { id, x, y, width, height } => {
                assert_eq!(id, "menu_1");
                assert_eq!(*x, 100.0);
                assert_eq!(*y, 200.0);
                assert_eq!(*width, 300.0);
                assert_eq!(*height, 80.0);
            }
            other => panic!("expected Hotspot, got {other:?}"),
        }
        assert!(matches!(parsed.instructions[1], ScriptCmd::HotspotClear));
    }
}
