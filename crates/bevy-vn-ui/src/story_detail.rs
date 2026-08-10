//! Story summary screens — one per anthology story. Shows the story's
//! `bg-chapterN.png` overview image with chapter jump buttons placed per
//! the original `ui_story01..06` layouts, plus a back button.

use bevy::prelude::*;
use bevy_vn_core::messages::StorySelectEvent;
use bevy_vn_core::state::{VnAppState, VnMenuState, VnTransition};

use crate::transition::request_transition;
use crate::responsive::ResponsiveCanvas;

const BTN_CIRCLE: &str = "pa/conf/button-circle.png";
const BTN_RECT: &str = "pa/chapter/ch-btn.png";
const BTN_BACK: &str = "pa/chapter/btn-分支2.png";

/// Which story's summary is currently shown (0-5, index into STORIES).
#[derive(Resource, Default)]
pub struct CurrentStory(pub usize);

#[derive(Component)]
struct StoryDetailScreen;

#[derive(Component, Clone)]
struct ChapterButton { script: String }

#[derive(Component)]
struct BackButton;

struct ChapterSpec {
    script: &'static str,
    x: f32,
    y: f32,
    rect: bool,
}

struct StorySpec {
    bg: &'static str,
    chapters: &'static [ChapterSpec],
}

const CIR: bool = false;
const RECT: bool = true;

const STORY1: &[ChapterSpec] = &[
    ChapterSpec { script: "nar1_00", x: 610.0, y: 110.0, rect: CIR },
    ChapterSpec { script: "nar1_01", x: 610.0, y: 153.0, rect: CIR },
    ChapterSpec { script: "nar1_02", x: 610.0, y: 201.0, rect: CIR },
    ChapterSpec { script: "nar1_04", x: 610.0, y: 251.0, rect: CIR },
    ChapterSpec { script: "nar1_05", x: 610.0, y: 294.0, rect: CIR },
    ChapterSpec { script: "nar1_06", x: 610.0, y: 340.0, rect: CIR },
    ChapterSpec { script: "nar1_07", x: 610.0, y: 390.0, rect: CIR },
    ChapterSpec { script: "nar1_08", x: 610.0, y: 433.0, rect: CIR },
];

const STORY2: &[ChapterSpec] = &[
    ChapterSpec { script: "nar2_01", x: 590.0, y: 113.0, rect: RECT },
    ChapterSpec { script: "nar2_02", x: 590.0, y: 160.0, rect: RECT },
    ChapterSpec { script: "nar2_04", x: 590.0, y: 207.0, rect: RECT },
    ChapterSpec { script: "nar2_05", x: 590.0, y: 254.0, rect: RECT },
    ChapterSpec { script: "nar2_06", x: 590.0, y: 298.0, rect: RECT },
    ChapterSpec { script: "nar2_07", x: 590.0, y: 342.0, rect: RECT },
    ChapterSpec { script: "nar2_08", x: 590.0, y: 390.0, rect: RECT },
    ChapterSpec { script: "nar2_09", x: 590.0, y: 434.0, rect: RECT },
    ChapterSpec { script: "nar2_10", x: 710.0, y: 113.0, rect: RECT },
    ChapterSpec { script: "nar2_11", x: 710.0, y: 160.0, rect: RECT },
    ChapterSpec { script: "nar2_12", x: 710.0, y: 207.0, rect: RECT },
    ChapterSpec { script: "nar2_13", x: 710.0, y: 254.0, rect: RECT },
    ChapterSpec { script: "nar2_14", x: 710.0, y: 298.0, rect: RECT },
    ChapterSpec { script: "nar2_15", x: 710.0, y: 342.0, rect: RECT },
    ChapterSpec { script: "nar2_16", x: 710.0, y: 390.0, rect: RECT },
    ChapterSpec { script: "nar2_17", x: 710.0, y: 434.0, rect: RECT },
    ChapterSpec { script: "nar2_18", x: 820.0, y: 113.0, rect: RECT },
    ChapterSpec { script: "nar2_19", x: 820.0, y: 160.0, rect: RECT },
    ChapterSpec { script: "nar2_20", x: 820.0, y: 207.0, rect: RECT },
];

