//! In-game system menu — faithful to the original Narcissu 10th menu
//! (`ui_menu` from list_android_ja.tbl): a panel assembled from image
//! sprites — `mw/bg-mean.png` background + six buttons clipped from
//! `mw/btn-mean.png` (normal/hover/pressed states) + a back button from
//! `save/btnsys.png`. Opened with right-click / F2 / ESC while inside
//! `VnAppState::Gameplay`, mirroring 原作 `adv_menu()`. Stays inside
//! Gameplay so the scene survives; script advancement is blocked by
//! `GameplayMenuMode.active` in the runner.

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy_vn_core::messages::PlaySeEvent;
use bevy_vn_core::state::{
    GameplayMenuMode, SaveLoadKind, SaveLoadMode, SaveLoadReturn, SettingsOverlayMode, SkipMode,
    VnAppState,
};

use crate::backlog::BacklogState;

const BG_PATH: &str = "pa/ja/mw/bg-mean.png";
const BTN_PATH: &str = "pa/ja/mw/btn-mean.png";
const BTNSYS_PATH: &str = "pa/ja/save/btnsys.png";
const BLACK_PATH: &str = "image/bg/black.png";

const Z_FIL: i32 = 5;
const Z_BG: i32 = 6;
const Z_BTN: i32 = 7;

const BTN_W: f32 = 129.0;
const BTN_H: f32 = 32.0;
/// 四状态横排间距(原作 clip_a/c/c_d: 135px 一档)
const STATE_W: f32 = 135.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    Save,
    Load,
    Config,
    Backlog,
    Skip,
    Title,
    Close,
}

struct BtnSpec {
    action: MenuAction,
    x: f32,
    y: f32,
    row: f32,
}

const MENU_BUTTONS: [BtnSpec; 6] = [
    BtnSpec { action: MenuAction::Save,    x: 430.0, y: 296.0, row: 0.0 },
    BtnSpec { action: MenuAction::Load,    x: 430.0, y: 336.0, row: 32.0 },
    BtnSpec { action: MenuAction::Config,  x: 430.0, y: 376.0, row: 63.0 },
    BtnSpec { action: MenuAction::Backlog, x: 580.0, y: 296.0, row: 96.0 },
    BtnSpec { action: MenuAction::Skip,    x: 581.0, y: 336.0, row: 128.0 },
    BtnSpec { action: MenuAction::Title,   x: 581.0, y: 376.0, row: 160.0 },
];

#[derive(Component)]
struct GameplayMenuRoot;

#[derive(Component)]
struct MenuButton(MenuAction);

pub struct GameplayMenuPlugin;

impl Plugin for GameplayMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (toggle_menu_input, update_menu_ui, handle_menu_clicks, update_btn_visuals),
        );
    }
}

/// 呼出/关闭 — 原作 advkey: 右键(RCLICK=adv_menu) F2(MENU) ESC(EXIT=close_ui),
/// 以及 menu_check(): 菜单未开 && 无其他覆盖层时才呼出。
fn toggle_menu_input(
    state: Res<State<VnAppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mode: Res<SaveLoadMode>,
    settings_overlay: Res<SettingsOverlayMode>,
    mut backlog: ResMut<BacklogState>,
    mut menu: ResMut<GameplayMenuMode>,
) {
    if *state.get() != VnAppState::Gameplay {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) || mouse.just_pressed(MouseButton::Right) {
        if menu.active {
            menu.active = false;
        } else if backlog.visible {
            backlog.visible = false;
        } else if !mode.active && !settings_overlay.active {
            menu.active = true;
        }
    }
    if keys.just_pressed(KeyCode::F2) && !menu.active {
        if !mode.active && !settings_overlay.active && !backlog.visible {
            menu.active = true;
        }
    }
}

