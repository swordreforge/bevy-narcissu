//! bevy-vn-core — Core engine for Bevy 0.19 visual novel games.
//!
//! This crate provides the foundation: script IR, interpreter, state machine,
//! event types for cross-plugin communication, theme system, and save/load abstractions.
//!
//! It has NO dependencies on rendering, audio, or UI — those are separate crates
//! that consume the Events and traits defined here.

pub mod assets;
pub mod engine_config;
pub mod messages;
pub mod runner;
pub mod save;
pub mod script;
pub mod state;
pub mod theme;

use bevy::prelude::*;

use engine_config::VnEngineConfig;
use script::VnScriptLoader;
use state::{VnAppState, VnTransition};
use theme::VnTheme;

/// Core engine plugin. Must be added before any subsystem plugin
/// (render, audio, UI, save, video).
pub struct VnCorePlugin {
    pub config: VnEngineConfig,
}

impl Plugin for VnCorePlugin {
    fn build(&self, app: &mut App) {
        // ── Resources ──
        app.insert_resource(self.config.clone());
        app.insert_resource(VnTheme::default());
        app.insert_resource(VnTransition::default());

        // ── State machine ──
        app.init_state::<VnAppState>();

        // ── Asset loader for .vnscript.ron ──
        app.init_asset::<script::VnScriptAsset>();
        app.register_asset_loader(VnScriptLoader);

        // ── Event types (registered once, consumed by subsystem plugins) ──
        app.add_message::<messages::AdvanceEvent>();
        app.add_message::<messages::TransitionRequest>();
        app.add_message::<messages::TransitionComplete>();

        // Rendering events
        app.add_message::<messages::SetBgEvent>();
        app.add_message::<messages::ShowFgEvent>();
        app.add_message::<messages::HideFgEvent>();
        app.add_message::<messages::ShowFaceEvent>();
        app.add_message::<messages::HideFaceEvent>();
        app.add_message::<messages::ShowCgEvent>();
        app.add_message::<messages::HideCgEvent>();
        app.add_message::<messages::ScrollBgEvent>();
        app.add_message::<messages::SpriteEvent>();
        app.add_message::<messages::SpriteEffectEvent>();
        app.add_message::<messages::ScreenEffectEvent>();

        // Audio events
        app.add_message::<messages::PlayBgmEvent>();
        app.add_message::<messages::StopBgmEvent>();
        app.add_message::<messages::PlaySeEvent>();
        app.add_message::<messages::StopSeEvent>();
        app.add_message::<messages::PlayVoiceEvent>();
        app.add_message::<messages::SetVolumeEvent>();

        // Video events
        app.add_message::<messages::PlayMovieEvent>();
        app.add_message::<messages::StopMovieEvent>();
        app.add_message::<messages::SpriteVideoEvent>();
        app.add_message::<messages::StopSpriteVideoEvent>();

        // State mutation events
        app.add_message::<messages::UnlockCgEvent>();
        app.add_message::<messages::UnlockBgmEvent>();
        app.add_message::<messages::AffectionChangeEvent>();
        app.add_message::<messages::BacklogPushEvent>();
        app.add_message::<messages::SavePointEvent>();

        // UI state events
        app.add_message::<messages::DialogueStateEvent>();
        app.add_message::<messages::ClearDialogueEvent>();
        app.add_message::<messages::ChoiceStateEvent>();
        app.add_message::<messages::ChoiceSelectedEvent>();

        // ── Script runner (bridges ScriptEngine → Messages) ──
        app.add_plugins(runner::ScriptRunnerPlugin);
    }
}

/// Re-exports for convenience.
pub mod prelude {
    pub use crate::assets::{BgmPathInfo, DefaultAssetProvider, VnAssetProvider};
    pub use crate::engine_config::VnEngineConfig;
    pub use crate::messages::*;
    pub use crate::save::{SaveData, SaveStateProvider};
    pub use crate::script::{
        ConditionOp, ExpressionError, FgPosition, ScreenEffectKind, ScriptCmd, ScriptEngine,
        ScriptMeta, ScriptVersion, Transition, VnScript, VnScriptAsset, VnScriptLoader,
    };
    pub use crate::state::{TransitionPhase, VnAppState, VnMenuState, VnTransition};
    pub use crate::theme::{
        BacklogTheme, ChoiceTheme, ColorTheme, DialogueTheme, FontTheme, GalleryTheme,
        SaveLoadTheme, SettingsTheme, TitleTheme, TransitionTheme, VnTheme,
    };
    pub use crate::VnCorePlugin;
}
