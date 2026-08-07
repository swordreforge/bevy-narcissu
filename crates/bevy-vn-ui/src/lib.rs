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
pub mod settings_data;
pub mod chapter_names;
pub mod save_load_ui;
pub mod gallery;
pub mod hotspot;
pub mod gameplay_menu;
pub mod character_select;
pub mod chapter_select;
pub mod story_detail;
pub mod brand_logo;

use bevy::prelude::*;
use bevy_vn_core::messages::PlaySeEvent;

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
            hotspot::HotspotPlugin,
            character_select::CharacterSelectPlugin,
            chapter_select::ChapterSelectPlugin,
            story_detail::StoryDetailPlugin,
            brand_logo::BrandLogoPlugin,
            gameplay_menu::GameplayMenuPlugin,
        ))
        .add_systems(Update, force_any_character_linebreak)
        .add_systems(Update, play_button_click_sfx)
        .add_systems(OnExit(bevy_vn_core::state::VnAppState::Gameplay), hide_dialogue_ui);
    }
}

/// Original engine fires the system ok SE (sysse ok -> system_cancel.ogg)
/// on every button press; mirror that for all UI buttons.
fn play_button_click_sfx(
    q: Query<&Interaction, (With<Button>, Changed<Interaction>)>,
    mut se: MessageWriter<PlaySeEvent>,
) {
    if q.iter().any(|i| *i == Interaction::Pressed) {
        se.write(PlaySeEvent {
            file: "system_cancel".to_string(),
            channel: Some(0),
            volume: None,
        });
    }
}

/// Dialogue box is spawned once at startup and stays alive; hide it whenever
/// gameplay ends so a finished story never leaves the box over the title.
fn hide_dialogue_ui(
    state: Option<Res<dialogue::DialogueUiState>>,
    mut q_root: Query<&mut Node, With<dialogue::DialogueRoot>>,
) {
    let Some(state) = state else { return };
    let Some(root) = state.root else { return };
    if let Ok(mut node) = q_root.get_mut(root) {
        node.display = Display::None;
    }
}

/// Override Bevy's default word-boundary line breaking (ICU4X UAX#14).
/// The bundled icu data has no Japanese segmentation model, so every CJK
/// text layout logs "No segmentation model for language: ja" — switch to
/// per-character breaking, which is visually identical for CJK text and
/// avoids the ICU4X lookup entirely.
fn force_any_character_linebreak(mut q: Query<&mut TextLayout>) {
    for mut layout in &mut q {
        if layout.linebreak != LineBreak::AnyCharacter {
            layout.linebreak = LineBreak::AnyCharacter;
        }
    }
}
