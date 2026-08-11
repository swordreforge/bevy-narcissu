//! Title screen — image-based, faithful to the original Narcissu 10th
//! Anniversary layout (960x540 logical pixels).
//!
//! Background follows the original engine (list_windows.tbl `ui_title01`):
//! `bg = { file="title/bg", x=0, y=-423, w=960, h=540 }`. The source PNG is
//! 960x963; the original displays the bottom 540px band (y offset -423) and
//! animates it from the top band down over 7s (title.lua `title_anime`).

use bevy::prelude::*;
use bevy_vn_core::state::{SaveLoadKind, SaveLoadMode, SaveLoadReturn, VnAppState, VnMenuState, VnTransition};

use crate::transition::request_transition;
use crate::responsive::ResponsiveCanvas;

const BG_PATH: &str = "pa/title/bg.png";
const LOGO_PATH: &str = "pa/title/logo.png";
const BTN_PATH: &str = "pa/title/btn.png";
const TYPEMOON_PATH: &str = "pa/title/TYPEMOON.png";

/// Original title.bg is 960x963 — only the bottom 540px band is shown in the
/// 960x540 viewport (y=-423 in the original engine's ui_title01 table).
const BG_SRC_W: f32 = 960.0;
const BG_SRC_H: f32 = 963.0;
const BG_BAND_H: f32 = 540.0;
const BG_OFFSET_MAX: f32 = BG_SRC_H - BG_BAND_H; // 423 — bottom band offset

/// Background slide-in animation, matching title.lua `title_anime`:
/// `systween{ id=bg, y="0,-423", time=7000, ease="none" }`.
const BG_SLIDE_DURATION: f32 = 7.0;

#[derive(Component)]
struct TitleScreen;

/// Drives the background band slide (y offset 0 → BG_OFFSET_MAX over 7s).
#[derive(Component)]
struct BgSlideAnim {
    elapsed: f32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TitleAction {
    Start,
    Chapter,
    Load,
    Settings,
    Extra,
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
}

#[derive(Component)]
struct TitleButton(TitleAction);

struct ButtonSpec {
    action: TitleAction,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    clip_y: f32,
}

pub struct TitlePlugin;
impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnAppState::Title), spawn_title)
            .add_systems(Update, (animate_bg_slide, handle_title_click))
            .add_systems(OnExit(VnAppState::Title), despawn_title);
    }
}

fn spawn_title(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bg = asset_server.load::<Image>(BG_PATH);
    let logo = asset_server.load::<Image>(LOGO_PATH);
    let btn = asset_server.load::<Image>(BTN_PATH);
    let typemoon = asset_server.load::<Image>(TYPEMOON_PATH);

    let buttons = [
        ButtonSpec { action: TitleAction::Start, x: 220.0, y: 226.0, w: 129.0, h: 32.0, clip_y: 0.0 },
        ButtonSpec { action: TitleAction::Chapter, x: 220.0, y: 266.0, w: 129.0, h: 32.0, clip_y: 32.0 },
        ButtonSpec { action: TitleAction::Load, x: 220.0, y: 306.0, w: 129.0, h: 32.0, clip_y: 63.0 },
        ButtonSpec { action: TitleAction::Settings, x: 220.0, y: 346.0, w: 129.0, h: 32.0, clip_y: 96.0 },
        ButtonSpec { action: TitleAction::Extra, x: 220.0, y: 386.0, w: 129.0, h: 32.0, clip_y: 128.0 },
        // Quit only exists on native builds — a browser tab cannot quit,
        // and an in-game exit button is meaningless on wasm.
        #[cfg(not(target_arch = "wasm32"))]
        ButtonSpec { action: TitleAction::Quit, x: 220.0, y: 426.0, w: 129.0, h: 32.0, clip_y: 160.0 },
    ];

    commands.spawn((
        TitleScreen,
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
        // Fixed 960x540 canvas, centered in whatever window size. Scaling is
        // handled by ResponsivePlugin (see responsive.rs): the canvas scales
        // uniformly to fit the window while the layout stays 960x540.
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
                ImageNode {
                    image: bg,
                    image_mode: NodeImageMode::Stretch,
                    // Crop the source (960x963) to a 960x540 band; the band's y
                    // offset starts at 0 (top) and slides down to 423 (bottom).
                    rect: Some(Rect::new(0.0, 0.0, BG_SRC_W, BG_BAND_H)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BgSlideAnim { elapsed: 0.0 },
                ZIndex(0),
            ));

            canvas.spawn((
                ImageNode { image: logo, ..default() },
                // Engine renders no-clip obj at native PNG size (511x152), not
                // the tbl layout-box w=336 — see logo03/TYPEMOON (tbl 336x152,
                // PNG 105x30). 336 would squash the title and misalign buttons.
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(50.0),
                    top: Val::Px(49.0),
                    width: Val::Px(511.0),
                    height: Val::Px(152.0),
                    ..default()
                },
                ZIndex(1),
            ));

            canvas.spawn((
                ImageNode { image: typemoon, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(850.0),
                    top: Val::Px(510.0),
                    width: Val::Px(105.0),
                    height: Val::Px(30.0),
                    ..default()
                },
                ZIndex(1),
            ));

            for b in buttons {
                canvas.spawn((
                    TitleButton(b.action),
                    Button,
                    ImageNode {
                        image: btn.clone(),
                        rect: Some(Rect::new(0.0, b.clip_y, b.w, b.clip_y + b.h)),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(b.x),
                        top: Val::Px(b.y),
                        width: Val::Px(b.w),
                        height: Val::Px(b.h),
                        ..default()
                    },
                    ZIndex(2),
                ));
            }
        });
    });
}

