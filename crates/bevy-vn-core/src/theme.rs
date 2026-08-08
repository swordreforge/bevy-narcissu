//! Theme system — three-layer override model.
//!
//! Layer 1: VnTheme::default() (built-in defaults)
//! Layer 2: assets/theme.ron (RON config file, loaded via AssetServer)
//! Layer 3: bsn!{} component-level override (per-spawn)
//!
//! Theme values with type f32 for font sizes are converted to FontSize
//! via `px()` at the use site (see bevy-vn-ui).
//!
//! VnTheme doubles as a Bevy [`Asset`] so theme.ron is loaded through the
//! AssetServer — this fixes the old CWD-relative `std::fs` read (which
//! silently fell back to defaults) and works identically on wasm.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Global theme configuration resource.
#[derive(Resource, Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct VnTheme {
    pub dialogue: DialogueTheme,
    pub choice: ChoiceTheme,
    pub title: TitleTheme,
    pub settings: SettingsTheme,
    pub save_load: SaveLoadTheme,
    pub backlog: BacklogTheme,
    pub gallery: GalleryTheme,
    pub transitions: TransitionTheme,
    pub fonts: FontTheme,
    pub colors: ColorTheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTheme {
    /// Distance from screen bottom
    pub margin_bottom: f32,
    /// Dialogue box height
    pub height: f32,
    /// Background color [r, g, b, a]
    pub background_color: [f32; 4],
    /// Text color [r, g, b, a]
    pub text_color: [f32; 4],
    /// Speaker name color [r, g, b, a]
    pub speaker_color: [f32; 4],
    /// Speaker name box width
    pub speaker_box_width: f32,
    /// Font size in pixels (wrapped with px() at use site)
    pub font_size: f32,
    /// Speaker name font size in pixels
    pub speaker_font_size: f32,
    /// Inner padding [left, right, top, bottom]
    pub padding: [f32; 4],
    /// Text reveal speed (chars/sec). Overrides VnEngineConfig.text_speed when set.
    pub text_speed: Option<f64>,
    /// Window design variant
    pub design: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChoiceTheme {
    #[serde(default = "default_max_visible")]
    pub max_visible: usize,
    #[serde(default = "default_item_height")]
    pub item_height: f32,
    #[serde(default = "default_font_size_24")]
    pub font_size: f32,
    #[serde(default = "default_choice_padding")]
    pub padding: [f32; 4],
}
fn default_max_visible() -> usize { 6 }
fn default_item_height() -> f32 { 48.0 }
fn default_font_size_24() -> f32 { 24.0 }
fn default_choice_padding() -> [f32; 4] { [12.0, 24.0, 12.0, 24.0] }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleTheme {
    pub title_font_size: f32,
    pub menu_font_size: f32,
}
impl Default for TitleTheme {
    fn default() -> Self {
        Self { title_font_size: 56.0, menu_font_size: 28.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsTheme {
    pub font_size: f32,
    pub slider_width: f32,
}
impl Default for SettingsTheme {
    fn default() -> Self {
        Self { font_size: 24.0, slider_width: 300.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLoadTheme {
    pub font_size: f32,
    pub thumbnail_width: f32,
    pub thumbnail_height: f32,
}
impl Default for SaveLoadTheme {
    fn default() -> Self {
        Self { font_size: 24.0, thumbnail_width: 200.0, thumbnail_height: 112.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklogTheme {
    pub font_size: f32,
    pub max_entries: usize,
}
impl Default for BacklogTheme {
    fn default() -> Self {
        Self { font_size: 22.0, max_entries: 50 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryTheme {
    pub thumbnail_width: f32,
    pub thumbnail_height: f32,
    pub columns: usize,
}
impl Default for GalleryTheme {
    fn default() -> Self {
        Self { thumbnail_width: 240.0, thumbnail_height: 135.0, columns: 4 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionTheme {
    pub fade_duration: f32,
    pub fg_fade_duration: f32,
    pub cg_fade_duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTheme {
    pub menu_bg: [f32; 4],
    pub menu_text: [f32; 4],
    pub button_normal: [f32; 4],
    pub button_hover: [f32; 4],
    pub button_press: [f32; 4],
    pub choice_highlight: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontTheme {
    /// Default UI font path
    pub default: String,
    /// Dialogue font (None = use default)
    pub dialogue: Option<String>,
    /// Title font (None = use default)
    pub title: Option<String>,
    /// Monospace font
    pub mono: Option<String>,
}

impl Default for VnTheme {
    fn default() -> Self {
        Self {
            dialogue: DialogueTheme {
                margin_bottom: 20.0,
                height: 180.0,
                background_color: [0.0, 0.0, 0.0, 0.7],
                text_color: [1.0, 1.0, 1.0, 1.0],
                speaker_color: [0.8, 0.8, 1.0, 1.0],
                speaker_box_width: 200.0,
                font_size: 28.0,
                speaker_font_size: 22.0,
                padding: [20.0, 20.0, 20.0, 20.0],
                text_speed: None,
                design: "default".into(),
            },
            choice: ChoiceTheme {
                max_visible: 6,
                item_height: 48.0,
                font_size: 24.0,
                padding: [12.0, 24.0, 12.0, 24.0],
            },
            transitions: TransitionTheme {
                fade_duration: 1.0,
                fg_fade_duration: 0.5,
                cg_fade_duration: 1.0,
            },
            fonts: FontTheme {
                default: "fonts/default.ttf".into(),
                dialogue: None,
                title: None,
                mono: None,
            },
            colors: ColorTheme {
                menu_bg: [0.05, 0.05, 0.1, 0.95],
                menu_text: [0.9, 0.9, 0.95, 1.0],
                button_normal: [0.15, 0.15, 0.25, 1.0],
                button_hover: [0.25, 0.25, 0.40, 1.0],
                button_press: [0.35, 0.35, 0.50, 1.0],
                choice_highlight: [0.3, 0.5, 0.8, 1.0],
            },
            title: TitleTheme::default(),
            settings: SettingsTheme::default(),
            save_load: SaveLoadTheme::default(),
            backlog: BacklogTheme::default(),
            gallery: GalleryTheme::default(),
        }
    }
}

/// AssetLoader for `theme.ron` — parses the RON text into a [`VnTheme`].
#[derive(TypePath)]
pub struct VnThemeLoader;

impl AssetLoader for VnThemeLoader {
    type Asset = VnTheme;
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
        ron::de::from_bytes(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}
