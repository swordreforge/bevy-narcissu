//! Save/Load screen UI — sprite-based, faithful to the original Narcissu 10th
//! Anniversary layout (960×540 logical pixels). Driven by the `SaveLoadMode`
//! resource rather than OnEnter/OnExit state hooks, so the in-game ESC entry
//! (which stays inside `VnAppState::Gameplay`) works without state transitions.

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy_vn_core::messages::SetBgEvent;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_core::state::{
    OverlayToggle, SaveLoadKind, SaveLoadMode, SaveLoadReturn, VnAppState, VnMenuState, VnTransition,
};
use bevy_vn_render::bg::BgState;
use bevy_vn_render::AssetPathProvider;
use bevy_vn_save::{SaveManager, SlotMeta};

use crate::backlog::BacklogState;
use crate::chapter_names::chapter_title;
use crate::transition::{request_overlay, request_transition_with_overlay};

const Z_BG: i32 = 0;
const Z_SLOT: i32 = 1;
const Z_TEXT: i32 = 2;
const Z_CTRL: i32 = 3;

const BTN_PATH: &str = "pa/ja/save/btn.png";
const BTNSYS_PATH: &str = "pa/ja/save/btnsys.png";
const NONE_PATH: &str = "pa/ja/save/none.png";
const NEW_PATH: &str = "pa/ja/save/new.png";

const SLOT_W: f32 = 176.0;
const SLOT_H: f32 = 96.0;

const SLOT_POSITIONS: [(f32, f32); 8] = [
    (60.0, 125.0),
    (267.0, 125.0),
    (475.0, 125.0),
    (690.0, 125.0),
    (60.0, 328.0),
    (267.0, 328.0),
    (475.0, 328.0),
    (690.0, 328.0),
];

#[derive(Component)]
struct SaveLoadScreen;

#[derive(Component)]
struct SlotButton(usize);

#[derive(Component)]
struct SlotFrame;

#[derive(Component)]
struct DeleteButton(usize);

#[derive(Component)]
struct PageButton(usize);

#[derive(Component)]
struct BackButton;

#[derive(Resource, Default)]
struct SaveLoadUiState {
    page: usize,
    dirty: bool,
}

pub struct SaveLoadUiPlugin;

impl Plugin for SaveLoadUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveLoadUiState>().add_systems(
            Update,
            (
                update_save_load_ui,
                handle_save_load_clicks,
                update_slot_visuals,
            ),
        );
    }
}

fn update_save_load_ui(
    mode: Res<SaveLoadMode>,
    mut ui_state: ResMut<SaveLoadUiState>,
    q_root: Query<Entity, With<SaveLoadScreen>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mgr: Res<SaveManager>,
    provider: Res<AssetPathProvider>,
) {
    if mode.active && q_root.is_empty() {
        spawn_save_load_screen(
            &mut commands,
            &asset_server,
            &provider,
            &mgr,
            *mode,
            ui_state.page,
        );
    } else if !mode.active && !q_root.is_empty() {
        for e in q_root.iter() {
            commands.entity(e).despawn();
        }
        ui_state.page = 0;
    } else if mode.active && ui_state.dirty && !q_root.is_empty() {
        for e in q_root.iter() {
            commands.entity(e).despawn();
        }
        ui_state.dirty = false;
        spawn_save_load_screen(
            &mut commands,
            &asset_server,
            &provider,
            &mgr,
            *mode,
            ui_state.page,
        );
    }
}

