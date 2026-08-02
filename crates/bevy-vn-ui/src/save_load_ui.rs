//! Save/Load screen UI.

use bevy::prelude::*;
use bevy_vn_core::state::VnAppState;
use bevy_vn_core::theme::VnTheme;

#[derive(Component)]
struct SaveLoadScreen;

pub struct SaveLoadUiPlugin;

impl Plugin for SaveLoadUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnAppState::Menu), spawn_save_load.run_if(in_save_load))
            .add_systems(OnExit(VnAppState::Menu), despawn_save_load);
    }
}

fn in_save_load(state: Res<State<VnAppState>>) -> bool {
    *state.get() == VnAppState::Menu
}

fn spawn_save_load(mut commands: Commands, theme: Option<Res<VnTheme>>) {
    let slt = theme.as_ref().map(|t| t.save_load.clone()).unwrap_or_default();

    commands.spawn((
        SaveLoadScreen,
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 0.95)),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("Save / Load"),
            TextFont { font_size: FontSize::Px(slt.font_size.max(1.0)), ..default() },
            TextColor(Color::WHITE),
        ));
        // Save slot placeholders
        for i in 0..10 {
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(slt.thumbnail_height.max(1.0)),
                    padding: UiRect::all(Val::Px(12.0)),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 1.0)),
            ))
            .with_child((
                Text::new(format!("Slot {} — Empty", i + 1)),
                TextFont { font_size: FontSize::Px(slt.font_size * 0.8), ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        }
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
            TextFont { font_size: FontSize::Px(slt.font_size * 0.8), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn despawn_save_load(mut commands: Commands, q: Query<Entity, With<SaveLoadScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
