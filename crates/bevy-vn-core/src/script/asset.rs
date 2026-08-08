//! Bevy Asset and AssetLoader for `.vnscript.ron` files.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;

use crate::script::cmd::VnScript;

#[derive(Asset, TypePath, Clone)]
pub struct VnScriptAsset {
    pub script: VnScript,
}

/// AssetLoader for `.vnscript.ron` files.
#[derive(TypePath)]
pub struct VnScriptLoader;

impl AssetLoader for VnScriptLoader {
    type Asset = VnScriptAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let script: VnScript = ron::de::from_bytes(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(VnScriptAsset { script })
    }

    fn extensions(&self) -> &[&str] {
        &["vnscript.ron"]
    }
}

/// Manifest listing every `.vnscript.ron` file to load, one filename per
/// line. `load_folder` cannot be used on wasm (HttpWasmAssetReader returns
/// an empty directory listing), so the script set is enumerated in this
/// file and loaded individually — same code path on native and wasm.
#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct ScriptManifest {
    pub files: Vec<String>,
}

/// AssetLoader for `manifest.list` files.
#[derive(TypePath)]
pub struct ScriptManifestLoader;

impl AssetLoader for ScriptManifestLoader {
    type Asset = ScriptManifest;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let files = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(ToOwned::to_owned)
            .collect();
        Ok(ScriptManifest { files })
    }

    fn extensions(&self) -> &[&str] {
        &["list"]
    }
}

#[cfg(test)]
mod tests {
    use crate::script::cmd::*;
    use crate::script::cmd::*;

    #[test]
    fn deserialize_minimal_script() {
        // adjacently-tagged format: (cmd: variant, args: (...))
        let ron = r#"VnScript(
    version: V1,
    meta: ScriptMeta(name: Some("demo"), next_script: None),
    instructions: [
        (cmd: label, args: (name: "start")),
        (cmd: dialogue, args: (speaker: Some("Narrator"), text: "Hello", voice: None)),
        (cmd: halt),
    ],
)"#;
        let script: VnScript = ron::de::from_str(ron).unwrap();
        assert_eq!(script.version, ScriptVersion::V1);
        assert_eq!(script.meta.name.as_deref(), Some("demo"));
        assert_eq!(script.instructions.len(), 3);
    }

    #[test]
    fn deserialize_with_defaults() {
        let ron = r#"VnScript(
    version: V1,
    meta: ScriptMeta(),
    instructions: [(cmd: return)],
)"#;
        let script: VnScript = ron::de::from_str(ron).unwrap();
        assert!(script.meta.name.is_none());
        assert!(script.meta.next_script.is_none());
    }
}
