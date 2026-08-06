//! Persistent game settings — the complete set of options exposed by the
//! original Narcissu 10th Anniversary settings screens.
//!
//! Stored as `saves/settings.json` (serde_json). All volume/speed fields are
//! 0-100 f32; `effect`/`messkip`/`rclick` are small ints; the rest are bools.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Every user-tunable option from the original two-page settings UI plus the
/// text sub-screen. Field names mirror the original engine's config keys.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    // ── Volume sliders (0-100) ──
    pub master: f32,
    pub bgm: f32,
    pub bgmvo: f32,
    pub voice: f32,
    pub se: f32,
    pub sysse: f32,
    pub movie: f32,

    // ── Speed / alpha sliders (0-100) ──
    pub mspeed: f32,
    pub aspeed: f32,
    pub mw_alpha: f32,

    // ── Radio groups (small ints) ──
    pub effect: i32,
    pub messkip: i32,
    pub rclick: i32,

    // ── Bool toggles (page 1 / page 2) ──
    pub fl_mspeed: bool,
    pub fl_master: bool,
    pub fl_bgm: bool,
    pub fl_bgmvo: bool,
    pub fl_voice: bool,
    pub fl_se: bool,
    pub fl_sysse: bool,
    pub fl_movie: bool,
    pub voiceskip: bool,

    // ── Per-character volumes (0-100) ──
    pub c001: f32,
    pub c002: f32,
    pub c003: f32,
    pub c004: f32,
    pub c005: f32,
    pub man: f32,
    pub fem: f32,

    // ── Per-character enable toggles ──
    pub fl_c001: bool,
    pub fl_c002: bool,
    pub fl_c003: bool,
    pub fl_c004: bool,
    pub fl_c005: bool,
    pub fl_man: bool,
    pub fl_fem: bool,

    // ── Text sub-screen toggles ──
    pub shadow: bool,
    pub outline: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            // Volumes default to 100 (full).
            master: 100.0,
            bgm: 100.0,
            bgmvo: 100.0,
            voice: 100.0,
            se: 100.0,
            sysse: 100.0,
            movie: 100.0,
            // Speeds/alpha default to 100.
            mspeed: 100.0,
            aspeed: 100.0,
            mw_alpha: 100.0,
            // Radio defaults: effect=1 (on), messkip=1 (on), rclick=0.
            effect: 1,
            messkip: 1,
            rclick: 0,
            // Master flags default ON.
            fl_mspeed: true,
            fl_master: true,
            fl_bgm: true,
            fl_bgmvo: true,
            fl_voice: true,
            fl_se: true,
            fl_sysse: true,
            fl_movie: true,
            voiceskip: false,
            // Per-char volumes default 100.
            c001: 100.0,
            c002: 100.0,
            c003: 100.0,
            c004: 100.0,
            c005: 100.0,
            man: 100.0,
            fem: 100.0,
            // Per-char toggles default ON.
            fl_c001: true,
            fl_c002: true,
            fl_c003: true,
            fl_c004: true,
            fl_c005: true,
            fl_man: true,
            fl_fem: true,
            // Text sub-screen defaults ON.
            shadow: true,
            outline: true,
        }
    }
}

/// Resolve the settings file path under `save_dir` (same convention as
/// bevy-vn-save: relative paths resolve against the process CWD).
pub fn settings_path(save_dir: &str) -> PathBuf {
    Path::new(save_dir).join("settings.json")
}

/// Load settings from `saves/settings.json`. Missing or unparsable file
/// falls back to [`GameSettings::default`] with a warning.
pub fn load_settings(save_dir: &str) -> GameSettings {
    let path = settings_path(save_dir);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                bevy::log::warn!("settings.json parse error ({e}); using defaults");
                GameSettings::default()
            }
        },
        Err(_) => GameSettings::default(),
    }
}

/// Persist settings to `saves/settings.json` (creates the directory).
pub fn save_settings(settings: &GameSettings, save_dir: &str) {
    let path = settings_path(save_dir);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(settings) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                bevy::log::warn!("failed to write settings.json: {e}");
            }
        }
        Err(e) => bevy::log::warn!("failed to serialize settings: {e}"),
    }
}