fn handle_save_load_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    q_root: Query<Entity, With<SaveLoadScreen>>,
    q_slot: Query<(&SlotButton, &Interaction)>,
    q_del: Query<(&DeleteButton, &Interaction)>,
    q_page: Query<(&PageButton, &Interaction)>,
    q_back: Query<&Interaction, With<BackButton>>,
    mut mgr: ResMut<SaveManager>,
    mut engine: ResMut<ScriptEngine>,
    bg: Res<BgState>,
    backlog: Res<BacklogState>,
    mode: Res<SaveLoadMode>,
    mut ui_state: ResMut<SaveLoadUiState>,
    mut transition: ResMut<VnTransition>,
    mut wbg: MessageWriter<SetBgEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if !mode.active {
        return;
    }
    if q_root.is_empty() {
        return;
    }

    let kind = mode.kind;
    let return_to = mode.return_to;

    for (slot, inter) in q_slot.iter() {
        if *inter != Interaction::Pressed {
            continue;
        }
        let index = slot.0;
        match kind {
            SaveLoadKind::Save => {
                if engine.current_script.is_empty() {
                    continue;
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let desc = fmt_time(now);
                let n = backlog.entries.len();
                let start = n.saturating_sub(3);
                let preview: String = backlog.entries[start..]
                    .iter()
                    .map(|e| e.text.as_str())
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                let last_dialogue = backlog
                    .entries
                    .last()
                    .map(|e| (e.speaker.clone(), e.text.clone()));
                let meta = SlotMeta {
                    bg: bg.current_bg.clone(),
                    chapter: chapter_title(&engine.current_script).map(str::to_string),
                    preview: if preview.is_empty() {
                        None
                    } else {
                        Some(preview)
                    },
                    last_dialogue,
                };
                if let Err(e) = mgr.save_with_meta(index, &engine, &desc, meta) {
                    warn!("save failed: {e}");
                }
                ui_state.dirty = true;
            }
            SaveLoadKind::Load => {
                if mgr.slots[index].is_none() {
                    continue;
                }
                let bg_key = mgr.slots[index].as_ref().and_then(|s| s.meta.bg.clone());
                if let Err(e) = mgr.load(index, &mut engine) {
                    warn!("load failed: {e}");
                    continue;
                }
                if let Some(key) = bg_key {
                    let _ = wbg.write(SetBgEvent {
                        image: key,
                        transition: None,
                    });
                }
                request_transition_with_overlay(
                    &mut transition,
                    Some(VnAppState::Gameplay),
                    None,
                    vec![OverlayToggle::SaveLoadClose],
                );
                return;
            }
        }
    }

    for (del, inter) in q_del.iter() {        if *inter != Interaction::Pressed {
            continue;
        }
        if let Err(e) = mgr.delete(del.0) {
            warn!("delete failed: {e}");
        }
        ui_state.dirty = true;
    }

    for (page, inter) in q_page.iter() {
        if *inter != Interaction::Pressed {
            continue;
        }
        ui_state.page = page.0.min(2);
        ui_state.dirty = true;
    }

    for inter in q_back.iter() {
        if *inter != Interaction::Pressed {
            continue;
        }
        match return_to {
            SaveLoadReturn::Title => {
                request_transition_with_overlay(
                    &mut transition,
                    Some(VnAppState::Title),
                    Some(VnMenuState::Main),
                    vec![OverlayToggle::SaveLoadClose],
                );
            }
            SaveLoadReturn::Settings => {
                request_transition_with_overlay(
                    &mut transition,
                    None,
                    Some(VnMenuState::Settings),
                    vec![OverlayToggle::SaveLoadClose],
                );
            }
            SaveLoadReturn::Gameplay => {
                request_overlay(&mut transition, vec![OverlayToggle::SaveLoadClose]);
            }
        }
        return;
    }
}

fn update_slot_visuals(
    q_slot: Query<(Entity, &Interaction), With<SlotButton>>,
    mut q_frame: Query<(&ChildOf, &mut ImageNode), With<SlotFrame>>,
) {
    let slots: Vec<(Entity, Interaction)> = q_slot.iter().map(|(e, i)| (e, *i)).collect();
    for (child_of, mut img) in q_frame.iter_mut() {
        let inter = slots
            .iter()
            .find(|(e, _)| *e == child_of.parent())
            .map(|(_, i)| *i)
            .unwrap_or(Interaction::None);
        let rect = match inter {
            Interaction::Hovered => Rect::new(176.0, 48.0, 352.0, 144.0),
            Interaction::Pressed => Rect::new(352.0, 48.0, 528.0, 144.0),
            _ => Rect::new(0.0, 48.0, 176.0, 144.0),
        };
        img.rect = Some(rect);
    }
}

