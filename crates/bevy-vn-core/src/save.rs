//! Save/load system abstractions.
//!
//! Each subsystem implements SaveStateProvider to contribute
//! its state to the save file. VnSavePlugin orchestrates collection/restore.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for subsystems that need to persist state across save/load.
pub trait SaveStateProvider: Send + Sync {
    /// Collect current state as a JSON value for saving.
    fn collect_save_data(&self, world: &World) -> serde_json::Value;

    /// Restore state from a previously-saved JSON value.
    fn restore_save_data(&self, world: &mut World, data: &serde_json::Value) -> Result<(), String>;
}

/// Root save data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub timestamp: u64,
    pub screenshot: Option<Vec<u8>>,
    /// Per-subsystem data: provider_key → JSON blob.
    /// Each subsystem is free to define its own schema.
    pub subsystems: HashMap<String, serde_json::Value>,
}

/// Key used by ScriptEngine for its save subsystem entry.
pub const SCRIPT_ENGINE_KEY: &str = "script_engine";
/// Key used by AffectionMap for its save subsystem entry.
pub const AFFECTION_KEY: &str = "affection";
/// Key used by UnlockState for its save subsystem entry.
pub const UNLOCK_KEY: &str = "unlock";
/// Key used by Settings for its save subsystem entry.
pub const SETTINGS_KEY: &str = "settings";
