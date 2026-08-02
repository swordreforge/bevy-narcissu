//! Title screen.

use bevy::prelude::*;
use bevy_vn_core::state::VnAppState;

#[derive(Component)]
struct TitleScreen;

#[derive(Component, Clone, Copy)]
enum TitleAction { NewGame, LoadGame, Settings, Gallery, Quit }

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnAppState::Title), spawn_title)
            .add_systems(Update, handle_title_click)
            .add_systems(OnExit(VnAppState::Title), despawn_title);
    }
}

fn spawn_title(mut commands: Commands) {
    commands.spawn((
        TitleScreen,
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(24.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 1.0)),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("Visual Novel Engine"),
            TextFont { font_size: FontSize::Px(48.0), ..default() },
            TextColor(Color::WHITE),
        ));

        for (label, action) in [
            ("New Game", TitleAction::NewGame),
            ("Load Game", TitleAction::LoadGame),
            ("Settings", TitleAction::Settings),
            ("Gallery", TitleAction::Gallery),
            ("Quit", TitleAction::Quit),
        ] {
            parent.spawn((
                Button,
                action,
                Node {
                    width: Val::Px(300.0), height: Val::Px(56.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.12, 0.2, 1.0)),
            ))
            .with_child((
                Text::new(label),
                TextFont { font_size: FontSize::Px(24.0), ..default() },
                TextColor(Color::WHITE),
            ));
        }
    });
}

fn handle_title_click(
    _state: Option<Res<State<VnAppState>>>,
    q: Query<(&TitleAction, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<VnAppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    // Only run when in Title state
    if _state.map_or(true, |s| *s.get() != VnAppState::Title) { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (action, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        match action {
            TitleAction::NewGame => next.set(VnAppState::Gameplay),
            TitleAction::LoadGame | TitleAction::Settings | TitleAction::Gallery => {
                next.set(VnAppState::Menu);
            }
            TitleAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

fn despawn_title(mut commands: Commands, q: Query<Entity, With<TitleScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
