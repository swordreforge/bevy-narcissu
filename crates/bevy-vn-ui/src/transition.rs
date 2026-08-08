//! Unified menu transition — fullscreen black overlay that covers the old
//! screen, commits the pending state, then reveals the new screen.
//!
//! All UI state switches (title → menu, menu ↔ sub-menu, menu → gameplay,
//! gameplay → title) go through `VnTransition` instead of setting
//! `NextState` directly. Durations mirror the original engine's
//! `quickfade=250` (list_windows.tbl): 0.15s cover + 0.25s reveal.

use bevy::prelude::*;
use bevy_vn_core::state::{
    GameplayMenuMode, OverlayToggle, SaveLoadMode, SettingsOverlayMode, TransitionPhase, VnAppState,
    VnMenuState, VnTransition,
};

/// Cover time before the pending state is committed.
const FADE_OUT_SECS: f32 = 0.15;
/// Reveal time after the new screen has spawned.
const FADE_IN_SECS: f32 = 0.25;

#[derive(Component)]
struct FadeOverlay;

/// Request a menu transition. Both options are optional so callers can
/// switch only the app state (e.g. menu → gameplay) or only the menu
/// sub-state (e.g. settings → save/load).
pub fn request_transition(
    transition: &mut VnTransition,
    app: Option<VnAppState>,
    menu: Option<VnMenuState>,
) {
    request_fade(transition, app, menu, Vec::new());
}

/// Request an in-game overlay transition (menu / settings / save-load
/// opened from Gameplay). Overlays are resource flags, so instead of a
/// state switch the toggles are applied when the fade-out completes.
pub fn request_overlay(transition: &mut VnTransition, toggles: Vec<OverlayToggle>) {
    request_fade(transition, None, None, toggles);
}

/// Combine a state switch with overlay toggles in one fade (e.g. leaving
/// gameplay for the title while closing the in-game menu).
pub fn request_transition_with_overlay(
    transition: &mut VnTransition,
    app: Option<VnAppState>,
    menu: Option<VnMenuState>,
    toggles: Vec<OverlayToggle>,
) {
    request_fade(transition, app, menu, toggles);
}

fn request_fade(
    transition: &mut VnTransition,
    app: Option<VnAppState>,
    menu: Option<VnMenuState>,
    overlays: Vec<OverlayToggle>,
) {
    if transition.phase != TransitionPhase::Idle {
        return;
    }
    transition.pending = app;
    transition.pending_menu = menu;
    transition.pending_overlays = overlays;
    transition.phase = TransitionPhase::FadingOut;
    transition.duration = FADE_OUT_SECS;
    transition.elapsed = 0.0;
}

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_overlay)
            .add_systems(Update, drive_transition);
    }
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        FadeOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ZIndex(i32::MAX),
    ));
}

fn drive_transition(
    time: Res<Time>,
    mut transition: ResMut<VnTransition>,
    mut next_app: ResMut<NextState<VnAppState>>,
    mut next_menu: ResMut<NextState<VnMenuState>>,
    mut menu_mode: ResMut<GameplayMenuMode>,
    mut settings_overlay: ResMut<SettingsOverlayMode>,
    mut save_mode: ResMut<SaveLoadMode>,
    mut q: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    if transition.phase == TransitionPhase::Idle {
        return;
    }
    let Ok(mut bg) = q.single_mut() else { return };

    transition.elapsed += time.delta_secs();
    let t = (transition.elapsed / transition.duration).clamp(0.0, 1.0);

    match transition.phase {
        TransitionPhase::FadingOut => {
            bg.0.set_alpha(t);
            if t >= 1.0 {
                if let Some(app) = transition.pending.take() {
                    next_app.set(app);
                }
                if let Some(menu) = transition.pending_menu.take() {
                    next_menu.set(menu);
                }
                for toggle in transition.pending_overlays.drain(..) {
                    apply_overlay(toggle, &mut menu_mode, &mut settings_overlay, &mut save_mode);
                }
                transition.phase = TransitionPhase::FadingIn;
                transition.duration = FADE_IN_SECS;
                transition.elapsed = 0.0;
            }
        }
        TransitionPhase::FadingIn => {
            bg.0.set_alpha(1.0 - t);
            if t >= 1.0 {
                transition.phase = TransitionPhase::Idle;
            }
        }
        TransitionPhase::Idle => {}
    }
}

fn apply_overlay(
    toggle: OverlayToggle,
    menu_mode: &mut GameplayMenuMode,
    settings_overlay: &mut SettingsOverlayMode,
    save_mode: &mut SaveLoadMode,
) {
    match toggle {
        OverlayToggle::GameplayMenu(v) => menu_mode.active = v,
        OverlayToggle::SettingsOverlay(v) => settings_overlay.active = v,
        OverlayToggle::SaveLoad { kind, return_to } => {
            *save_mode = SaveLoadMode {
                active: true,
                kind,
                return_to,
            };
        }
        OverlayToggle::SaveLoadClose => *save_mode = SaveLoadMode::default(),
    }
}
