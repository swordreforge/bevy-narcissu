//! bevy-vn-render — Rendering plugin for the Bevy VN engine.

pub mod bg;
pub mod cg;
pub mod fg;
pub mod overlay;
pub mod sprite;

use bevy::prelude::*;
use bg::BgPlugin;
use cg::CgPlugin;
use fg::FgPlugin;
use overlay::OverlayPlugin;
use sprite::SpriteOverlayPlugin;

/// Asset path resolver. Insert as a resource, or omit for defaults.
#[derive(Resource, Clone)]
pub struct AssetPathProvider {
    pub fg_dir: String,
    pub bg_dir: String,
    pub cg_dir: String,
    pub anime_dir: String,
}

impl Default for AssetPathProvider {
    fn default() -> Self {
        Self {
            fg_dir: "image/obj".into(),
            bg_dir: "image/bg".into(),
            cg_dir: "image/ev".into(),
            anime_dir: "image/anime".into(),
        }
    }
}

impl AssetPathProvider {
    pub fn fg(&self, char_id: &str, expr: &str) -> String { format!("{}/{}/{}.png", self.fg_dir, char_id, expr) }
    pub fn bg(&self, image: &str) -> String { format!("{}/{}.png", self.bg_dir, image) }
    pub fn cg(&self, image: &str) -> String { format!("{}/{}.png", self.cg_dir, image) }
    pub fn sprite(&self, id: &str) -> String { format!("{}/{}.png", self.anime_dir, id) }
}

pub struct VnRenderPlugin { pub fg_slots: usize }
impl Default for VnRenderPlugin { fn default() -> Self { Self { fg_slots: 3 } } }

impl Plugin for VnRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(fg::FgSlotConfig { slot_count: self.fg_slots });
        app.add_plugins((BgPlugin, FgPlugin, CgPlugin, OverlayPlugin, SpriteOverlayPlugin));
    }
}
