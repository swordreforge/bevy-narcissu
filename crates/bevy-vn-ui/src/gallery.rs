//! Extra screen — CG gallery + BGM gallery, faithful to the original
//! Narcissu 10th Anniversary layout (960x540 logical pixels).
//!
//! Original source: `assets/system/extra/{cg,bgm}.lua` + `ui_cgmode` /
//! `ui_bgmmode` templates in `list_windows_ja.tbl`. Mode switch via the
//! `sys01`/`sys02` buttons on `extra/btn.png`.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy_vn_audio::bgm::BgmManager;
use bevy_vn_core::messages::{PlayBgmEvent, SetVolumeEvent, StopBgmEvent};
use bevy_vn_core::state::{VnAppState, VnMenuState};

const BG_CG: &str = "ui/extra/gallery-bg.png";
const BG_BGM: &str = "ui/extra/music-bg.png";
const BTN: &str = "ui/extra/btn.png";
const BTN2: &str = "ui/extra/btn2.png";
const PIN: &str = "ui/extra/pin.png";
const BGM_THUMB: &str = "ui/extra/bgm/bgm-";
const CG_THUMB: &str = "image/thumb/_th_ev";

const BGM_COUNT: u32 = 47;
const CG_COUNT: u32 = 23;
const CG_PER_PAGE: u32 = 6;
const CG_PAGES: u32 = 4;

/// Which extra sub-screen is active.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum ExtraMode { Cg, Bgm }

#[derive(Component)]
struct ExtraScreen;

#[derive(Component)]
struct ModeButton(ExtraMode);

#[derive(Component)]
struct BgmButton(u32);

#[derive(Component)]
struct CgButton(u32);

#[derive(Component)]
struct PlayerButton(PlayerAction);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum PlayerAction { Play, Stop, Back, Next }

#[derive(Component)]
struct BackButton;

#[derive(Component)]
struct PrevPage;

#[derive(Component)]
struct NextPage;

#[derive(Component)]
struct Viewer;

#[derive(Component)]
struct ViewerClose;

/// Volume slider track (clickable) + its draggable pin.
#[derive(Component)]
struct VolumeSlider;

#[derive(Component)]
struct VolumePin;

/// Marks a background node as belonging to a specific mode (CG or BGM).
#[derive(Component)]
struct ExtraBg(ExtraMode);

/// Marks a node as visible only in a specific mode.
#[derive(Component)]
struct ExtraModeOnly(ExtraMode);

/// Marks a CG slot as belonging to a page (0-based).
#[derive(Component)]
struct CgPage(u32);

#[derive(Resource, Default)]
struct PageState { cg: u32 }

/// One BGM list row: file id (1..47), grid column/row.
struct BgmSlot { id: u32, x: f32, y: f32 }

pub struct GalleryPlugin;
impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PageState>()
            .add_systems(OnEnter(VnMenuState::Gallery), spawn_gallery)
            .add_systems(
                Update,
                (
                    handle_mode_switch,
                    handle_bgm_play,
                    handle_cg_view,
                    handle_player,
                    handle_volume,
                    handle_back,
                    handle_paging,
                    handle_viewer_click,
                    update_bgm_state,
                    update_volume_pin,
                )
                    .chain(),
            )
            .add_systems(OnExit(VnMenuState::Gallery), despawn_gallery);
    }
}

