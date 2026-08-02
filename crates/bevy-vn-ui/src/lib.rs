//! bevy-vn-ui — UI plugin for the Bevy VN engine.
//!
//! Provides all visual novel screens: dialogue, choice, backlog,
//! title, settings, save/load, gallery.
//!
//! Consumes state events from bevy-vn-core and reads VnTheme for styling.

pub mod screen;
pub mod theme;
pub mod dialogue;
pub mod choice;
pub mod backlog;
pub mod title;
pub mod settings;
pub mod save_load_ui;
pub mod gallery;

use bevy::prelude::*;

use dialogue::DialoguePlugin;
use choice::ChoicePlugin;

/// UI plugin. Must be added AFTER VnCorePlugin.
pub struct VnUiPlugin;

impl Plugin for VnUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            theme::VnThemePlugin,
            DialoguePlugin,
            ChoicePlugin,
            backlog::BacklogPlugin,
            title::TitlePlugin,
            settings::SettingsPlugin,
            save_load_ui::SaveLoadUiPlugin,
            gallery::GalleryPlugin,
        ));
    }
}