const STORY3: &[ChapterSpec] = &[
    ChapterSpec { script: "nar4_00", x: 565.0, y: 110.0, rect: CIR },
    ChapterSpec { script: "nar4_02", x: 565.0, y: 153.0, rect: CIR },
    ChapterSpec { script: "nar4_03", x: 565.0, y: 196.0, rect: CIR },
    ChapterSpec { script: "nar4_04", x: 565.0, y: 240.0, rect: CIR },
    ChapterSpec { script: "nar4_05", x: 565.0, y: 287.0, rect: CIR },
    ChapterSpec { script: "nar4_06", x: 565.0, y: 330.0, rect: CIR },
    ChapterSpec { script: "nar4_07", x: 565.0, y: 370.0, rect: CIR },
    ChapterSpec { script: "nar4_08", x: 565.0, y: 410.0, rect: CIR },
    ChapterSpec { script: "nar4_09", x: 752.0, y: 110.0, rect: CIR },
    ChapterSpec { script: "nar4_10", x: 752.0, y: 153.0, rect: CIR },
    ChapterSpec { script: "nar4_ep", x: 752.0, y: 370.0, rect: CIR },
    ChapterSpec { script: "4atogaki", x: 752.0, y: 410.0, rect: CIR },
];

const STORY4: &[ChapterSpec] = &[
    ChapterSpec { script: "omake", x: 645.0, y: 110.0, rect: CIR },
    ChapterSpec { script: "@あかり1", x: 590.0, y: 150.0, rect: CIR },
    ChapterSpec { script: "@あかり2", x: 745.0, y: 150.0, rect: CIR },
    ChapterSpec { script: "@あかり3", x: 590.0, y: 182.0, rect: CIR },
    ChapterSpec { script: "@あかり4", x: 745.0, y: 182.0, rect: CIR },
    ChapterSpec { script: "@あかり5", x: 590.0, y: 212.0, rect: CIR },
    ChapterSpec { script: "@あかり6", x: 745.0, y: 212.0, rect: CIR },
    ChapterSpec { script: "narsumi00", x: 645.0, y: 260.0, rect: CIR },
    ChapterSpec { script: "narsumi01", x: 590.0, y: 302.0, rect: CIR },
    ChapterSpec { script: "narsumi02", x: 745.0, y: 302.0, rect: CIR },
    ChapterSpec { script: "narsumi03", x: 590.0, y: 337.0, rect: CIR },
    ChapterSpec { script: "narsumi04", x: 745.0, y: 337.0, rect: CIR },
    ChapterSpec { script: "narsumi05", x: 590.0, y: 370.0, rect: CIR },
    ChapterSpec { script: "narsumi06", x: 745.0, y: 370.0, rect: CIR },
    ChapterSpec { script: "narsumi07", x: 590.0, y: 402.0, rect: CIR },
    ChapterSpec { script: "narsumi08", x: 745.0, y: 402.0, rect: CIR },
];

const STORY5: &[ChapterSpec] = &[
    ChapterSpec { script: "himeko_ep00", x: 605.0, y: 232.0, rect: CIR },
    ChapterSpec { script: "himeko_atogaki", x: 605.0, y: 295.0, rect: CIR },
];

