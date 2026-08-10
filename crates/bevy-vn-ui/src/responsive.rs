//! Responsive canvas scaling for fixed-logical-resolution screens.
//!
//! All VN screens (title, settings, gallery, save/load, ...) are laid out in
//! a fixed 960x540 logical canvas. This module lets that canvas scale up to
//! fit the actual window: mark the canvas node with [`ResponsiveCanvas`] and
//! this plugin keeps its [`UiTransform`] scale at `min(win_w/960, win_h/540)`
//! so the whole 960x540 layout (every Val::Px inside it) scales uniformly,
//! centered, and interaction picking follows the scaled transform.

use bevy::prelude::*;
use bevy::window::WindowResized;

/// Logical canvas size every VN screen is designed against.
pub const LOGICAL_WIDTH: f32 = 960.0;
pub const LOGICAL_HEIGHT: f32 = 540.0;

/// Marks a fixed-size (960x540) canvas node that should scale to fit the window.
#[derive(Component, Default)]
pub struct ResponsiveCanvas;

/// Scale factor applied to all `ResponsiveCanvas` nodes. Read by other systems
/// that need the current scale (e.g. converting screen-space coordinates).
#[derive(Resource, Deref, DerefMut)]
pub struct UiCanvasScale(pub f32);

impl Default for UiCanvasScale {
    fn default() -> Self {
        Self(1.0)
    }
}

pub struct ResponsivePlugin;

impl Plugin for ResponsivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCanvasScale>()
            .add_systems(PreUpdate, (apply_canvas_scale, handle_window_resize));
    }
}

fn apply_canvas_scale(
    scale: Res<UiCanvasScale>,
    mut q: Query<&mut UiTransform, With<ResponsiveCanvas>>,
) {
    let s = scale.0;
    for mut transform in &mut q {
        if transform.scale != Vec2::splat(s) {
            transform.scale = Vec2::splat(s);
        }
    }
}

fn handle_window_resize(
    mut resize_messages: MessageReader<WindowResized>,
    windows: Query<&Window>,
    mut scale: ResMut<UiCanvasScale>,
) {
    let Some(window) = windows.iter().next() else { return };
    let new_scale = (window.width() / LOGICAL_WIDTH).min(window.height() / LOGICAL_HEIGHT);
    if new_scale > 0.0 && new_scale != scale.0 {
        scale.0 = new_scale;
    }
    resize_messages.clear();
}
