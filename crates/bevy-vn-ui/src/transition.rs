//! Unified menu transition — fullscreen black overlay that covers the old
//! screen, commits the pending state, then reveals the new screen.
//!
//! All UI state switches (title → menu, menu ↔ sub-menu, menu → gameplay,
//! gameplay → title) go through `VnTransition` instead of setting
//! `NextState` directly. Durations mirror the original engine's
//! `quickfade=250` (list_windows.tbl): 0.15s cover + 0.25s reveal.

use bevy::prelude::*;
use bevy_vn_core::state::{TransitionPhase, VnAppState, VnMenuState, VnTransition};

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
    if transition.phase != TransitionPhase::Idle {
        return;
    }
    transition.pending = app;
    transition.pending_menu = menu;
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
