//! Settings screen — sprite-based, faithful to the original Narcissu 10th
//! Anniversary layout. Interactions are click-only (no drag / keyboard).
//! Two pages + one text sub-screen, values persisted to saves/settings.json.

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy_vn_core::engine_config::VnEngineConfig;
use bevy_vn_core::messages::SetVolumeEvent;
use bevy_vn_core::state::{SaveLoadKind, SaveLoadMode, SaveLoadReturn, VnAppState, VnMenuState};
use bevy_vn_core::theme::VnTheme;

use crate::settings_data::{load_settings, save_settings, GameSettings};

const BG_PATH: &str = "pa/conf/bg01.png";
const BG2_PATH: &str = "pa/conf/bg02.png";
const SUBBG_PATH: &str = "pa/conf/sub02.png";
const TAB_PATH: &str = "pa/conf/MID-MEAN.png";
const TRACK_PATH: &str = "pa/conf/btn-logo.png";
const VO_TRACK_PATH: &str = "pa/conf/btn.png";
const PIN_PATH: &str = "pa/conf/spin01.png";
const VO_PIN_PATH: &str = "pa/conf/spin02.png";
const CIRCLE_PATH: &str = "pa/conf/button-circle.png";
const MW_PATH: &str = "pa/conf/mw.png";
const CHECK_PATHS: [&str; 8] = [
    "pa/conf/check01.png", "pa/conf/check02.png", "pa/conf/check03.png",
    "pa/conf/check04.png", "pa/conf/check05.png", "pa/conf/check06.png",
    "pa/conf/check07.png", "pa/conf/check08.png",
];

const Z_BG: i32 = 0;
const Z_DECOR: i32 = 1;
const Z_CTRL: i32 = 2;
const Z_SUB: i32 = 10;
const SLIDER_ZONES: usize = 20;

#[derive(Component)]
struct SettingsScreen;

/// The 960×540 content canvas under `SettingsScreen`. Replaced as a whole
/// when switching pages so children are never left orphaned.
#[derive(Component)]
struct SettingsCanvas;

#[derive(Component)]
struct TabBar;

#[derive(Component)]
struct BackButton;

#[derive(Component)]
struct Page1Root;

#[derive(Component)]
struct Page2Root;

#[derive(Component)]
struct SubScreenRoot;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SliderKind {
    MwAlpha,
    Aspeed,
    Mspeed,
    Master,
    Bgm,
    Bgmvo,
    Voice,
    Se,
    Sysse,
    Movie,
    VoChar(u8),
}

#[derive(Component, Clone, Copy)]
struct SliderZone {
    kind: SliderKind,
    zone: usize,
}

#[derive(Component, Clone, Copy)]
struct SliderPin {
    kind: SliderKind,
    track_left: f32,
    track_w: f32,
    pin_w: f32,
}

#[derive(Component, Clone, Copy)]
struct SliderValue {
    kind: SliderKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToggleKind {
    Effect,
    Messkip,
    Rclick,
    FlMspeed,
    FlMaster,
    FlBgm,
    FlBgmvo,
    FlVoice,
    FlSe,
    FlSysse,
    FlMovie,
    Voiceskip,
    FlChar(u8),
    Shadow,
    Outline,
    Reset(SliderKind),
}

#[derive(Component, Clone, Copy)]
struct ToggleSetting {
    kind: ToggleKind,
    value: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavTarget {
    Save,
    Load,
    Title,
    Back,
    Page1,
    Page2,
    SubOpen,
    SubClose,
}

#[derive(Component)]
struct NavAction(NavTarget);

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(VnMenuState::Settings), setup_settings)
            .add_systems(
                Update,
                handle_settings_clicks.run_if(in_state(VnMenuState::Settings)),
            )
            .add_systems(
                Update,
                handle_value_clicks.run_if(in_state(VnMenuState::Settings)),
            )
            .add_systems(
                Update,
                update_slider_visuals.run_if(in_state(VnMenuState::Settings)),
            )
            .add_systems(
                Update,
                update_toggle_visuals.run_if(in_state(VnMenuState::Settings)),
            )
            .add_systems(OnExit(VnMenuState::Settings), teardown_settings);
    }
}

