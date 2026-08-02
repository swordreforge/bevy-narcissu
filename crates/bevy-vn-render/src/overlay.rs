//! Screen overlay: flash, fade.

use bevy::prelude::*;
use bevy_vn_core::messages::ScreenEffectEvent;

#[derive(Component)] pub struct OverlayMarker;

#[derive(Resource, Default)]
pub struct OverlayState { pub entity: Option<Entity> }

pub struct OverlayPlugin;
impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayState>()
            .add_systems(Update, handle_screen_effect);
    }
}

fn parse_color(s: &Option<String>) -> Color {
    match s.as_deref() {
        None | Some("") | Some("0") => Color::NONE,
        Some("White") | Some("white") | Some("2") => Color::WHITE,
        Some("Black") | Some("black") | Some("1") => Color::BLACK,
        _ => Color::WHITE,
    }
}

fn handle_screen_effect(
    mut reader: MessageReader<ScreenEffectEvent>,
    mut commands: Commands,
    mut state: ResMut<OverlayState>,
    q_overlay: Query<Entity, With<OverlayMarker>>,
) {
    for event in reader.read() {
        let color = parse_color(&event.color);
        if color == Color::NONE {
            for e in q_overlay.iter() { commands.entity(e).despawn(); }
            state.entity = None;
            continue;
        }
        if let Some(e) = state.entity {
            commands.entity(e).insert(BackgroundColor(color));
        } else {
            let e = commands.spawn((
                OverlayMarker,
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                BackgroundColor(color),
                ZIndex(i32::MAX),
            )).id();
            state.entity = Some(e);
        }
    }
}
