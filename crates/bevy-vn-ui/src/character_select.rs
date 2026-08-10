//! Character select screen — 2x3 grid of the six anthology stories,
//! faithful to the original Narcissu 10th Anniversary `ui_character`
//! layout (960x540 logical pixels).

use bevy::prelude::*;
use bevy_vn_core::messages::StorySelectEvent;
use bevy_vn_core::state::{VnAppState, VnMenuState, VnTransition};

use crate::transition::request_transition;
use crate::responsive::ResponsiveCanvas;

const BG_PATH: &str = "pa/charters/bg.png";
const BTN_PATH: &str = "pa/charters/btn-分支.png";
const EXIT_PATH: &str = "pa/chapter/btn-分支2.png";

#[derive(Component)]
struct CharacterScreen;

#[derive(Component, Clone)]
struct StoryEntry { script: String, label: String }

#[derive(Component)]
struct ExitButton;

struct StorySpec {
    name: &'static str,
    x: f32,
    y: f32,
    script: &'static str,
    label: &'static str,
    clip_y: f32,
}

const STORIES: [StorySpec; 6] = [
    StorySpec { name: "水仙1", x: 380.0, y: 282.0, script: "game-logic", label: "story01", clip_y: 0.0 },
    StorySpec { name: "水仙2", x: 380.0, y: 332.0, script: "game-logic", label: "story02", clip_y: 48.0 },
    StorySpec { name: "水仙 zero", x: 380.0, y: 382.0, script: "game-logic", label: "story03", clip_y: 96.0 },
    StorySpec { name: "水仙 堇", x: 580.0, y: 282.0, script: "game-logic", label: "story04", clip_y: 144.0 },
    StorySpec { name: "姬子终章", x: 580.0, y: 332.0, script: "game-logic", label: "story05", clip_y: 192.0 },
    StorySpec { name: "小小伊丽丝", x: 580.0, y: 382.0, script: "game-logic", label: "story06", clip_y: 240.0 },
];

pub struct CharacterSelectPlugin;
impl Plugin for CharacterSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnMenuState::RouteSelect), spawn_character)
            .add_systems(Update, handle_character_click.run_if(in_state(VnAppState::Menu)))
            .add_systems(OnExit(VnMenuState::RouteSelect), despawn_character);
    }
}

fn spawn_character(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bg = asset_server.load::<Image>(BG_PATH);
    let btn = asset_server.load::<Image>(BTN_PATH);
    let exit = asset_server.load::<Image>(EXIT_PATH);

    commands.spawn((
        CharacterScreen,
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|parent| {
        parent.spawn((
            ResponsiveCanvas,
            Node {
                width: Val::Px(960.0),
                height: Val::Px(540.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|canvas| {
            canvas.spawn((
                ImageNode { image: bg, image_mode: NodeImageMode::Stretch, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(960.0),
                    height: Val::Px(500.0),
                    ..default()
                },
                ZIndex(0),
            ));

            for s in STORIES {
                canvas.spawn((
                    StoryEntry { script: s.script.into(), label: s.label.into() },
                    Button,
                    ImageNode {
                        image: btn.clone(),
                        rect: Some(Rect::new(0.0, s.clip_y, 152.0, s.clip_y + 48.0)),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(s.x),
                        top: Val::Px(s.y),
                        width: Val::Px(152.0),
                        height: Val::Px(48.0),
                        ..default()
                    },
                    ZIndex(1),
                ))
                .with_child((
                    Text::new(s.name),
                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                    TextColor(Color::WHITE),
                ));
            }

            canvas.spawn((
                ExitButton,
                Button,
                ImageNode {
                    image: exit,
                    rect: Some(Rect::new(0.0, 1008.0, 197.0, 1056.0)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(780.0),
                    top: Val::Px(55.0),
                    width: Val::Px(197.0),
                    height: Val::Px(48.0),
                    ..default()
                },
                ZIndex(1),
            ));
        });
    });
}

fn handle_character_click(
    state: Option<Res<State<VnMenuState>>>,
    q: Query<(&StoryEntry, &Interaction), Without<ExitButton>>,
    q_exit: Query<&Interaction, With<ExitButton>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: MessageWriter<StorySelectEvent>,
    mut transition: ResMut<VnTransition>,
) {
    let Some(state) = state else { return };
    if *state.get() != VnMenuState::RouteSelect { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (entry, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        writer.write(StorySelectEvent { script: entry.script.clone(), label: entry.label.clone() });
        request_transition(&mut transition, Some(VnAppState::Gameplay), None);
        return;
    }
    for inter in q_exit.iter() {
        if *inter == Interaction::Pressed {
            request_transition(&mut transition, Some(VnAppState::Title), Some(VnMenuState::Main));
            return;
        }
    }
}

fn despawn_character(mut commands: Commands, q: Query<Entity, With<CharacterScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