fn setup_settings(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<VnEngineConfig>,
) {
    let settings = load_settings(&config.save_dir);
    commands.spawn((
        SettingsScreen,
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
        parent
            .spawn((
                SettingsCanvas,
                Node {
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|canvas| {
                spawn_canvas_content(canvas, &asset_server, &settings, 1);
            });
    });
    commands.insert_resource(settings);
}

fn spawn_canvas_content(
    canvas: &mut ChildSpawnerCommands,
    server: &AssetServer,
    s: &GameSettings,
    page: usize,
) {
    spawn_tabs(canvas, server, page);
    spawn_page(canvas, server, s, page);
    spawn_back(canvas, server);
}

fn spawn_tabs(parent: &mut ChildSpawnerCommands, server: &AssetServer, current: usize) {
    let tab = server.load::<Image>(TAB_PATH);
    let tabs = [
        (NavTarget::Save, 370.0, 0.0),
        (NavTarget::Load, 486.0, 135.0),
        (NavTarget::Page1, 598.0, 272.0),
        (NavTarget::Page2, 708.0, 543.0),
        (NavTarget::Title, 825.0, 408.0),
    ];
    for (nav, x, clip_x) in tabs {
        let is_current = matches!(nav, NavTarget::Page1 | NavTarget::Page2)
            && ((nav == NavTarget::Page1) == (current == 1));
        let (rect, z) = if is_current {
            (Rect::new(clip_x, 63.0, clip_x + 135.0, 95.0), Z_DECOR)
        } else {
            (Rect::new(clip_x, 0.0, clip_x + 135.0, 32.0), Z_CTRL)
        };
        let mut ent = parent.spawn((
            TabBar,
            ImageNode {
                image: tab.clone(),
                rect: Some(rect),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(32.0),
                width: Val::Px(135.0),
                height: Val::Px(32.0),
                ..default()
            },
            ZIndex(z),
        ));
        if !is_current {
            ent.insert((Button, NavAction(nav)));
        }
    }
}

fn spawn_back(parent: &mut ChildSpawnerCommands, server: &AssetServer) {
    let tab = server.load::<Image>(TAB_PATH);
    parent.spawn((
        BackButton,
        ImageNode {
            image: tab,
            rect: Some(Rect::new(680.0, 0.0, 815.0, 32.0)),
            ..default()
        },
        Button,
        NavAction(NavTarget::Back),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(805.0),
            top: Val::Px(505.0),
            width: Val::Px(135.0),
            height: Val::Px(32.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_page(parent: &mut ChildSpawnerCommands, server: &AssetServer, s: &GameSettings, page: usize) {
    if page == 1 {
        spawn_page1(parent, server, s);
    } else {
        spawn_page2(parent, server, s);
    }
}

fn spawn_page1(parent: &mut ChildSpawnerCommands, server: &AssetServer, s: &GameSettings) {
    let bg = server.load::<Image>(BG_PATH);
    parent
        .spawn((Page1Root, Node::default()))
        .with_children(|p| {
            p.spawn((
                ImageNode { image: bg, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(Z_BG),
            ));

            p.spawn((
                ImageNode { image: server.load::<Image>(MW_PATH), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(610.0),
                    top: Val::Px(392.0),
                    width: Val::Px(300.0),
                    height: Val::Px(90.0),
                    ..default()
                },
                ZIndex(Z_DECOR),
            ));

            spawn_slider(p, server, SliderKind::MwAlpha, 640.0, 185.0, 220.0, s.mw_alpha);
            spawn_slider(p, server, SliderKind::Aspeed, 640.0, 260.0, 220.0, s.aspeed);
            spawn_slider(p, server, SliderKind::Mspeed, 165.0, 405.0, 250.0, s.mspeed);

            spawn_reset(p, server, SliderKind::Mspeed, 600.0, 342.0);
            spawn_reset(p, server, SliderKind::MwAlpha, 720.0, 342.0);
            spawn_reset(p, server, SliderKind::Aspeed, 835.0, 342.0);

            spawn_group_toggle(p, server, ToggleKind::Effect, 1, 175.0, 186.0, s.effect);
            spawn_group_toggle(p, server, ToggleKind::Effect, 0, 380.0, 185.0, s.effect);
            spawn_group_toggle(p, server, ToggleKind::Messkip, 0, 175.0, 258.0, s.messkip);
            spawn_group_toggle(p, server, ToggleKind::Messkip, 1, 380.0, 258.0, s.messkip);
            spawn_group_toggle(p, server, ToggleKind::Rclick, 0, 175.0, 332.0, s.rclick);
            spawn_group_toggle(p, server, ToggleKind::Rclick, 1, 266.0, 332.0, s.rclick);
            spawn_group_toggle(p, server, ToggleKind::Rclick, 2, 360.0, 332.0, s.rclick);

            spawn_bool_toggle(p, server, ToggleKind::FlMspeed, s.fl_mspeed, 332.0, 458.0);
            spawn_circle_button(p, server, NavTarget::SubOpen, 152.0, 458.0);
        });
}

fn spawn_page2(parent: &mut ChildSpawnerCommands, server: &AssetServer, s: &GameSettings) {
    let bg = server.load::<Image>(BG2_PATH);
    parent
        .spawn((Page2Root, Node::default()))
        .with_children(|p| {
            p.spawn((
                ImageNode { image: bg, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(Z_BG),
            ));

            let vol_sliders = [
                (SliderKind::Master, 239.0, 187.0, s.master),
                (SliderKind::Bgm, 239.0, 235.0, s.bgm),
                (SliderKind::Bgmvo, 239.0, 280.0, s.bgmvo),
                (SliderKind::Voice, 239.0, 330.0, s.voice),
                (SliderKind::Se, 239.0, 370.0, s.se),
                (SliderKind::Sysse, 239.0, 410.0, s.sysse),
                (SliderKind::Movie, 239.0, 460.0, s.movie),
            ];
            let labels = ["マスター", "BGM", "BGM+ボイス", "ボイス", "SE", "システムSE", "ムービー"];
            for ((kind, x, y, val), label) in vol_sliders.into_iter().zip(labels) {
                spawn_label(p, label, x - 142.0, y);
                // tbl ui_config2: all seven share clip/cw=190, area=190; the
                // per-slider `w` is only the click hitbox, not the track.
                spawn_slider(p, server, kind, x, y, 190.0, val);
            }

            let fl_defs = [
                (ToggleKind::FlMaster, 63.0, 0usize, s.fl_master),
                (ToggleKind::FlBgm, 128.0, 1, s.fl_bgm),
                (ToggleKind::FlBgmvo, 180.0, 2, s.fl_bgmvo),
                (ToggleKind::FlVoice, 68.0, 3, s.fl_voice),
                (ToggleKind::FlSe, 108.0, 4, s.fl_se),
                (ToggleKind::FlSysse, 120.0, 5, s.fl_sysse),
                (ToggleKind::FlMovie, 87.0, 6, s.fl_movie),
            ];
            let fl_ys = [192.0, 237.0, 285.0, 330.0, 375.0, 420.0, 465.0];
            for ((kind, w, idx, on), y) in fl_defs.into_iter().zip(fl_ys) {
                spawn_check_label(p, server, kind, w, idx, on, 93.0, y);
            }

            spawn_check_label(p, server, ToggleKind::Voiceskip, 240.0, 7, s.voiceskip, 590.0, 460.0);

            let char_y = [193.0, 225.0, 262.0, 294.0, 326.0, 358.0, 395.0];
            let vo_y = [195.0, 230.0, 262.0, 294.0, 326.0, 358.0, 390.0];
            let vols = [s.c001, s.c002, s.c003, s.c004, s.c005, s.man, s.fem];
            let fons = [s.fl_c001, s.fl_c002, s.fl_c003, s.fl_c004, s.fl_c005, s.fl_man, s.fl_fem];
            for i in 0..7 {
                spawn_char_toggle(p, server, i as u8, fons[i], 660.0, char_y[i]);
                spawn_vo_slider(p, server, i as u8, vols[i], 743.0, vo_y[i]);
            }
        });
}

fn spawn_sub_screen(parent: &mut ChildSpawnerCommands, server: &AssetServer, s: &GameSettings) {
    let bg = server.load::<Image>(SUBBG_PATH);
    let circle = server.load::<Image>(CIRCLE_PATH);
    parent
        .spawn((SubScreenRoot, Node::default()))
        .with_children(|p| {
            p.spawn((
                ImageNode { image: bg, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(Z_SUB),
            ));
            spawn_circle_bool(p, &circle, ToggleKind::Shadow, s.shadow, 510.0, 285.0, Z_SUB + 1);
            spawn_circle_bool(p, &circle, ToggleKind::Outline, s.outline, 510.0, 365.0, Z_SUB + 1);
            p.spawn((
                ImageNode {
                    image: server.load::<Image>(TAB_PATH),
                    rect: Some(Rect::new(680.0, 0.0, 815.0, 32.0)),
                    ..default()
                },
                Button,
                NavAction(NavTarget::SubClose),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(805.0),
                    top: Val::Px(505.0),
                    width: Val::Px(135.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ZIndex(Z_SUB + 1),
            ));
        });
}

// ── Builders ──

fn spawn_label(parent: &mut ChildSpawnerCommands, text: &str, x: f32, y: f32) {
    parent.spawn((
        Text::new(text),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y + 5.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_slider(
    parent: &mut ChildSpawnerCommands,
    server: &AssetServer,
    kind: SliderKind,
    x: f32,
    y: f32,
    clip_w: f32,
    val: f32,
) {
    let track = server.load::<Image>(TRACK_PATH);
    let pin = server.load::<Image>(PIN_PATH);
    // Track visual width = tbl clip/cw (190 on the sound page, w on the
    // basic page). spin01.png is a two-state sheet: light knob (idle) left,
    // dark knob (dragging) right, each pin_w=19px wide; p2=19 is that width.
    let zone_w = clip_w / SLIDER_ZONES as f32;
    let pin_w = 19.0;
    let pin_x = x + (val / 100.0) * (clip_w - pin_w);

    parent.spawn((
        ImageNode {
            image: track.clone(),
            rect: Some(Rect::new(0.0, 183.0, clip_w, 207.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(clip_w),
            height: Val::Px(24.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));

    for i in 0..SLIDER_ZONES {
        parent.spawn((
            Button,
            SliderZone { kind, zone: i },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x + i as f32 * zone_w),
                top: Val::Px(y),
                width: Val::Px(zone_w + 0.5),
                height: Val::Px(24.0),
                ..default()
            },
            ZIndex(Z_CTRL + 1),
        ));
    }

    parent.spawn((
        SliderPin { kind, track_left: x, track_w: clip_w, pin_w },
        ImageNode {
            image: pin,
            rect: Some(Rect::new(0.0, 0.0, pin_w, 24.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(pin_x),
            top: Val::Px(y),
            width: Val::Px(pin_w),
            height: Val::Px(24.0),
            ..default()
        },
        ZIndex(Z_CTRL + 2),
    ));

    parent.spawn((
        SliderValue { kind },
        Text::new(format!("{:>3}", val as i32)),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x + clip_w + 8.0),
            top: Val::Px(y + 5.0),
            ..default()
        },
        ZIndex(Z_CTRL + 2),
    ));
}

fn spawn_vo_slider(parent: &mut ChildSpawnerCommands, server: &AssetServer, idx: u8, val: f32, x: f32, y: f32) {
    let kind = SliderKind::VoChar(idx);
    let track = server.load::<Image>(VO_TRACK_PATH);
    let pin = server.load::<Image>(VO_PIN_PATH);
    let w = 84.0;
    let zone_w = w / SLIDER_ZONES as f32;
    // spin02.png: same two-state sheet as spin01, 15px per state (p2=15).
    let pin_w = 15.0;
    let pin_x = x + (val / 100.0) * (w - pin_w);

    parent.spawn((
        ImageNode {
            image: track.clone(),
            rect: Some(Rect::new(560.0, 135.0, 644.0, 150.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(w),
            height: Val::Px(15.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));

    for i in 0..SLIDER_ZONES {
        parent.spawn((
            Button,
            SliderZone { kind, zone: i },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x + i as f32 * zone_w),
                top: Val::Px(y),
                width: Val::Px(zone_w + 0.5),
                height: Val::Px(15.0),
                ..default()
            },
            ZIndex(Z_CTRL + 1),
        ));
    }

    parent.spawn((
        SliderPin { kind, track_left: x, track_w: w, pin_w },
        ImageNode {
            image: pin,
            rect: Some(Rect::new(0.0, 0.0, pin_w, 15.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(pin_x),
            top: Val::Px(y),
            width: Val::Px(pin_w),
            height: Val::Px(15.0),
            ..default()
        },
        ZIndex(Z_CTRL + 2),
    ));
}

fn spawn_reset(parent: &mut ChildSpawnerCommands, server: &AssetServer, kind: SliderKind, x: f32, y: f32) {
    let circle = server.load::<Image>(CIRCLE_PATH);
    parent.spawn((
        Button,
        ToggleSetting { kind: ToggleKind::Reset(kind), value: 100 },
        ImageNode {
            image: circle.clone(),
            rect: Some(Rect::new(0.0, 0.0, 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_group_toggle(
    parent: &mut ChildSpawnerCommands,
    server: &AssetServer,
    kind: ToggleKind,
    value: i32,
    x: f32,
    y: f32,
    current: i32,
) {
    let circle = server.load::<Image>(CIRCLE_PATH);
    // button-circle.png: horizontal 20px frames, selected = yellow (x=20..40).
    let cx = if current == value { 20.0 } else { 0.0 };
    parent.spawn((
        Button,
        ToggleSetting { kind, value },
        ImageNode {
            image: circle.clone(),
            rect: Some(Rect::new(cx, 0.0, cx + 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_bool_toggle(
    parent: &mut ChildSpawnerCommands,
    server: &AssetServer,
    kind: ToggleKind,
    on: bool,
    x: f32,
    y: f32,
) {
    let circle = server.load::<Image>(CIRCLE_PATH);
    let cx = if on { 20.0 } else { 0.0 };
    parent.spawn((
        Button,
        ToggleSetting { kind, value: if on { 1 } else { 0 } },
        ImageNode {
            image: circle.clone(),
            rect: Some(Rect::new(cx, 0.0, cx + 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_circle_button(parent: &mut ChildSpawnerCommands, server: &AssetServer, nav: NavTarget, x: f32, y: f32) {
    let circle = server.load::<Image>(CIRCLE_PATH);
    parent.spawn((
        Button,
        NavAction(nav),
        ImageNode {
            image: circle,
            rect: Some(Rect::new(0.0, 0.0, 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_circle_bool(
    parent: &mut ChildSpawnerCommands,
    circle: &Handle<Image>,
    kind: ToggleKind,
    on: bool,
    x: f32,
    y: f32,
    z: i32,
) {
    let cx = if on { 20.0 } else { 0.0 };
    parent.spawn((
        Button,
        ToggleSetting { kind, value: if on { 1 } else { 0 } },
        ImageNode {
            image: circle.clone(),
            rect: Some(Rect::new(cx, 0.0, cx + 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(z),
    ));
}

fn spawn_check_label(
    parent: &mut ChildSpawnerCommands,
    server: &AssetServer,
    kind: ToggleKind,
    w: f32,
    idx: usize,
    on: bool,
    x: f32,
    y: f32,
) {
    let check = server.load::<Image>(CHECK_PATHS[idx]);
    // check0N.png: vertical 15px states, checked = black box (y=31), else gray.
    let cy = if on { 31.0 } else { 0.0 };
    parent.spawn((
        Button,
        ToggleSetting { kind, value: if on { 1 } else { 0 } },
        ImageNode {
            image: check.clone(),
            rect: Some(Rect::new(0.0, cy, w, cy + 15.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(w),
            height: Val::Px(15.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_char_toggle(parent: &mut ChildSpawnerCommands, server: &AssetServer, idx: u8, on: bool, x: f32, y: f32) {
    let circle = server.load::<Image>(CIRCLE_PATH);
    let cx = if on { 20.0 } else { 0.0 };
    parent.spawn((
        Button,
        ToggleSetting { kind: ToggleKind::FlChar(idx), value: if on { 1 } else { 0 } },
        ImageNode {
            image: circle.clone(),
            rect: Some(Rect::new(cx, 0.0, cx + 20.0, 20.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

// ── Click handling ──

fn handle_settings_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    asset_server: Res<AssetServer>,
    q_nav: Query<(&NavAction, &Interaction)>,
    q_root: Query<Entity, With<SettingsScreen>>,
    q_canvas: Query<Entity, With<SettingsCanvas>>,
    q_sub: Query<Entity, With<SubScreenRoot>>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut next_menu: ResMut<NextState<VnMenuState>>,
    mut next_app: ResMut<NextState<VnAppState>>,
    mut mode: ResMut<SaveLoadMode>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (nav, inter) in q_nav.iter() {
        if *inter != Interaction::Pressed { continue; }
        match nav.0 {
            NavTarget::Save | NavTarget::Load => {
                let kind = if nav.0 == NavTarget::Save { SaveLoadKind::Save } else { SaveLoadKind::Load };
                *mode = SaveLoadMode {
                    active: true,
                    kind,
                    return_to: SaveLoadReturn::Settings,
                };
                next_menu.set(VnMenuState::SaveLoad);
                return;
            }
            NavTarget::Title | NavTarget::Back => {
                next_menu.set(VnMenuState::Main);
                next_app.set(VnAppState::Title);
                return;
            }
            NavTarget::Page1 | NavTarget::Page2 => {
                let page = if nav.0 == NavTarget::Page1 { 1 } else { 2 };
                let root = q_root.iter().next();
                for e in q_canvas.iter().chain(q_sub.iter()) {
                    commands.entity(e).despawn();
                }
                if let Some(r) = root {
                    commands.entity(r).with_children(|parent| {
                        parent
                            .spawn((
                                SettingsCanvas,
                                Node {
                                    width: Val::Px(960.0),
                                    height: Val::Px(540.0),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                            ))
                            .with_children(|canvas| {
                                spawn_canvas_content(canvas, &asset_server, &settings, page);
                            });
                    });
                }
                return;
            }
            NavTarget::SubOpen => {
                if q_sub.is_empty() {
                    if let Some(c) = q_canvas.iter().next() {
                        commands.entity(c).with_children(|parent| {
                            spawn_sub_screen(parent, &asset_server, &settings);
                        });
                    }
                }
                return;
            }
            NavTarget::SubClose => {
                for e in q_sub.iter() {
                    commands.entity(e).despawn();
                }
                return;
            }
        }
    }
}

fn handle_value_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    q_slider: Query<(&SliderZone, &Interaction)>,
    q_toggle: Query<(&ToggleSetting, &Interaction)>,
    mut settings: ResMut<GameSettings>,
    mut config: ResMut<VnEngineConfig>,
    mut theme: ResMut<VnTheme>,
    mut writer: MessageWriter<SetVolumeEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }

    let mut changed = false;
    for (zone, inter) in q_slider.iter() {
        if *inter != Interaction::Pressed { continue; }
        let val = zone.zone as f32 / (SLIDER_ZONES - 1) as f32 * 100.0;
        changed |= set_slider_value(&mut settings, zone.kind, val);
    }
    for (tg, inter) in q_toggle.iter() {
        if *inter != Interaction::Pressed { continue; }
        changed |= apply_toggle(&mut settings, tg.kind, tg.value);
    }
    if changed {
        sync_to_engine(&settings, &mut config, &mut theme, &mut writer);
        save_settings(&settings, &config.save_dir);
    }
}

fn set_slider_value(s: &mut GameSettings, kind: SliderKind, val: f32) -> bool {
    let v = val.round();
    match kind {
        SliderKind::MwAlpha if s.mw_alpha != v => { s.mw_alpha = v; true }
        SliderKind::Aspeed if s.aspeed != v => { s.aspeed = v; true }
        SliderKind::Mspeed if s.mspeed != v => { s.mspeed = v; true }
        SliderKind::Master if s.master != v => { s.master = v; true }
        SliderKind::Bgm if s.bgm != v => { s.bgm = v; true }
        SliderKind::Bgmvo if s.bgmvo != v => { s.bgmvo = v; true }
        SliderKind::Voice if s.voice != v => { s.voice = v; true }
        SliderKind::Se if s.se != v => { s.se = v; true }
        SliderKind::Sysse if s.sysse != v => { s.sysse = v; true }
        SliderKind::Movie if s.movie != v => { s.movie = v; true }
        SliderKind::VoChar(1) if s.c001 != v => { s.c001 = v; true }
        SliderKind::VoChar(2) if s.c002 != v => { s.c002 = v; true }
        SliderKind::VoChar(3) if s.c003 != v => { s.c003 = v; true }
        SliderKind::VoChar(4) if s.c004 != v => { s.c004 = v; true }
        SliderKind::VoChar(5) if s.c005 != v => { s.c005 = v; true }
        SliderKind::VoChar(6) if s.man != v => { s.man = v; true }
        SliderKind::VoChar(_) if s.fem != v => { s.fem = v; true }
        _ => false,
    }
}

fn apply_toggle(s: &mut GameSettings, kind: ToggleKind, value: i32) -> bool {
    let b = value != 0;
    match kind {
        ToggleKind::Effect if s.effect != value => { s.effect = value; true }
        ToggleKind::Messkip if s.messkip != value => { s.messkip = value; true }
        ToggleKind::Rclick if s.rclick != value => { s.rclick = value; true }
        ToggleKind::FlMspeed if s.fl_mspeed != b => { s.fl_mspeed = b; true }
        ToggleKind::FlMaster if s.fl_master != b => { s.fl_master = b; true }
        ToggleKind::FlBgm if s.fl_bgm != b => { s.fl_bgm = b; true }
        ToggleKind::FlBgmvo if s.fl_bgmvo != b => { s.fl_bgmvo = b; true }
        ToggleKind::FlVoice if s.fl_voice != b => { s.fl_voice = b; true }
        ToggleKind::FlSe if s.fl_se != b => { s.fl_se = b; true }
        ToggleKind::FlSysse if s.fl_sysse != b => { s.fl_sysse = b; true }
        ToggleKind::FlMovie if s.fl_movie != b => { s.fl_movie = b; true }
        ToggleKind::Voiceskip if s.voiceskip != b => { s.voiceskip = b; true }
        ToggleKind::Shadow if s.shadow != b => { s.shadow = b; true }
        ToggleKind::Outline if s.outline != b => { s.outline = b; true }
        ToggleKind::FlChar(1) if s.fl_c001 != b => { s.fl_c001 = b; true }
        ToggleKind::FlChar(2) if s.fl_c002 != b => { s.fl_c002 = b; true }
        ToggleKind::FlChar(3) if s.fl_c003 != b => { s.fl_c003 = b; true }
        ToggleKind::FlChar(4) if s.fl_c004 != b => { s.fl_c004 = b; true }
        ToggleKind::FlChar(5) if s.fl_c005 != b => { s.fl_c005 = b; true }
        ToggleKind::FlChar(6) if s.fl_man != b => { s.fl_man = b; true }
        ToggleKind::FlChar(_) if s.fl_fem != b => { s.fl_fem = b; true }
        ToggleKind::Reset(k) => {
            let before = slider_value(s, k);
            let _ = set_slider_value(s, k, 100.0);
            slider_value(s, k) != before
        }
        _ => false,
    }
}

fn sync_to_engine(
    s: &GameSettings,
    config: &mut VnEngineConfig,
    theme: &mut VnTheme,
    writer: &mut MessageWriter<SetVolumeEvent>,
) {
    let master = s.master / 100.0;
    let bgm = if s.fl_bgm && s.fl_bgmvo { master * s.bgm / 100.0 } else { 0.0 };
    let se = if s.fl_se && s.fl_sysse { master * s.se / 100.0 } else { 0.0 };
    let voice = if s.fl_voice { master * s.voice / 100.0 } else { 0.0 };
    writer.write(SetVolumeEvent { bgm: Some(bgm), se: Some(se), voice: Some(voice) });

    config.text_speed = if s.fl_mspeed { s.mspeed as f64 } else { 0.0 };
    config.auto_delay = s.aspeed as f64 / 10.0;
    theme.dialogue.text_speed = Some(config.text_speed);
    theme.dialogue.background_color[3] = s.mw_alpha / 100.0;
}

// ── Visual sync ──

fn update_slider_visuals(
    settings: Res<GameSettings>,
    q_zone: Query<(&SliderZone, &Interaction)>,
    mut q_pin: Query<(&SliderPin, &mut Node)>,
    mut q_pin_img: Query<(&SliderPin, &mut ImageNode)>,
    mut q_val: Query<(&SliderValue, &mut Text)>,
) {
    for (pin, mut node) in q_pin.iter_mut() {
        let val = slider_value(&settings, pin.kind);
        node.left = Val::Px(pin.track_left + (val / 100.0) * (pin.track_w - pin.pin_w));
    }
    // Pin sprite sheets hold two states side by side: light knob (idle) at
    // x=0..pin_w, dark knob (dragged) at x=pin_w..2*pin_w.
    for (pin, mut img) in q_pin_img.iter_mut() {
        let dragging = q_zone
            .iter()
            .any(|(z, i)| z.kind == pin.kind && *i == Interaction::Pressed);
        if let Some(rect) = &mut img.rect {
            let x0 = if dragging { pin.pin_w } else { 0.0 };
            rect.min.x = x0;
            rect.max.x = x0 + pin.pin_w;
        }
    }
    for (val, mut text) in q_val.iter_mut() {
        text.0 = format!("{:>3}", slider_value(&settings, val.kind) as i32);
    }
}

fn update_toggle_visuals(
    settings: Res<GameSettings>,
    mut q: Query<(&ToggleSetting, &mut ImageNode)>,
) {
    for (tg, mut img) in q.iter_mut() {
        if matches!(tg.kind, ToggleKind::Reset(_)) {
            continue;
        }
        let on = match tg.kind {
            ToggleKind::Effect => settings.effect == tg.value,
            ToggleKind::Messkip => settings.messkip == tg.value,
            ToggleKind::Rclick => settings.rclick == tg.value,
            _ => toggle_value(&settings, tg.kind),
        };
        // Circle toggles (button-circle.png): horizontal 20px frames,
        // selected = yellow (x=20..40). Checkboxes (check0N.png): vertical
        // 15px states, checked = black box (y=31..47).
        let (dx, dy) = match tg.kind {
            ToggleKind::Effect | ToggleKind::Messkip | ToggleKind::Rclick
            | ToggleKind::FlMspeed | ToggleKind::FlChar(_)
            | ToggleKind::Shadow | ToggleKind::Outline => {
                if on { (20.0, 0.0) } else { (0.0, 0.0) }
            }
            _ => {
                if on { (0.0, 31.0) } else { (0.0, 0.0) }
            }
        };
        if let Some(rect) = &mut img.rect {
            let w = rect.max.x - rect.min.x;
            let h = rect.max.y - rect.min.y;
            rect.min.x = dx;
            rect.max.x = dx + w;
            rect.min.y = dy;
            rect.max.y = dy + h;
        }
    }
}

fn slider_value(s: &GameSettings, kind: SliderKind) -> f32 {
    match kind {
        SliderKind::MwAlpha => s.mw_alpha,
        SliderKind::Aspeed => s.aspeed,
        SliderKind::Mspeed => s.mspeed,
        SliderKind::Master => s.master,
        SliderKind::Bgm => s.bgm,
        SliderKind::Bgmvo => s.bgmvo,
        SliderKind::Voice => s.voice,
        SliderKind::Se => s.se,
        SliderKind::Sysse => s.sysse,
        SliderKind::Movie => s.movie,
        SliderKind::VoChar(1) => s.c001,
        SliderKind::VoChar(2) => s.c002,
        SliderKind::VoChar(3) => s.c003,
        SliderKind::VoChar(4) => s.c004,
        SliderKind::VoChar(5) => s.c005,
        SliderKind::VoChar(6) => s.man,
        SliderKind::VoChar(_) => s.fem,
    }
}

fn toggle_value(s: &GameSettings, kind: ToggleKind) -> bool {
    match kind {
        ToggleKind::Effect => s.effect != 0,
        ToggleKind::Messkip => s.messkip != 0,
        ToggleKind::Rclick => s.rclick != 0,
        ToggleKind::FlMspeed => s.fl_mspeed,
        ToggleKind::FlMaster => s.fl_master,
        ToggleKind::FlBgm => s.fl_bgm,
        ToggleKind::FlBgmvo => s.fl_bgmvo,
        ToggleKind::FlVoice => s.fl_voice,
        ToggleKind::FlSe => s.fl_se,
        ToggleKind::FlSysse => s.fl_sysse,
        ToggleKind::FlMovie => s.fl_movie,
        ToggleKind::Voiceskip => s.voiceskip,
        ToggleKind::Shadow => s.shadow,
        ToggleKind::Outline => s.outline,
        ToggleKind::FlChar(1) => s.fl_c001,
        ToggleKind::FlChar(2) => s.fl_c002,
        ToggleKind::FlChar(3) => s.fl_c003,
        ToggleKind::FlChar(4) => s.fl_c004,
        ToggleKind::FlChar(5) => s.fl_c005,
        ToggleKind::FlChar(6) => s.fl_man,
        ToggleKind::FlChar(_) => s.fl_fem,
        ToggleKind::Reset(_) => false,
    }
}

fn teardown_settings(
    mut commands: Commands,
    q: Query<Entity, With<SettingsScreen>>,
    settings: Res<GameSettings>,
    config: Res<VnEngineConfig>,
) {
    save_settings(&settings, &config.save_dir);
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}