// ── Spawn helpers ──

fn spawn_save_load_screen(
    commands: &mut Commands,
    server: &AssetServer,
    provider: &AssetPathProvider,
    mgr: &SaveManager,
    mode: SaveLoadMode,
    page: usize,
) {
    let page = page.min(2);
    let bg_path = if mode.kind == SaveLoadKind::Save {
        "pa/ja/save/_save_base.png"
    } else {
        "pa/ja/save/_load_base.png"
    };

    commands
        .spawn((
            SaveLoadScreen,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|container| {
            // Fixed 960x540 canvas, centered in whatever window size.
            container
                .spawn((Node {
                    width: Val::Px(960.0),
                    height: Val::Px(540.0),
                    flex_shrink: 0.0,
                    ..default()
                },))
                .with_children(|root| {
                    let bg_img = server.load::<Image>(bg_path);
                    root.spawn((
                        ImageNode {
                            image: bg_img,
                            ..default()
                        },
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

                    let btn = server.load::<Image>(BTN_PATH);
                    let none_img = server.load::<Image>(NONE_PATH);
                    let new_img = server.load::<Image>(NEW_PATH);

                    // 原作 save.lua: new 标识贴最近存档槽 (no == saveslot.last)
                    let latest = mgr
                        .slots
                        .iter()
                        .enumerate()
                        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s.timestamp)))
                        .max_by_key(|&(_, t)| t)
                        .map(|(i, _)| i);

                    let slots_on_page = match page {
                        0 | 1 => 8,
                        2 => 4,
                        _ => 0,
                    };
                    for i in 0..slots_on_page {
                        let slot_idx = page * 8 + i;
                        let (x, y) = SLOT_POSITIONS[i];
                        spawn_slot(
                            root, &btn, &none_img, &new_img, server, provider, mgr, slot_idx, x, y, latest,
                        );
                    }

                    spawn_page_buttons(root, &btn, mode.kind, page);

                    let btnsys = server.load::<Image>(BTNSYS_PATH);
                    spawn_back_button(root, &btnsys);
                });
        });
}

fn spawn_slot(
    root: &mut ChildSpawnerCommands,
    btn: &Handle<Image>,
    none_img: &Handle<Image>,
    new_img: &Handle<Image>,
    server: &AssetServer,
    provider: &AssetPathProvider,
    mgr: &SaveManager,
    slot_idx: usize,
    x: f32,
    y: f32,
    latest: Option<usize>,
) {
    let slot = mgr.slots.get(slot_idx).and_then(|s| s.as_ref());
    let filled = slot.is_some();

    let thumb = if let Some(s) = slot {
        if let Some(ref key) = s.meta.bg {
            server.load::<Image>(&provider.bg(key))
        } else {
            none_img.clone()
        }
    } else {
        none_img.clone()
    };

    root.spawn((
        SlotButton(slot_idx),
        Button,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(SLOT_W),
            height: Val::Px(SLOT_H),
            ..default()
        },
        ZIndex(Z_SLOT),
    ))
    .with_children(|slot_parent| {
        slot_parent.spawn((
            ImageNode {
                image: thumb,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(SLOT_W),
                height: Val::Px(SLOT_H),
                ..default()
            },
            ZIndex(Z_SLOT),
        ));
        slot_parent.spawn((
            SlotFrame,
            ImageNode {
                image: btn.clone(),
                rect: Some(Rect::new(0.0, 48.0, 176.0, 144.0)),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(SLOT_W),
                height: Val::Px(SLOT_H),
                ..default()
            },
            ZIndex(Z_SLOT),
        ));
        if filled && latest == Some(slot_idx) {
            // 原作: new.png 32x32 贴最近存档槽左上角 (v.x + 0, v.y + 0)
            slot_parent.spawn((
                ImageNode {
                    image: new_img.clone(),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ZIndex(Z_SLOT + 1),
            ));
        }
    });

    let title = slot
        .and_then(|s| s.meta.chapter.clone())
        .unwrap_or_default();
    root.spawn((
        Text::new(title),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.0, 0.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y + 95.0),
            width: Val::Px(SLOT_W),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ZIndex(Z_TEXT),
    ));

    let preview_raw = slot
        .and_then(|s| s.meta.preview.clone())
        .unwrap_or_default();
    let preview = truncate_preview(&preview_raw);
    root.spawn((
        Text::new(preview),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.0, 0.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x + 3.0),
            top: Val::Px(y + 114.0),
            width: Val::Px(170.0),
            ..default()
        },
        ZIndex(Z_TEXT),
    ));

    if filled {
        root.spawn((
            DeleteButton(slot_idx),
            Button,
            ImageNode {
                image: btn.clone(),
                rect: Some(Rect::new(416.0, 48.0, 472.0, 63.0)),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x + 110.0),
                top: Val::Px(y + 81.0),
                width: Val::Px(56.0),
                height: Val::Px(15.0),
                ..default()
            },
            ZIndex(Z_CTRL),
        ));
    }
}

