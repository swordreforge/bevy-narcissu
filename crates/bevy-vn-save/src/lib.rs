//! bevy-vn-save — Save/load plugin for the Bevy VN engine.
//!
//! Manages persistent save slots as JSON files. Collects script engine
//! state (flags, position, call stack) and restores it on load.
//!
//! Save file layout: `saves/slot_N.json`

use bevy::prelude::*;
use bevy_vn_core::script::ScriptEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ── Save data ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    pub version: u32,
    pub timestamp: u64,
    pub description: String,
    pub engine: EngineSnapshot,
    /// Extension point: per-subsystem JSON blobs.
    #[serde(default)]
    pub subsystems: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSnapshot {
    pub current_script: String,
    pub current_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub global_flags: HashMap<u32, i32>,
    pub current_route: Option<String>,
    pub finished: bool,
}

impl From<&ScriptEngine> for EngineSnapshot {
    fn from(e: &ScriptEngine) -> Self {
        Self {
            current_script: e.current_script.clone(),
            current_line: e.current_line,
            call_stack: e.call_stack.clone(),
            flags: e.flags.clone(),
            global_flags: e.global_flags.clone(),
            current_route: e.current_route.clone(),
            finished: e.finished,
        }
    }
}

impl EngineSnapshot {
    fn apply(&self, e: &mut ScriptEngine) {
        e.current_script = self.current_script.clone();
        e.current_line = self.current_line;
        e.call_stack = self.call_stack.clone();
        e.flags = self.flags.clone();
        e.global_flags = self.global_flags.clone();
        e.current_route = self.current_route.clone();
        e.finished = false; // reset on load
    }
}

// ── Manager ──

const SLOT_COUNT: usize = 20;
const SAVE_VERSION: u32 = 1;

#[derive(Resource)]
pub struct SaveManager {
    pub save_dir: PathBuf,
    pub slots: [Option<SaveSlot>; SLOT_COUNT],
    pub loaded: bool,
}

impl SaveManager {
    pub fn new(save_dir: PathBuf) -> Self {
        let mut mgr = Self {
            save_dir,
            slots: [const { None }; SLOT_COUNT],
            loaded: false,
        };
        mgr.refresh();
        mgr
    }

    /// Scan the save directory for existing slots.
    pub fn refresh(&mut self) {
        let _ = fs::create_dir_all(&self.save_dir);
        for i in 0..SLOT_COUNT {
            let path = self.slot_path(i);
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(slot) = serde_json::from_slice::<SaveSlot>(&bytes) {
                    self.slots[i] = Some(slot);
                }
            } else {
                self.slots[i] = None;
            }
        }
        self.loaded = true;
    }

    /// Save engine state to a slot.
    pub fn save(&mut self, index: usize, engine: &ScriptEngine, description: &str) -> Result<(), String> {
        if index >= SLOT_COUNT {
            return Err(format!("slot index {index} out of range"));
        }
        let slot = SaveSlot {
            version: SAVE_VERSION,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
            description: description.to_string(),
            engine: EngineSnapshot::from(engine),
            subsystems: HashMap::new(),
        };
        let json = serde_json::to_string_pretty(&slot).map_err(|e| e.to_string())?;
        fs::write(self.slot_path(index), json).map_err(|e| e.to_string())?;
        self.slots[index] = Some(slot);
        Ok(())
    }

    /// Load engine state from a slot.
    pub fn load(&self, index: usize, engine: &mut ScriptEngine) -> Result<(), String> {
        let slot = self.slots[index].as_ref()
            .ok_or_else(|| format!("slot {index} is empty"))?;
        if slot.version != SAVE_VERSION {
            return Err(format!("save version {} not supported", slot.version));
        }
        slot.engine.apply(engine);
        Ok(())
    }

    /// Delete a save slot.
    pub fn delete(&mut self, index: usize) -> Result<(), String> {
        if index >= SLOT_COUNT { return Err("index out of range".into()); }
        fs::remove_file(self.slot_path(index)).map_err(|e| e.to_string())?;
        self.slots[index] = None;
        Ok(())
    }

    fn slot_path(&self, index: usize) -> PathBuf {
        self.save_dir.join(format!("slot_{index}.json"))
    }
}

// ── Plugin ──

pub struct VnSavePlugin {
    pub save_dir: String,
}

impl Default for VnSavePlugin {
    fn default() -> Self {
        Self { save_dir: "saves".into() }
    }
}

impl Plugin for VnSavePlugin {
    fn build(&self, app: &mut App) {
        let save_dir = PathBuf::from(&self.save_dir);
        app.insert_resource(SaveManager::new(save_dir));
        app.add_systems(Update, handle_save_point);
    }
}

/// Respond to SavePointEvent by auto-saving at the next available slot.
fn handle_save_point(
    mut reader: MessageReader<bevy_vn_core::messages::SavePointEvent>,
    engine: Res<ScriptEngine>,
    mut mgr: ResMut<SaveManager>,
) {
    for _event in reader.read() {
        let desc = format!("auto-{}", chrono_now());
        // Use slot 0 as auto-save target
        if let Err(e) = mgr.save(0, &engine, &desc) {
            warn!("auto-save failed: {e}");
        }
    }
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}