/// Slides the background band from the top (y=0) down to the bottom band
/// (y=423) over BG_SLIDE_DURATION seconds — mirrors title.lua `title_anime`.
/// Linear ease matches the original (`ease="none"`). Any mouse click skips
/// the remaining animation and jumps straight to the final bottom band,
/// matching the original's interruptible `title_skipset` behavior. The slide
/// stops on leaving the title state (the entity is despawned by `despawn_title`).
fn animate_bg_slide(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut q: Query<(&mut BgSlideAnim, &mut ImageNode)>,
) {
    for (mut anim, mut image) in &mut q {
        if mouse.just_pressed(MouseButton::Left) {
            anim.elapsed = BG_SLIDE_DURATION; // skip to the end
        } else {
            anim.elapsed += time.delta_secs();
        }
        let t = (anim.elapsed / BG_SLIDE_DURATION).clamp(0.0, 1.0);
        let y = BG_OFFSET_MAX * t;
        image.rect = Some(Rect::new(0.0, y, BG_SRC_W, y + BG_BAND_H));
    }
}

fn handle_title_click(
    state: Res<State<VnAppState>>,
    q: Query<(&TitleButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut transition: ResMut<VnTransition>,
    mut mode: ResMut<SaveLoadMode>,
    #[cfg(not(target_arch = "wasm32"))]
    mut exit: MessageWriter<AppExit>,
) {
    if *state.get() != VnAppState::Title { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (btn, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        match btn.0 {
            TitleAction::Start => {
                request_transition(&mut transition, Some(VnAppState::Menu), Some(VnMenuState::RouteSelect));
            }
            TitleAction::Chapter => {
                request_transition(&mut transition, Some(VnAppState::Menu), Some(VnMenuState::ChapterSelect));
            }
            TitleAction::Load => {
                *mode = SaveLoadMode {
                    active: true,
                    kind: SaveLoadKind::Load,
                    return_to: SaveLoadReturn::Title,
                };
                request_transition(&mut transition, Some(VnAppState::Menu), Some(VnMenuState::SaveLoad));
            }
            TitleAction::Settings => {
                request_transition(&mut transition, Some(VnAppState::Menu), Some(VnMenuState::Settings));
            }
            TitleAction::Extra => {
                request_transition(&mut transition, Some(VnAppState::Menu), Some(VnMenuState::Gallery));
            }
            #[cfg(not(target_arch = "wasm32"))]
            TitleAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

fn despawn_title(mut commands: Commands, q: Query<Entity, With<TitleScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