fn spawn_gallery(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bg_cg = asset_server.load::<Image>(BG_CG);
    let bg_bgm = asset_server.load::<Image>(BG_BGM);
    let btn = asset_server.load::<Image>(BTN);
    let btn2 = asset_server.load::<Image>(BTN2);
    let pin = asset_server.load::<Image>(PIN);

    commands.spawn((
        ExtraScreen,
        Node {
            width: percent(100), height: percent(100),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|root| {
        root.spawn((
            Node {
                width: Val::Px(960.0), height: Val::Px(540.0),
                flex_shrink: 0.0, ..default()
            },
        ))
        .with_children(|canvas| {
            // Backgrounds — CG visible by default, BGM hidden.
            canvas.spawn((
                ExtraBg(ExtraMode::Cg),
                ImageNode { image: bg_cg, image_mode: NodeImageMode::Stretch, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0), top: Val::Px(0.0),
                    width: Val::Px(960.0), height: Val::Px(540.0),
                    ..default()
                },
                ZIndex(0),
            ));
            canvas.spawn((
                ExtraBg(ExtraMode::Bgm),
                ImageNode { image: bg_bgm, image_mode: NodeImageMode::Stretch, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0), top: Val::Px(0.0),
                    width: Val::Px(960.0), height: Val::Px(540.0),
                    display: Display::None,
                    ..default()
                },
                ZIndex(0),
            ));

            // Pins (decorative, both modes) — from btn.png clip 440,144.
            for (px, py) in [(-2.0, -13.0), (0.0, -11.0)] {
                canvas.spawn((
                    ImageNode { image: btn.clone(), rect: Some(Rect::new(440.0, 144.0, 455.0, 159.0)), ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px), top: Val::Px(py),
                        width: Val::Px(15.0), height: Val::Px(15.0),
                        ..default()
                    },
                    ZIndex(1),
                ));
            }

            // Mode switch buttons (shared by both modes).
            mode_button(canvas, &btn, ExtraMode::Cg, 657.0, 32.0, 0.0);
            mode_button(canvas, &btn, ExtraMode::Bgm, 775.0, 32.0, 135.0);

            // Exit button: clip 272,0 (normal), placed at 760,500 (both modes).
            canvas.spawn((
                BackButton,
                Button,
                ImageNode { image: btn.clone(), rect: Some(Rect::new(272.0, 0.0, 407.0, 32.0)), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(760.0), top: Val::Px(500.0),
                    width: Val::Px(135.0), height: Val::Px(32.0),
                    ..default()
                },
                ZIndex(2),
            ));

            // ── CG grid (4 pages × 6 slots, 240x135 each) ──
            for page in 0..CG_PAGES {
                let base = page * CG_PER_PAGE;
                for i in 0..CG_PER_PAGE {
                    let idx = base + i + 1;
                    if idx > CG_COUNT { continue; }
                    let x = 101.0 + (i % 3) as f32 * 259.0;
                    let y = 182.0 + (i / 3) as f32 * 156.0;
                    let thumb = asset_server.load::<Image>(&format!("{CG_THUMB}{idx:02}.png"));
                    canvas.spawn((
                        CgButton(idx),
                        CgPage(page),
                        ExtraModeOnly(ExtraMode::Cg),
                        Button,
                        ImageNode { image: btn2.clone(), rect: Some(Rect::new(0.0, 0.0, 240.0, 135.0)), ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x), top: Val::Px(y),
                            width: Val::Px(240.0), height: Val::Px(135.0),
                            display: if page == 0 { Display::Flex } else { Display::None },
                            ..default()
                        },
                        ZIndex(2),
                    ))
                    .with_child((
                        ImageNode { image: thumb, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0), top: Val::Px(0.0),
                            width: Val::Px(240.0), height: Val::Px(135.0),
                            ..default()
                        },
                        ZIndex(3),
                    ));
                }
            }

            // Page navigation (left/right arrows, CG mode only).
            canvas.spawn((
                PrevPage, ExtraModeOnly(ExtraMode::Cg),
                Button,
                ImageNode { image: btn.clone(), rect: Some(Rect::new(368.0, 144.0, 392.0, 168.0)), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(54.0), top: Val::Px(300.0),
                    width: Val::Px(24.0), height: Val::Px(24.0),
                    ..default()
                },
                ZIndex(2),
            ));
            canvas.spawn((
                NextPage, ExtraModeOnly(ExtraMode::Cg),
                Button,
                ImageNode { image: btn.clone(), rect: Some(Rect::new(368.0, 168.0, 392.0, 192.0)), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(881.0), top: Val::Px(300.0),
                    width: Val::Px(24.0), height: Val::Px(24.0),
                    ..default()
                },
                ZIndex(2),
            ));

            // ── BGM list: 4 cols × 12 rows (47 covers, hidden until BGM mode) ──
            for slot in bgm_slots() {
                let img = asset_server.load::<Image>(&format!("{BGM_THUMB}{}.png", slot.id));
                canvas.spawn((
                    BgmButton(slot.id),
                    ExtraModeOnly(ExtraMode::Bgm),
                    Button,
                    ImageNode {
                        image: img,
                        rect: Some(Rect::new(0.0, 0.0, 176.0, 28.0)),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(slot.x), top: Val::Px(slot.y),
                        width: Val::Px(176.0), height: Val::Px(28.0),
                        display: Display::None,
                        ..default()
                    },
                    ZIndex(2),
                ));
            }

            // BGM player controls (play/stop/prev/next, BGM mode only).
            player_button(canvas, &btn, PlayerAction::Back, 202.0, 430.0, 576.0, 32.0);
            player_button(canvas, &btn, PlayerAction::Play, 78.0, 430.0, 480.0, 63.0);
            player_button(canvas, &btn, PlayerAction::Stop, 155.0, 430.0, 543.0, 32.0);
            player_button(canvas, &btn, PlayerAction::Next, 254.0, 430.0, 608.0, 32.0);

            // Volume slider: track from btn.png clip 639,96 (189x24), pin 19x24 from pin.png.
            canvas.spawn((
                VolumeSlider,
                ExtraModeOnly(ExtraMode::Bgm),
                Button,
                RelativeCursorPosition::default(),
                ImageNode { image: btn.clone(), rect: Some(Rect::new(639.0, 96.0, 828.0, 120.0)), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(98.0), top: Val::Px(395.0),
                    width: Val::Px(189.0), height: Val::Px(24.0),
                    display: Display::None,
                    ..default()
                },
                ZIndex(2),
            ))
            .with_child((
                VolumePin,
                ImageNode { image: pin.clone(), rect: Some(Rect::new(0.0, 0.0, 19.0, 24.0)), ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0), top: Val::Px(0.0),
                    width: Val::Px(19.0), height: Val::Px(24.0),
                    ..default()
                },
                ZIndex(3),
            ));
        });
    });
}

