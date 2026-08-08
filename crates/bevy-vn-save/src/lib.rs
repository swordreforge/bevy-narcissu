//! bevy-vn-save — Save/load plugin for the Bevy VN engine.
//!
//! Manages persistent save slots as JSON files through the [`AppStorage`]
//! abstraction (native: FsStorage; wasm: injected localStorage impl).
//! Collects script engine state (flags, position, call stack) and restores
//! it on load.
//!
//! Save file layout: `saves/slot_N.json`

use std::sync::Arc;

use bevy::prelude::*;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_core::storage::{AppStorage, FsStorage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Save data ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    pub version: u32,
    pub timestamp: u64,
    pub description: String,
    pub engine: EngineSnapshot,
    /// Display metadata captured at save time (scene thumbnail, chapter,
    /// dialogue preview). Missing on old saves — `#[serde(default)]` keeps
    /// them loadable.
    #[serde(default)]
    pub meta: SlotMeta,
    /// Extension point: per-subsystem JSON blobs.
    #[serde(default)]
    pub subsystems: HashMap<String, serde_json::Value>,
}

/// Display metadata shown on the save/load screen slot.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SlotMeta {
    /// Background image key (e.g. "bg01"), resolved via `AssetPathProvider`.
    pub bg: Option<String>,
    /// Human-readable chapter title (e.g. "一号公路").
    pub chapter: Option<String>,
    /// Abbreviated dialogue preview (multi-line, truncated).
    pub preview: Option<String>,
    /// Last spoken line `(speaker, text)` — re-shown on load so the player
    /// sees the line they saved on.
    pub last_dialogue: Option<(Option<String>, String)>,
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
    pub storage: Arc<dyn AppStorage>,
    pub save_dir: String,
    pub slots: [Option<SaveSlot>; SLOT_COUNT],
    pub loaded: bool,
}

impl SaveManager {
    pub fn new(save_dir: String, storage: Arc<dyn AppStorage>) -> Self {
        let mut mgr = Self {
            storage,
            save_dir,
            slots: [const { None }; SLOT_COUNT],
            loaded: false,
        };
        mgr.refresh();
        mgr
    }

    /// Scan the save directory for existing slots.
    pub fn refresh(&mut self) {
        for i in 0..SLOT_COUNT {
            let path = self.slot_path(i);
            match self.storage.read(&path) {
                Ok(Some(text)) => {
                    if let Ok(slot) = serde_json::from_str::<SaveSlot>(&text) {
                        self.slots[i] = Some(slot);
                    }
                }
                _ => self.slots[i] = None,
            }
        }
        self.loaded = true;
    }

    /// Save engine state to a slot.
    pub fn save(&mut self, index: usize, engine: &ScriptEngine, description: &str) -> Result<(), String> {
        self.save_with_meta(index, engine, description, SlotMeta::default())
    }

    /// Save engine state plus display metadata to a slot.
    pub fn save_with_meta(
        &mut self,
        index: usize,
        engine: &ScriptEngine,
        description: &str,
        meta: SlotMeta,
    ) -> Result<(), String> {
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
            meta,
            subsystems: HashMap::new(),
        };
        let json = serde_json::to_string_pretty(&slot).map_err(|e| e.to_string())?;
        self.storage.write(&self.slot_path(index), &json)?;
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
        self.storage.remove(&self.slot_path(index))?;
        self.slots[index] = None;
        Ok(())
    }

    fn slot_path(&self, index: usize) -> String {
        format!("{}/slot_{index}.json", self.save_dir)
    }
}

// ── Plugin ──

/// The platform storage injected as a Bevy resource — shared by save slots
/// and settings persistence. Native default is [`FsStorage`]; wasm builds
/// insert a localStorage-backed impl before adding `VnSavePlugin`.
#[derive(Resource, Clone)]
pub struct AppStorageResource(pub Arc<dyn AppStorage>);

pub struct VnSavePlugin {
    pub save_dir: String,
    pub storage: Option<Arc<dyn AppStorage>>,
}

impl Default for VnSavePlugin {
    fn default() -> Self {
        Self { save_dir: "saves".into(), storage: None }
    }
}

