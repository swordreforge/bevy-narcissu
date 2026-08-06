//! Chapter select screen — the anthology story grid reached from the
//! title's Chapter button, faithful to the original Narcissu 10th
//! Anniversary `ui_chapter` layout (960x540 logical pixels).
//! Picking a story opens its summary screen (story_detail).

use bevy::prelude::*;
use bevy_vn_core::state::{VnAppState, VnMenuState};
use crate::story_detail::CurrentStory;

const BG_PATH: &str = "pa/chapter/main-bg.png";
const BTN_PATH: &str = "pa/chapter/btn-分支2.png";

#[derive(Component)]
struct ChapterScreen;

#[derive(Component, Clone)]
struct StoryEntry { story_idx: usize }

#[derive(Component)]
struct ExitButton;

struct StorySpec {
    name: &'static str,
    x: f32,
    y: f32,
    story_idx: usize,
    clip_y: f32,
    clip_w: f32,
    clip_h: f32,
}

const STORIES: [StorySpec; 6] = [
    StorySpec { name: "水仙1+2合集", x: 177.0, y: 322.0, story_idx: 0, clip_y: 0.0, clip_w: 228.0, clip_h: 50.0 },
    StorySpec { name: "水仙2", x: 418.0, y: 322.0, story_idx: 1, clip_y: 52.0, clip_w: 228.0, clip_h: 52.0 },
    StorySpec { name: "水仙Zero", x: 662.0, y: 322.0, story_idx: 2, clip_y: 100.0, clip_w: 228.0, clip_h: 50.0 },
    StorySpec { name: "水仙堇", x: 182.0, y: 450.0, story_idx: 3, clip_y: 148.0, clip_w: 228.0, clip_h: 50.0 },
    StorySpec { name: "水仙姬子", x: 418.0, y: 450.0, story_idx: 4, clip_y: 200.0, clip_w: 228.0, clip_h: 42.0 },
    StorySpec { name: "小小伊丽丝", x: 662.0, y: 450.0, story_idx: 5, clip_y: 246.0, clip_w: 228.0, clip_h: 50.0 },
];

pub struct ChapterSelectPlugin;
impl Plugin for ChapterSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnMenuState::ChapterSelect), spawn_chapter)
            .add_systems(Update, handle_chapter_click.run_if(in_state(VnAppState::Menu)))
            .add_systems(OnExit(VnMenuState::ChapterSelect), despawn_chapter);
    }
}

fn spawn_chapter(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bg = asset_server.load::<Image>(BG_PATH);
    let btn = asset_server.load::<Image>(BTN_PATH);

    commands.spawn((
        ChapterScreen,
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
                    height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(0),
            ));

            for s in STORIES {
                canvas.spawn((
                    StoryEntry { story_idx: s.story_idx },
                    Button,
                    ImageNode {
                        image: btn.clone(),
                        rect: Some(Rect::new(0.0, s.clip_y, s.clip_w, s.clip_y + s.clip_h)),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(s.x),
                        top: Val::Px(s.y),
                        width: Val::Px(152.0),
                        height: Val::Px(80.0),
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
                    image: btn,
                    rect: Some(Rect::new(0.0, 1008.0, 197.0, 1056.0)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(700.0),
                    top: Val::Px(500.0),
                    width: Val::Px(197.0),
                    height: Val::Px(48.0),
                    ..default()
                },
                ZIndex(1),
            ));
        });
    });
}

fn handle_chapter_click(
    state: Option<Res<State<VnMenuState>>>,
    q: Query<(&StoryEntry, &Interaction), Without<ExitButton>>,
    q_exit: Query<&Interaction, With<ExitButton>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cur: ResMut<CurrentStory>,
    mut next: ResMut<NextState<VnMenuState>>,
    mut next_app: ResMut<NextState<VnAppState>>,
) {
    let Some(state) = state else { return };
    if *state.get() != VnMenuState::ChapterSelect { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (entry, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        cur.0 = entry.story_idx;
        next.set(VnMenuState::StoryDetail);
        return;
    }
    for inter in q_exit.iter() {
        if *inter == Interaction::Pressed {
            next.set(VnMenuState::Main);
            next_app.set(VnAppState::Title);
            return;
        }
    }
}

fn despawn_chapter(mut commands: Commands, q: Query<Entity, With<ChapterScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
