//! Asset path resolution trait.
//!
//! Game projects implement this trait to define their own
//! asset organization. The engine renders/plays whatever paths
//! the provider returns — no hardcoded directory conventions.

/// Information about a BGM track, including A/B loop segments if applicable.
#[derive(Debug, Clone)]
pub struct BgmPathInfo {
    /// Path to the intro segment (A part).
    pub intro: String,
    /// Path to the loop segment (B part). None if no split.
    pub loop_segment: Option<String>,
}

/// Trait for resolving logical asset IDs to filesystem paths.
///
/// Implement this and insert as a Resource. All engine subsystems
/// that load assets (render, audio) use this trait via `Res<dyn VnAssetProvider>`.
pub trait VnAssetProvider: Send + Sync {
    /// Resolve a character sprite expression to an image path.
    fn fg_path(&self, char_id: &str, expression: &str) -> String;

    /// Resolve a background image ID to a path.
    fn bg_path(&self, image: &str) -> String;

    /// Resolve a CG/event image ID to a path.
    fn cg_path(&self, image: &str) -> String;

    /// Resolve a BGM ID to its path info (A/B segments).
    fn bgm_path(&self, id: &str) -> BgmPathInfo;

    /// Resolve a sound effect ID to a path.
    fn se_path(&self, file: &str) -> String;

    /// Resolve a voice file ID to a path.
    fn voice_path(&self, file: &str) -> String;

    /// Resolve a sprite overlay ID to a path (optional).
    fn sprite_path(&self, id: &str) -> Option<String>;
}

/// Default implementation: simple prefix-based path builder.
pub struct DefaultAssetProvider {
    pub fg_dir: String,
    pub bg_dir: String,
    pub cg_dir: String,
    pub bgm_dir: String,
    pub se_dir: String,
    pub voice_dir: String,
}

impl VnAssetProvider for DefaultAssetProvider {
    fn fg_path(&self, char_id: &str, expression: &str) -> String {
        format!("{}/{}/{}.png", self.fg_dir, char_id, expression)
    }

    fn bg_path(&self, image: &str) -> String {
        format!("{}/{}.png", self.bg_dir, image)
    }

    fn cg_path(&self, image: &str) -> String {
        format!("{}/{}.png", self.cg_dir, image)
    }

    fn bgm_path(&self, id: &str) -> BgmPathInfo {
        let intro = format!("{}/bgm_{}_a.ogg", self.bgm_dir, id);
        let loop_seg = format!("{}/bgm_{}_b.ogg", self.bgm_dir, id);
        BgmPathInfo {
            intro,
            loop_segment: Some(loop_seg),
        }
    }

    fn se_path(&self, file: &str) -> String {
        format!("{}/{}.ogg", self.se_dir, file)
    }

    fn voice_path(&self, file: &str) -> String {
        format!("{}/{}.ogg", self.voice_dir, file)
    }

    fn sprite_path(&self, id: &str) -> Option<String> {
        Some(format!("image/anime/{}.png", id))
    }
}