impl Plugin for VnSavePlugin {
    fn build(&self, app: &mut App) {
        let storage: Arc<dyn AppStorage> = match &self.storage {
            Some(s) => s.clone(),
            None => Arc::new(FsStorage),
        };
        app.insert_resource(AppStorageResource(storage.clone()));
        app.insert_resource(SaveManager::new(self.save_dir.clone(), storage));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> ScriptEngine {
        let mut e = ScriptEngine::new();
        e.flags.insert("score".into(), 42);
        e.global_flags.insert(100, 1);
        e.current_script = "main".into();
        e.current_line = 5;
        e.call_stack = vec![("main".into(), 2)];
        e.current_route = Some("heroine_a".into());
        e
    }

    #[test]
    fn snapshot_roundtrip() {
        let engine = make_engine();
        let snap = EngineSnapshot::from(&engine);
        assert_eq!(snap.current_script, "main");
        assert_eq!(snap.current_line, 5);
        assert_eq!(snap.flags.get("score"), Some(&42));
        assert_eq!(snap.global_flags.get(&100), Some(&1));

        let mut restored = ScriptEngine::new();
        restored.current_script = "other".into();
        restored.current_line = 99;
        snap.apply(&mut restored);
        assert_eq!(restored.current_script, "main");
        assert_eq!(restored.current_line, 5);
        assert!(!restored.finished); // reset on load
        assert_eq!(restored.flags.get("score"), Some(&42));
    }

    #[test]
    fn save_slot_json_roundtrip() {
        let engine = make_engine();
        let slot = SaveSlot {
            version: 1,
            timestamp: 1234567890,
            description: "test save".into(),
            engine: EngineSnapshot::from(&engine),
            meta: SlotMeta::default(),
            subsystems: HashMap::new(),
        };
        let json = serde_json::to_string(&slot).unwrap();
        let parsed: SaveSlot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.description, "test save");
        assert_eq!(parsed.engine.current_line, 5);
        assert_eq!(parsed.engine.flags.get("score"), Some(&42));
    }

    #[test]
    fn legacy_slot_without_meta_loads() {
        let legacy = r#"{
            "version": 1,
            "timestamp": 1234567890,
            "description": "old save",
            "engine": {"current_script":"main","current_line":5,"call_stack":[["main",2]],"flags":{"score":42},"global_flags":{},"current_route":null,"finished":false},
            "subsystems": {}
        }"#;
        let parsed: SaveSlot = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.description, "old save");
        assert_eq!(parsed.meta, SlotMeta::default());
    }

    #[test]
    fn save_with_meta_roundtrip() {
        let tmp = std::env::temp_dir().join("bevy_vn_test_saves_meta");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mgr = SaveManager::new(tmp.to_str().unwrap().into(), Arc::new(FsStorage));
        let engine = make_engine();

        let meta = SlotMeta {
            bg: Some("bg01".into()),
            chapter: Some("一号公路".into()),
            preview: Some("行.「その道は…」".into()),
            last_dialogue: Some((Some("濑津美".into()), "その道は…".into())),
        };
        mgr.save_with_meta(1, &engine, "meta save", meta.clone()).unwrap();
        assert_eq!(mgr.slots[1].as_ref().unwrap().meta.chapter.as_deref(), Some("一号公路"));

        mgr.refresh();
        assert_eq!(mgr.slots[1].as_ref().unwrap().meta.bg.as_deref(), Some("bg01"));
        assert_eq!(
            mgr.slots[1].as_ref().unwrap().meta.last_dialogue,
            Some((Some("濑津美".into()), "その道は…".into()))
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn save_load_in_memory() {
        let tmp = std::env::temp_dir().join("bevy_vn_test_saves");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mgr = SaveManager::new(tmp.to_str().unwrap().into(), Arc::new(FsStorage));
        assert!(!mgr.slots[0].is_some());

        let mut engine = make_engine();
        // Save
        mgr.save(0, &engine, "auto").unwrap();
        assert!(mgr.slots[0].is_some());
        assert_eq!(mgr.slots[0].as_ref().unwrap().description, "auto");

        // Modify engine then load
        engine.current_line = 999;
        engine.flags.clear();
        mgr.load(0, &mut engine).unwrap();
        assert_eq!(engine.current_line, 5);
        assert_eq!(engine.flags.get("score"), Some(&42));

        // Load empty slot
        assert!(mgr.load(5, &mut engine).is_err());

        // Delete
        mgr.delete(0).unwrap();
        assert!(mgr.slots[0].is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn save_version_mismatch() {
        let tmp = std::env::temp_dir().join("bevy_vn_test_saves_v");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mgr = SaveManager::new(tmp.to_str().unwrap().into(), Arc::new(FsStorage));
        let mut engine = make_engine();

        mgr.save(0, &engine, "v1").unwrap();
        // Corrupt version
        if let Some(ref mut slot) = mgr.slots[0] {
            slot.version = 999;
            let json = serde_json::to_string(slot).unwrap();
            std::fs::write(mgr.slot_path(0), json).unwrap();
        }
        mgr.refresh();
        assert!(mgr.load(0, &mut engine).is_err());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