const STORY6: &[ChapterSpec] = &[
    ChapterSpec { script: "nar3_01", x: 595.0, y: 110.0, rect: CIR },
    ChapterSpec { script: "nar3_02", x: 595.0, y: 153.0, rect: CIR },
    ChapterSpec { script: "nar3_03", x: 595.0, y: 205.0, rect: CIR },
    ChapterSpec { script: "nar3_04", x: 595.0, y: 250.0, rect: CIR },
    ChapterSpec { script: "nar3_05", x: 595.0, y: 297.0, rect: CIR },
    ChapterSpec { script: "nar3_06", x: 595.0, y: 350.0, rect: CIR },
    ChapterSpec { script: "nar3_07", x: 595.0, y: 390.0, rect: CIR },
    ChapterSpec { script: "nar3_08", x: 595.0, y: 430.0, rect: CIR },
];

const STORIES: [StorySpec; 6] = [
    StorySpec { bg: "pa/chapter/story1/bg-chapter1.png", chapters: STORY1 },
    StorySpec { bg: "pa/chapter/story2/bg-chapter2.png", chapters: STORY2 },
    StorySpec { bg: "pa/chapter/story3/bg-chapter3.png", chapters: STORY3 },
    StorySpec { bg: "pa/chapter/story4/bg-chapter4.png", chapters: STORY4 },
    StorySpec { bg: "pa/chapter/story5/bg-chapter5.png", chapters: STORY5 },
    StorySpec { bg: "pa/chapter/story6/bg-chapter6.png", chapters: STORY6 },
];

pub struct StoryDetailPlugin;
impl Plugin for StoryDetailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentStory>()
            .add_systems(OnEnter(VnMenuState::StoryDetail), spawn_story_detail)
            .add_systems(Update, handle_chapter_click.run_if(in_state(VnAppState::Menu)))
            .add_systems(OnExit(VnMenuState::StoryDetail), despawn_story_detail);
    }
}

fn spawn_story_detail(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cur: Res<CurrentStory>,
) {
    let idx = cur.0.min(STORIES.len() - 1);
    let story = &STORIES[idx];
    let bg = asset_server.load::<Image>(story.bg);
    let btn_circle = asset_server.load::<Image>(BTN_CIRCLE);
    let btn_rect = asset_server.load::<Image>(BTN_RECT);
    let btn_back = asset_server.load::<Image>(BTN_BACK);

    commands.spawn((
        StoryDetailScreen,
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
                    height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(0),
            ));

            for ch in story.chapters {
                let (img, w, h) = if ch.rect {
                    (btn_rect.clone(), 53.0, 15.0)
                } else {
                    (btn_circle.clone(), 20.0, 20.0)
                };
                canvas.spawn((
                    ChapterButton { script: ch.script.into() },
                    Button,
                    ImageNode {
                        image: img,
                        rect: Some(Rect::new(0.0, 0.0, w, h)),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(ch.x),
                        top: Val::Px(ch.y),
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    ZIndex(1),
                ));
            }

            canvas.spawn((
                BackButton,
                Button,
                ImageNode {
                    image: btn_back,
                    rect: Some(Rect::new(0.0, 1008.0, 197.0, 1056.0)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(480.0),
                    top: Val::Px(500.0),
                    width: Val::Px(197.0),
                    height: Val::Px(48.0),
                    ..default()
                },
                ZIndex(2),
            ));
        });
    });
}

fn handle_chapter_click(
    state: Option<Res<State<VnMenuState>>>,
    q: Query<(&ChapterButton, &Interaction), Without<BackButton>>,
    q_back: Query<&Interaction, With<BackButton>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: MessageWriter<StorySelectEvent>,
    mut transition: ResMut<VnTransition>,
) {
    let Some(state) = state else { return };
    if *state.get() != VnMenuState::StoryDetail { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (ch, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        writer.write(StorySelectEvent { script: ch.script.clone(), label: "top".into() });
        request_transition(&mut transition, Some(VnAppState::Gameplay), None);
        return;
    }
    for inter in q_back.iter() {
        if *inter == Interaction::Pressed {
            request_transition(&mut transition, None, Some(VnMenuState::ChapterSelect));
            return;
        }
    }
}

fn despawn_story_detail(mut commands: Commands, q: Query<Entity, With<StoryDetailScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
