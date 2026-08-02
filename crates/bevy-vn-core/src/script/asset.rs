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