fn spawn_page_buttons(
    root: &mut ChildSpawnerCommands,
    btn: &Handle<Image>,
    kind: SaveLoadKind,
    page: usize,
) {
    let pages: [(usize, f32); 3] = [(0, 762.0), (1, 788.0), (2, 814.0)];
    for (p, x) in pages {
        // 页码按钮 clip 依原作 tbl:普通=y0-15 黑,当前页 clip_c=y31-46 红
        let rect = if p == page {
            Rect::new(p as f32 * 24.0, 31.0, p as f32 * 24.0 + 24.0, 46.0)
        } else {
            Rect::new(p as f32 * 24.0, 0.0, p as f32 * 24.0 + 24.0, 15.0)
        };
        root.spawn((
            PageButton(p),
            Button,
            ImageNode {
                image: btn.clone(),
                rect: Some(rect),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(83.0),
                width: Val::Px(24.0),
                height: Val::Px(15.0),
                ..default()
            },
            ZIndex(Z_CTRL),
        ));
    }

    // icon:下三角顶在当前页码上方(btn.png save y48-63 白 / load y63-78 红),
    // 依原作 tbl: icon x=0,y=-15 相对页码按钮,即 left=页码x, top=83-15
    if page <= 2 {
        let (_, x) = pages[page];
        let icon_rect = match kind {
            SaveLoadKind::Save => Rect::new(392.0, 48.0, 416.0, 63.0),
            SaveLoadKind::Load => Rect::new(392.0, 63.0, 416.0, 78.0),
        };
        root.spawn((
            ImageNode {
                image: btn.clone(),
                rect: Some(icon_rect),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(68.0),
                width: Val::Px(24.0),
                height: Val::Px(15.0),
                ..default()
            },
            ZIndex(Z_CTRL + 1),
        ));
    }

    // bt_page00: pagenew 按钮 — 回第一页(原作 x720 y86 w39 clip=351,48,39,15)
    root.spawn((
        PageButton(0),
        Button,
        ImageNode {
            image: btn.clone(),
            rect: Some(Rect::new(351.0, 48.0, 390.0, 63.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(720.0),
            top: Val::Px(86.0),
            width: Val::Px(39.0),
            height: Val::Px(15.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

fn spawn_back_button(root: &mut ChildSpawnerCommands, btnsys: &Handle<Image>) {
    root.spawn((
        BackButton,
        Button,
        ImageNode {
            image: btnsys.clone(),
            rect: Some(Rect::new(680.0, 0.0, 815.0, 32.0)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(710.0),
            top: Val::Px(510.0),
            width: Val::Px(135.0),
            height: Val::Px(32.0),
            ..default()
        },
        ZIndex(Z_CTRL),
    ));
}

// ── Utilities ──

fn truncate_preview(s: &str) -> String {
    s.lines()
        .take(2)
        .map(|l| {
            if l.chars().count() > 10 {
                let t: String = l.chars().take(10).collect();
                format!("{t}…")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fmt_time(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let (_, m, d) = civil_from_days(days);
    format!("{:02}/{:02} {:02}:{:02}", m, d, hh, mm)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + if m <= 2 { 1 } else { 0 }, m as u32, d as u32)
}
