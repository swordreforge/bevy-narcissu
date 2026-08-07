//! Game state machine.
//!
//! Uses Bevy States + SubStates for menu nesting.

use bevy::prelude::*;

/// Top-level engine state.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum VnAppState {
    #[default]
    Boot,
    Splash,
    Title,
    /// Main gameplay loop — all script execution happens here.
    Gameplay,
    /// Menu container. Specific screens use VnMenuState sub-state.
    Menu,
}

/// Menu sub-states (active when VnAppState::Menu).
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(VnAppState = VnAppState::Menu)]
pub enum VnMenuState {
    #[default]
    Main,
    SaveLoad,
    Settings,
    Gallery,
    Backlog,
    RouteSelect,
    ChapterSelect,
    StoryDetail,
    AfterStory,
}

/// Screen transition phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionPhase {
    #[default]
    Idle,
    FadingOut,
    FadingIn,
}

/// Centralized transition state.
#[derive(Resource, Default)]
pub struct VnTransition {
    pub phase: TransitionPhase,
    pub pending: Option<VnAppState>,
    pub duration: f32,
}

/// Which operation the save/load screen performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveLoadKind {
    #[default]
    Load,
    Save,
}

/// Where to return after closing the save/load screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveLoadReturn {
    #[default]
    Title,
    Settings,
    Gameplay,
}

/// Runtime mode for the save/load screen. `active` gates both the UI
/// spawn/despawn and script advancement, so the in-game menu can stay
/// inside `VnAppState::Gameplay` without wiping the scene.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SaveLoadMode {
    pub active: bool,
    pub kind: SaveLoadKind,
    pub return_to: SaveLoadReturn,
}

/// Runtime mode for the in-game system menu (原作 adv_menu / ui_menu):
/// opened with right-click / F2 / ESC while inside `VnAppState::Gameplay`.
/// `active` gates the menu UI and blocks script advancement, mirroring
/// the original `menu_check()` semantics without leaving Gameplay.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameplayMenuMode {
    pub active: bool,
}

/// Runtime mode for the settings panel opened *from the in-game menu*
/// (原作 adv_config). Stays inside `VnAppState::Gameplay` so the scene
/// survives; the menu-state path (`VnMenuState::Settings`) is unchanged.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettingsOverlayMode {
    pub active: bool,
}

/// Skip mode toggled from the in-game menu (原作 adv_skip). When active,
/// the runner emits `AdvanceEvent(Skip)` at a fixed cadence so dialogue
/// waits are fast-forwarded.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipMode {
    pub active: bool,
}
