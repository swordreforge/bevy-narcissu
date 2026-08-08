//! Backlog / text history UI.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_vn_core::messages::BacklogPushEvent;
use bevy_vn_core::theme::VnTheme;

#[derive(Resource, Default)]
pub struct BacklogState {
    pub entries: Vec<BacklogEntry>,
    pub visible: bool,
}

#[derive(Clone)]
pub struct BacklogEntry {
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Component)]
struct BacklogRoot;

#[derive(Component)]
struct BacklogScrollArea;

#[derive(Component)]
struct BacklogCloseButton;

pub struct BacklogPlugin;

impl Plugin for BacklogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BacklogState>()
            .add_systems(Update, (handle_backlog_push, rebuild_backlog_ui, update_backlog_visibility, scroll_backlog, handle_backlog_close));
    }
}

fn update_backlog_visibility(
    state: Res<BacklogState>,
    mut q_root: Query<&mut Node, With<BacklogRoot>>,
) {
    for mut node in q_root.iter_mut() {
        let want = if state.visible { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

const BACKLOG_SCROLL_SPEED: f32 = 60.0;
fn scroll_backlog(
    state: Res<BacklogState>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut q: Query<&mut ScrollPosition, With<BacklogScrollArea>>,
) {
    if !state.visible {
        return;
    }
    for ev in scroll_events.read() {
        for mut pos in &mut q {
            pos.0.y += ev.y * BACKLOG_SCROLL_SPEED;
        }
    }
}

fn handle_backlog_close(
    mut state: ResMut<BacklogState>,
    mut q_scroll: Query<&Interaction, (With<BacklogScrollArea>, Without<BacklogCloseButton>)>,
    mut q_close: Query<&Interaction, (With<BacklogCloseButton>, Changed<Interaction>)>,
) {
    if !state.visible {
        return;
    }
    for inter in &mut q_close {
        if *inter == Interaction::Pressed {
            state.visible = false;
            return;
        }
    }
    for inter in &mut q_scroll {
        if *inter == Interaction::Pressed {
            state.visible = false;
            return;
        }
    }
}

fn handle_backlog_push(
    mut reader: MessageReader<BacklogPushEvent>,
    mut state: ResMut<BacklogState>,
) {
    for event in reader.read() {
        state.entries.push(BacklogEntry {
            speaker: event.entry.speaker.clone(),
            text: event.entry.text.clone(),
        });
    }
}

fn rebuild_backlog_ui(
    state: Res<BacklogState>,
    mut commands: Commands,
    theme: Option<Res<VnTheme>>,
    q_old: Query<Entity, With<BacklogRoot>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Simple toggle with B key — production would use menu state
    if keys.just_pressed(KeyCode::KeyB) {
        // toggle not implemented here — placeholder
    }
    // Rebuild only when entries change — simplified: always check
    if !state.is_changed() { return; }

    for e in q_old.iter() { commands.entity(e).despawn(); }
    if state.entries.is_empty() { return; }

    let bt = theme.as_ref()
        .map(|t| t.backlog.clone())
        .unwrap_or_default();

    commands.spawn((
        BacklogRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(5.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            height: Val::Percent(80.0),
            flex_direction: FlexDirection::Column,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ZIndex(30),
    ))
    .with_children(|parent| {
        parent.spawn((
            BacklogScrollArea,
            Button,
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow { x: OverflowAxis::Visible, y: OverflowAxis::Scroll },
                scrollbar_width: 8.0,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
        ))
        .with_children(|scroll| {
            for entry in state.entries.iter().rev().take(bt.max_entries.max(50)) {
                scroll.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    ..default()
                }).with_children(|row| {
                    if let Some(ref s) = entry.speaker {
                        row.spawn((
                            Text::new(format!("{}：", s)),
                            TextFont { font_size: FontSize::Px(bt.font_size), ..default() },
                            TextColor(Color::srgb(0.7, 0.7, 1.0)),
                        ));
                    }
                    row.spawn((
                        Text::new(&entry.text),
                        TextFont { font_size: FontSize::Px(bt.font_size), ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
        parent.spawn((
            BacklogCloseButton,
            Button,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                right: Val::Px(8.0),
                width: Val::Px(28.0),
                height: Val::Px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.9)),
            ZIndex(1),
        ))
        .with_child((
            Text::new("✕"),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}
