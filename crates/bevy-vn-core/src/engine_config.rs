//! Global engine configuration.
//!
//! Inserted as a Resource by VnCorePlugin.

use bevy::prelude::*;

/// Global engine configuration resource.
#[derive(Resource, Debug, Clone)]
pub struct VnEngineConfig {
    /// Window logical resolution
    pub resolution: (f32, f32),
    /// Script directory relative to assets/
    pub script_dir: String,
    /// Save directory
    pub save_dir: String,
    /// Default font path
    pub default_font: String,
    /// Text reveal speed in characters per second (global default,
    /// can be overridden per-theme via VnTheme.dialogue.text_speed)
    pub text_speed: f64,
    /// Auto-mode delay in seconds
    pub auto_delay: f64,
}

impl Default for VnEngineConfig {
    fn default() -> Self {
        Self {
            resolution: (1280.0, 720.0),
            script_dir: "scripts".into(),
            save_dir: "saves".into(),
            default_font: "fonts/default.ttf".into(),
            text_speed: 50.0,
            auto_delay: 2.0,
        }
    }
}
