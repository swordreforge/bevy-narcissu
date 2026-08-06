//! Settings screen.

use bevy::prelude::*;
use bevy_vn_core::state::VnMenuState;
use bevy_vn_core::theme::VnTheme;

#[derive(Component)]
struct SettingsScreen;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnMenuState::Settings), spawn_settings)
            .add_systems(OnExit(VnMenuState::Settings), despawn_settings);
    }
}

fn spawn_settings(mut commands: Commands, theme: Option<Res<VnTheme>>) {
    let st = theme.as_ref().map(|t| t.settings.clone()).unwrap_or_default();

    commands.spawn((
        SettingsScreen,
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 0.95)),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("Settings"),
            TextFont { font_size: FontSize::Px(st.font_size.max(1.0)), ..default() },
            TextColor(Color::WHITE),
        ));
        // Placeholder: volume sliders, text speed, etc.
        parent.spawn((
            Text::new("Audio Volume: 80%"),
            TextFont { font_size: FontSize::Px(st.font_size * 0.8), ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
        parent.spawn((
            Text::new("Text Speed: 50 chars/sec"),
            TextFont { font_size: FontSize::Px(st.font_size * 0.8), ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
        parent.spawn((
            Button,
            Node {
                width: Val::Px(200.0), height: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.2, 1.0)),
        ))
        .with_child((
            Text::new("Back"),
            TextFont { font_size: FontSize::Px(st.font_size * 0.8), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn despawn_settings(mut commands: Commands, q: Query<Entity, With<SettingsScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
