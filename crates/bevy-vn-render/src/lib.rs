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
    pub fn fg(&self, char_id: &str, expr: &str) -> String { format!("{}/{}/{}.basisu.ktx2", self.fg_dir, char_id, expr) }
    pub fn bg(&self, image: &str) -> String { format!("{}/{}.basisu.ktx2", self.bg_dir, image) }
    pub fn cg(&self, image: &str) -> String { format!("{}/{}.basisu.ktx2", self.cg_dir, image) }
    pub fn sprite(&self, id: &str) -> String { format!("{}/{}.basisu.ktx2", self.anime_dir, id) }

    /// Resolve an asset path using `provider` when available; otherwise fall
    /// back to `AssetPathProvider::default()`.  This eliminates the duplicated
    /// `.as_ref().map(|p| p.X(…)).unwrap_or_else(|| format!(…))` pattern at
    /// every call site.
    pub fn resolve(provider: Option<&AssetPathProvider>, f: impl Fn(&AssetPathProvider) -> String) -> String {
        f(provider.unwrap_or(&Self::default()))
    }
}

pub struct VnRenderPlugin { pub fg_slots: usize }
impl Default for VnRenderPlugin { fn default() -> Self { Self { fg_slots: 3 } } }

impl Plugin for VnRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(fg::FgSlotConfig { slot_count: self.fg_slots });
        app.add_plugins((BgPlugin, FgPlugin, CgPlugin, OverlayPlugin, SpriteOverlayPlugin))
            .add_systems(OnExit(bevy_vn_core::state::VnAppState::Gameplay), clear_gameplay_scene);
    }
}

/// Wipe every gameplay-stage visual on exit (backgrounds, characters, CG,
/// overlays, sprites) and reset the per-subsystem bookkeeping, so returning
/// to the title never shows leftover story art behind the menu.
fn clear_gameplay_scene(
    mut commands: Commands,
    mut bg: ResMut<bg::BgState>,
    mut cg: ResMut<cg::CgState>,
    mut fg: ResMut<fg::FgSlotState>,
    mut ov: ResMut<overlay::OverlayState>,
    q_bg: Query<Entity, With<bg::BgMarker>>,
    q_cg: Query<Entity, With<cg::CgMarker>>,
    q_fg: Query<Entity, With<fg::FgSlotMarker>>,
    q_ov: Query<Entity, With<overlay::OverlayMarker>>,
    q_sp: Query<Entity, With<sprite::SpriteOverlayMarker>>,
) {
    for e in q_bg.iter().chain(q_cg.iter()).chain(q_fg.iter()).chain(q_ov.iter()).chain(q_sp.iter()) {
        commands.entity(e).despawn();
    }
    *bg = bg::BgState::default();
    *cg = cg::CgState::default();
    *fg = fg::FgSlotState::default();
    *ov = overlay::OverlayState::default();
}