fn mode_button(canvas: &mut ChildSpawnerCommands, btn: &Handle<Image>, mode: ExtraMode, x: f32, y: f32, clip_x: f32) {
    canvas.spawn((
        ModeButton(mode),
        Button,
        ImageNode {
            image: btn.clone(),
            rect: Some(Rect::new(clip_x, 0.0, clip_x + 135.0, 32.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x), top: Val::Px(y),
            width: Val::Px(135.0), height: Val::Px(32.0),
            ..default()
        },
        ZIndex(2),
    ));
}

fn player_button(
    canvas: &mut ChildSpawnerCommands, btn: &Handle<Image>, action: PlayerAction,
    x: f32, y: f32, clip_x: f32, w: f32,
) {
    canvas.spawn((
        PlayerButton(action),
        ExtraModeOnly(ExtraMode::Bgm),
        Button,
        ImageNode {
            image: btn.clone(),
            rect: Some(Rect::new(clip_x, 96.0, clip_x + w, 127.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x), top: Val::Px(y),
            width: Val::Px(w), height: Val::Px(32.0),
            display: Display::None,
            ..default()
        },
        ZIndex(2),
    ));
}

/// BGM grid layout — 4 columns at x=368/508/648/788, rows start y=144 step 33.
fn bgm_slots() -> Vec<BgmSlot> {
    let xs = [368.0, 508.0, 648.0, 788.0];
    (1..=BGM_COUNT)
        .map(|id| BgmSlot {
            id,
            x: xs[((id - 1) % 4) as usize],
            y: 144.0 + ((id - 1) / 4) as f32 * 33.0,
        })
        .collect()
}

fn handle_mode_switch(
    q_mode: Query<(&ModeButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    page: Res<PageState>,
    mut nodes: ParamSet<(
        Query<(&mut Node, &ExtraBg), Without<ExtraModeOnly>>,
        Query<(&mut Node, &ExtraModeOnly), (Without<ExtraBg>, Without<CgPage>)>,
        Query<(&CgPage, &mut Node), (With<CgButton>, Without<ExtraBg>)>,
        Query<&mut Node, With<BackButton>>,
    )>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for (mb, inter) in &q_mode {
        if *inter != Interaction::Pressed { continue; }
        let target = mb.0;
        for (mut node, bg) in nodes.p0().iter_mut() {
            node.display = if bg.0 == target { Display::Flex } else { Display::None };
        }
        for (mut node, mo) in nodes.p1().iter_mut() {
            node.display = if mo.0 == target { Display::Flex } else { Display::None };
        }
        for (cp, mut node) in nodes.p2().iter_mut() {
            let visible = target == ExtraMode::Cg && cp.0 == page.cg;
            node.display = if visible { Display::Flex } else { Display::None };
        }
        for mut node in nodes.p3().iter_mut() {
            node.left = Val::Px(if target == ExtraMode::Cg { 760.0 } else { 130.0 });
        }
    }
}

fn handle_bgm_play(
    q_bgm: Query<(&BgmButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: MessageWriter<PlayBgmEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for (bt, inter) in &q_bgm {
        if *inter != Interaction::Pressed { continue; }
        writer.write(PlayBgmEvent { id: bt.0.to_string(), volume: None, fade_ms: None });
    }
}

fn handle_cg_view(
    q_cg: Query<(&CgButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for (bt, inter) in &q_cg {
        if *inter != Interaction::Pressed { continue; }
        spawn_viewer(&mut commands, &asset_server, bt.0);
    }
}

fn handle_player(
    q_player: Query<(&PlayerButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    bgm: Res<BgmManager>,
    mut writer: MessageWriter<PlayBgmEvent>,
    mut stop: MessageWriter<StopBgmEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for (pb, inter) in &q_player {
        if *inter != Interaction::Pressed { continue; }
        match pb.0 {
            PlayerAction::Play => {
                if let Some(cur) = bgm.current_id.clone() {
                    writer.write(PlayBgmEvent { id: cur, volume: None, fade_ms: None });
                }
            }
            PlayerAction::Stop => {
                let _ = stop.write(StopBgmEvent { fade_ms: None });
            }
            PlayerAction::Back => {
                if let Some(cur) = bgm.current_id.clone() {
                    let n = step_bgm(&cur, -1);
                    writer.write(PlayBgmEvent { id: n.to_string(), volume: None, fade_ms: None });
                }
            }
            PlayerAction::Next => {
                if let Some(cur) = bgm.current_id.clone() {
                    let n = step_bgm(&cur, 1);
                    writer.write(PlayBgmEvent { id: n.to_string(), volume: None, fade_ms: None });
                }
            }
        }
    }
}

/// Volume slider: click/drag on track → set volume.
/// RelativeCursorPosition.normalized is centered on the node (-0.5..0.5).
fn handle_volume(
    q_slider: Query<(&VolumeSlider, &Interaction, &RelativeCursorPosition)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: MessageWriter<SetVolumeEvent>,
) {
    if !mouse.pressed(MouseButton::Left) { return; }
    for (_, inter, rel) in &q_slider {
        if *inter != Interaction::Pressed { continue; }
        if let Some(n) = rel.normalized {
            let t = (n.x + 0.5).clamp(0.0, 1.0);
            writer.write(SetVolumeEvent { bgm: Some(t), se: None, voice: None });
        }
    }
}

fn handle_back(
    q_back: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
    mouse: Res<ButtonInput<MouseButton>>,    mut stop: MessageWriter<StopBgmEvent>,
    mut next_menu: ResMut<NextState<VnMenuState>>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for inter in &q_back {
        if *inter == Interaction::Pressed {
            let _ = stop.write(StopBgmEvent { fade_ms: None });
            next_menu.set(VnMenuState::Main);
            next.set(VnAppState::Title);
        }
    }
}

fn handle_paging(
    q_prev: Query<&Interaction, (With<PrevPage>, Changed<Interaction>)>,
    q_next: Query<&Interaction, (With<NextPage>, Changed<Interaction>)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut page: ResMut<PageState>,
    mut cg_page: Query<(&CgButton, &CgPage, &mut Node)>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for inter in &q_prev {
        if *inter == Interaction::Pressed && page.cg > 0 {
            page.cg -= 1;
            refresh_cg_page(&mut cg_page, page.cg);
        }
    }
    for inter in &q_next {
        if *inter == Interaction::Pressed && page.cg + 1 < CG_PAGES {
            page.cg += 1;
            refresh_cg_page(&mut cg_page, page.cg);
        }
    }
}

fn step_bgm(cur: &str, d: i32) -> u32 {
    let n: i32 = cur.parse().unwrap_or(1);
    let mut n = n + d;
    if n < 1 { n = BGM_COUNT as i32; }
    if n > BGM_COUNT as i32 { n = 1; }
    n as u32
}

fn refresh_cg_page(q: &mut Query<(&CgButton, &CgPage, &mut Node)>, page: u32) {
    for (_, cp, mut node) in q.iter_mut() {
        node.display = if cp.0 == page { Display::Flex } else { Display::None };
    }
}

/// Highlight the currently playing BGM cover (clip to the "selected" region).
fn update_bgm_state(
    bgm: Res<BgmManager>,
    mut q: Query<(&BgmButton, &mut ImageNode)>,
) {
    let cur: u32 = bgm.current_id.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0);
    for (bt, mut img) in &mut q {
        let selected = bt.0 == cur;
        let rx = if selected { 176.0 } else { 0.0 };
        img.rect = Some(Rect::new(rx, 0.0, rx + 176.0, 28.0));
    }
}

/// Keep the volume pin aligned with the current BGM volume (189px track, 19px pin).
fn update_volume_pin(
    bgm: Res<BgmManager>,
    mut q: Query<&mut Node, With<VolumePin>>,
) {
    let max_left = 189.0 - 19.0;
    let left = (bgm.volume.clamp(0.0, 1.0) * max_left) as f32;
    for mut node in &mut q {
        node.left = Val::Px(left);
    }
}

fn spawn_viewer(commands: &mut Commands, asset_server: &AssetServer, idx: u32) {
    if idx > CG_COUNT { return; }
    let path = format!("{CG_THUMB}{idx:02}.png");
    commands.spawn((
        Viewer,
        Node {
            width: percent(100), height: percent(100),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ZIndex(50),
    ))
    .with_children(|root| {
        root.spawn((
            ImageNode { image: asset_server.load::<Image>(&path), ..default() },
            Node {
                width: Val::Px(480.0), height: Val::Px(270.0),
                ..default()
            },
            ZIndex(51),
        ));
        root.spawn((
            ViewerClose,
            Button,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(40.0), top: Val::Px(40.0),
                width: Val::Px(64.0), height: Val::Px(48.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
            ZIndex(52),
        ))
        .with_child((
            Text::new("✕"),
            TextFont { font_size: FontSize::Px(28.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn handle_viewer_click(
    mut commands: Commands,
    q: Query<(&ViewerClose, &Interaction)>,
    viewers: Query<Entity, With<Viewer>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    for (_, inter) in &q {
        if *inter == Interaction::Pressed {
            for e in &viewers { commands.entity(e).despawn(); }
        }
    }
}

fn despawn_gallery(mut commands: Commands, q: Query<Entity, With<ExtraScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