fn update_menu_ui(
    menu: Res<GameplayMenuMode>,
    q_root: Query<Entity, With<GameplayMenuRoot>>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    if menu.active && q_root.is_empty() {
        spawn_menu(&mut commands, &server);
    } else if !menu.active && !q_root.is_empty() {
        for e in q_root.iter() {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_menu(commands: &mut Commands, server: &AssetServer) {
    let bg = server.load::<Image>(BG_PATH);
    let btn = server.load::<Image>(BTN_PATH);
    let btnsys = server.load::<Image>(BTNSYS_PATH);

    commands
        .spawn((
            GameplayMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ZIndex(Z_FIL),
        ))
        .with_children(|container| {
            // 半透明黑遮罩 — 原作 init.black (bg/black.png) alpha=128
            container
                .spawn((
                    Node {
                        width: Val::Px(960.0),
                        height: Val::Px(540.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|root| {
                    root.spawn((
                        ImageNode {
                            image: server.load::<Image>(BLACK_PATH),
                            color: Color::srgba(1.0, 1.0, 1.0, 128.0 / 255.0),
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
                        ZIndex(Z_BG - 1),
                    ));
                    root.spawn((
                        ImageNode {
                            image: bg,
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

                    for spec in MENU_BUTTONS {
                        spawn_menu_button(root, &btn, spec);
                    }

                    // btn07 返回 — 原作 exec=close_ui, 位置 648,440
                    root.spawn((
                        MenuButton(MenuAction::Close),
                        Button,
                        ImageNode {
                            image: btnsys,
                            rect: Some(Rect::new(680.0, 0.0, 815.0, 32.0)),
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(648.0),
                            top: Val::Px(440.0),
                            width: Val::Px(135.0),
                            height: Val::Px(32.0),
                            ..default()
                        },
                        ZIndex(Z_BTN),
                    ));
                });
        });
}

fn spawn_menu_button(root: &mut ChildSpawnerCommands, btn: &Handle<Image>, spec: BtnSpec) {
    root.spawn((
        MenuButton(spec.action),
        Button,
        ImageNode {
            image: btn.clone(),
            rect: Some(Rect::new(0.0, spec.row, BTN_W, spec.row + BTN_H)),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(spec.x),
            top: Val::Px(spec.y),
            width: Val::Px(BTN_W),
            height: Val::Px(BTN_H),
            ..default()
        },
        ZIndex(Z_BTN),
    ));
}

fn handle_menu_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    q_root: Query<Entity, With<GameplayMenuRoot>>,
    q_btn: Query<(&MenuButton, &Interaction)>,
    mut menu: ResMut<GameplayMenuMode>,
    mut mode: ResMut<SaveLoadMode>,
    mut settings_overlay: ResMut<SettingsOverlayMode>,
    mut skip: ResMut<SkipMode>,
    mut backlog: ResMut<BacklogState>,
    mut next_app: ResMut<NextState<VnAppState>>,
    mut se: MessageWriter<PlaySeEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if !menu.active || q_root.is_empty() {
        return;
    }
    for (btn, inter) in q_btn.iter() {
        if *inter != Interaction::Pressed {
            continue;
        }
        let _ = se.write(PlaySeEvent {
            file: "system_cancel".to_string(),
            channel: Some(0),
            volume: None,
        });
        match btn.0 {
            MenuAction::Save | MenuAction::Load => {
                let kind = if btn.0 == MenuAction::Save {
                    SaveLoadKind::Save
                } else {
                    SaveLoadKind::Load
                };
                menu.active = false;
                *mode = SaveLoadMode {
                    active: true,
                    kind,
                    return_to: SaveLoadReturn::Gameplay,
                };
            }
            MenuAction::Config => {
                menu.active = false;
                settings_overlay.active = true;
            }
            MenuAction::Backlog => {
                menu.active = false;
                backlog.visible = true;
            }
            MenuAction::Skip => {
                skip.active = !skip.active;
            }
            MenuAction::Title => {
                menu.active = false;
                next_app.set(VnAppState::Title);
            }
            MenuAction::Close => {
                menu.active = false;
            }
        }
        break;
    }
}

fn update_btn_visuals(
    mut q_btn: Query<(&MenuButton, &Interaction, &mut ImageNode)>,
    skip: Res<SkipMode>,
) {
    for (btn, inter, mut img) in q_btn.iter_mut() {
        if btn.0 == MenuAction::Close {
            continue;
        }
        let row = MENU_BUTTONS
            .iter()
            .find(|s| s.action == btn.0)
            .map(|s| s.row)
            .unwrap_or(0.0);
        let skip_on = btn.0 == MenuAction::Skip && skip.active;
        let state = if skip_on || *inter == Interaction::Pressed {
            2.0
        } else if *inter == Interaction::Hovered {
            1.0
        } else {
            0.0
        };
        img.rect = Some(Rect::new(
            state * STATE_W,
            row,
            state * STATE_W + BTN_W,
            row + BTN_H,
        ));
    }
}
