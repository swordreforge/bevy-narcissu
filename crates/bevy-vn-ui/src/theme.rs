//! Theme loading plugin.
//!
//! Loads VnTheme from assets/theme.ron via the AssetServer (works on native
//! and wasm alike), falls back to VnCorePlugin's default VnTheme when the
//! file is missing or fails to parse.

use bevy::prelude::*;
use bevy_vn_core::theme::{VnTheme, VnThemeLoader};

const THEME_PATH: &str = "theme.ron";

#[derive(Resource)]
struct ThemeHandle(Handle<VnTheme>);

pub struct VnThemePlugin;

impl Plugin for VnThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<VnTheme>();
        app.register_asset_loader(VnThemeLoader);
        app.add_systems(Startup, load_theme);
        app.add_systems(Update, apply_loaded_theme);
    }
}

fn load_theme(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(ThemeHandle(asset_server.load(THEME_PATH)));
}

fn apply_loaded_theme(
    mut commands: Commands,
    handle: Res<ThemeHandle>,
    assets: Res<Assets<VnTheme>>,
    mut applied: Local<bool>,
) {
    if *applied { return; }
    if let Some(theme) = assets.get(&handle.0) {
        commands.insert_resource(theme.clone());
        info!("theme.ron loaded");
        *applied = true;
    }
}
