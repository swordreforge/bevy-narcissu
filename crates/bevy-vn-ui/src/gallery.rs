//! CG Gallery screen.

use bevy::prelude::*;
use bevy_vn_core::state::VnAppState;
use bevy_vn_core::theme::VnTheme;

#[derive(Component)]
struct GalleryScreen;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnAppState::Menu), spawn_gallery.run_if(in_gallery))
            .add_systems(OnExit(VnAppState::Menu), despawn_gallery);
    }
}

fn in_gallery(state: Res<State<VnAppState>>) -> bool {
    *state.get() == VnAppState::Menu
}

fn spawn_gallery(mut commands: Commands, theme: Option<Res<VnTheme>>) {
    let gt = theme.as_ref().map(|t| t.gallery.clone()).unwrap_or_default();

    commands.spawn((
        GalleryScreen,
        Node {
            width: percent(100), height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(40.0)),
            overflow: Overflow { x: OverflowAxis::Visible, y: OverflowAxis::Scroll },
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 0.95)),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("CG Gallery"),
            TextFont { font_size: FontSize::Px(gt.thumbnail_height.max(1.0)), ..default() },
            TextColor(Color::WHITE),
        ));
        // Thumbnail grid
        parent.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|grid| {
            for i in 1..=12 {
                grid.spawn((
                    Button,
                    Node {
                        width: Val::Px(gt.thumbnail_width.max(1.0)),
                        height: Val::Px(gt.thumbnail_height.max(1.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 1.0)),
                ))
                .with_child((
                    Text::new(format!("CG {:02}", i)),
                    TextFont { font_size: FontSize::Px(gt.thumbnail_height * 0.3), ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ));
            }
        });
        // Back button
        parent.spawn((
            Button,
            Node {
                width: Val::Px(200.0), height: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.2, 1.0)),
        ))
        .with_child((
            Text::new("Back"),
            TextFont { font_size: FontSize::Px(gt.thumbnail_height * 0.5), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn despawn_gallery(mut commands: Commands, q: Query<Entity, With<GalleryScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
