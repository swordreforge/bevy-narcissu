//! Title screen — image-based, faithful to the original Narcissu 10th
//! Anniversary layout (960x540 logical pixels).

use bevy::prelude::*;
use bevy_vn_core::state::{VnAppState, VnMenuState};

const BG_PATH: &str = "pa/title/bg.png";
const LOGO_PATH: &str = "pa/title/logo.png";
const BTN_PATH: &str = "pa/title/btn.png";
const TYPEMOON_PATH: &str = "pa/title/TYPEMOON.png";

#[derive(Component)]
struct TitleScreen;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TitleAction { Start, Chapter, Load, Settings, Extra, Quit }

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
            .add_systems(Update, handle_title_click)
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
        // Fixed 960x540 canvas, centered in whatever window size.
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
                ImageNode {
                    image: bg,
                    image_mode: NodeImageMode::Stretch,
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
                ZIndex(0),
            ));

            canvas.spawn((
                ImageNode { image: logo, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(50.0),
                    top: Val::Px(49.0),
                    width: Val::Px(336.0),
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

fn handle_title_click(
    state: Res<State<VnAppState>>,
    q: Query<(&TitleButton, &Interaction)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<VnAppState>>,
    mut next_menu: ResMut<NextState<VnMenuState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if *state.get() != VnAppState::Title { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }

    for (btn, inter) in q.iter() {
        if *inter != Interaction::Pressed { continue; }
        match btn.0 {
            TitleAction::Start | TitleAction::Chapter => {
                next_menu.set(VnMenuState::RouteSelect);
                next.set(VnAppState::Menu);
            }
            TitleAction::Load => {
                next_menu.set(VnMenuState::SaveLoad);
                next.set(VnAppState::Menu);
            }
            TitleAction::Settings => {
                next_menu.set(VnMenuState::Settings);
                next.set(VnAppState::Menu);
            }
            TitleAction::Extra => {
                next_menu.set(VnMenuState::Gallery);
                next.set(VnAppState::Menu);
            }
            TitleAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

fn despawn_title(mut commands: Commands, q: Query<Entity, With<TitleScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
