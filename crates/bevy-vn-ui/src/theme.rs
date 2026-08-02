//! Theme loading plugin.
//!
//! Loads VnTheme from RON file at startup, falls back to defaults.

use bevy::prelude::*;
use bevy_vn_core::theme::VnTheme;

pub struct VnThemePlugin;

impl Plugin for VnThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_theme);
    }
}

fn load_theme(mut commands: Commands) {
    let theme = std::fs::read_to_string("assets/theme.ron")
        .ok()
        .and_then(|s| ron::from_str::<VnTheme>(&s).ok())
        .unwrap_or_else(|| {
            info!("No theme.ron found, using defaults");
            VnTheme::default()
        });
    commands.insert_resource(theme);
}
