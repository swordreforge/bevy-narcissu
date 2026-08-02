//! Generic VnScreen trait for standard screen lifecycle.
//!
//! Any screen implementing VnScreen gets OnEnter spawn + OnExit despawn
//! + Update handling via a marker component query.

use bevy::prelude::*;

/// Marker component for entities belonging to a screen.
#[derive(Component)]
pub struct ScreenMarker;

/// Register a screen's enter/exit systems for a given state.
pub fn register_screen<S: States>(
    app: &mut App,
    state: S,
    enter: impl IntoSystem<(), (), ()>,
    update: impl IntoSystem<(), (), ()>,
    exit: impl IntoSystem<(), (), ()>,
) {
    app.add_systems(OnEnter(state.clone()), (
        enter,
        |_commands: Commands| {
            // Marker spawn helper — actual enter system should spawn entities
        },
    ).chain())
    .add_systems(Update, update.run_if(in_state(state.clone())))
    .add_systems(OnExit(state), (exit, despawn_screen_markers));
}

fn despawn_screen_markers(mut commands: Commands, q: Query<Entity, With<ScreenMarker>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
