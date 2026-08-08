//! Cross-platform storage abstraction for small JSON files (save slots,
//! settings).
//!
//! Platform differences are isolated behind a single trait:
//! - Native: [`FsStorage`] (std::fs, paths relative to process CWD).
//! - Web: [`LocalStorage`] (browser localStorage, wasm32 only).
//!
//! Consumers (`bevy-vn-save`, `bevy-vn-ui/settings_data`) take
//! `&dyn AppStorage` and never touch `std::fs` directly.

use std::path::Path;

/// Abstract key-value file storage. Paths are forward-slash relative
/// strings resolved against the storage root (CWD for native).
pub trait AppStorage: Send + Sync {
    /// Read a file as UTF-8 text. `Ok(None)` if the file does not exist.
    fn read(&self, path: &str) -> Result<Option<String>, String>;

    /// Write a UTF-8 text file, creating parent directories as needed.
    fn write(&self, path: &str, data: &str) -> Result<(), String>;

    /// Delete a file. `Ok(())` even if it did not exist.
    fn remove(&self, path: &str) -> Result<(), String>;
}

/// Native filesystem implementation. Paths resolve relative to the process
/// current working directory (same convention the engine used before the
/// storage abstraction existed).
#[derive(Debug, Clone, Copy, Default)]
pub struct FsStorage;

impl AppStorage for FsStorage {
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn write(&self, path: &str, data: &str) -> Result<(), String> {
        if let Some(dir) = Path::new(path).parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
        }
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    fn remove(&self, path: &str) -> Result<(), String> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Browser localStorage implementation (wasm32 only). Persists JSON blobs
/// across page reloads; keys are prefixed `bevy_vn:` to avoid collisions
/// with other apps sharing the origin.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalStorage;

#[cfg(target_arch = "wasm32")]
impl AppStorage for LocalStorage {
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        let storage = local_storage()?;
        storage.get_item(&key(path)).map_err(js_error)
    }

    fn write(&self, path: &str, data: &str) -> Result<(), String> {
        let storage = local_storage()?;
        storage.set_item(&key(path), data).map_err(js_error)
    }

    fn remove(&self, path: &str) -> Result<(), String> {
        let storage = local_storage()?;
        storage.remove_item(&key(path)).map_err(js_error)
    }
}

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .ok_or_else(|| "localStorage unavailable".into())
}

#[cfg(target_arch = "wasm32")]
fn key(path: &str) -> String {
    format!("bevy_vn:{path}")
}

#[cfg(target_arch = "wasm32")]
fn js_error(e: wasm_bindgen::JsValue) -> String {
    e.as_string().unwrap_or_else(|| "browser storage error".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_storage_roundtrip() {
        let dir = std::env::temp_dir().join("bevy_vn_storage_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("file.json");
        let path_str = path.to_str().unwrap().to_string();

        let s = FsStorage;
        assert_eq!(s.read(&path_str).unwrap(), None);

        s.write(&path_str, r#"{"ok":true}"#).unwrap();
        assert_eq!(s.read(&path_str).unwrap().as_deref(), Some(r#"{"ok":true}"#));

        s.remove(&path_str).unwrap();
        assert_eq!(s.read(&path_str).unwrap(), None);
        // Remove of a missing file is not an error.
        s.remove(&path_str).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }
}
