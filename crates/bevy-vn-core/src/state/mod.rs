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
