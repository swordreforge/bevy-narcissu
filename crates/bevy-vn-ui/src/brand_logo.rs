//! Brand logo opening sequence — faithful to the original Narcissu 10th
//! Anniversary `brandlogo` script: black frame, then logo frames fading
//! in/out, skippable by any key.

use bevy::prelude::*;
use bevy_vn_core::state::VnAppState;

const FRAMES: [&str; 4] = ["pa/logo/logo-0.png", "pa/logo/logo-1.png", "pa/logo/logo-2.png", "pa/logo/logo-3.png"];
const FRAME_MS: [u64; 4] = [1500, 1500, 1500, 1500];
const FADE_IN: f32 = 0.4;
const FADE_OUT: f32 = 0.4;

#[derive(Component)]
struct BrandLogoScreen;

#[derive(Component)]
struct LogoFrame { index: usize }

#[derive(Resource)]
struct LogoState { index: usize, timer: f32, alpha: f32, done: bool, outro: bool }

impl Default for LogoState {
    fn default() -> Self {
        Self { index: 0, timer: 0.0, alpha: 0.0, done: false, outro: false }
    }
}

pub struct BrandLogoPlugin;
impl Plugin for BrandLogoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogoState>()
            .add_systems(OnEnter(VnAppState::Splash), spawn_brand_logo)
            .add_systems(Update, advance_logo.run_if(in_state(VnAppState::Splash)))
            .add_systems(Update, handle_logo_skip.run_if(in_state(VnAppState::Splash)))
            .add_systems(OnExit(VnAppState::Splash), despawn_brand_logo);
    }
}

fn spawn_brand_logo(mut commands: Commands, asset_server: Res<AssetServer>, mut state: ResMut<LogoState>) {
    *state = LogoState::default();

    commands.spawn((
        BrandLogoScreen,
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::BLACK),
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
            for (i, path) in FRAMES.iter().enumerate() {
                canvas.spawn((
                    LogoFrame { index: i },
                    ImageNode {
                        image: asset_server.load::<Image>(*path),
                        image_mode: NodeImageMode::Stretch,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(960.0),
                        height: Val::Px(540.0),
                        display: if i == 0 { Display::Flex } else { Display::None },
                        ..default()
                    },
                    ZIndex(1),
                ));
            }
        });
    });
}

fn advance_logo(
    time: Res<Time>,
    mut state: ResMut<LogoState>,
    mut q: Query<(&LogoFrame, &mut Node, &mut ImageNode)>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    if state.done { return; }
    let dt = time.delta_secs();

    if state.outro {
        state.alpha = (state.alpha - dt / FADE_OUT).max(0.0);
        for (_, _, mut img) in q.iter_mut() {
            img.color.set_alpha(state.alpha);
        }
        if state.alpha <= 0.0 {
            state.done = true;
            next.set(VnAppState::Title);
        }
        return;
    }

    if state.alpha < 1.0 {
        state.alpha = (state.alpha + dt / FADE_IN).min(1.0);
        for (_, _, mut img) in q.iter_mut() {
            img.color.set_alpha(state.alpha);
        }
        return;
    }

    state.timer += dt;
    let frame_ms = FRAME_MS[state.index] as f32 / 1000.0;
    if state.timer < frame_ms { return; }
    state.timer = 0.0;

    if state.index + 1 >= FRAMES.len() {
        state.outro = true;
        return;
    }
    state.index += 1;
    for (frame, mut node, mut img) in q.iter_mut() {
        if frame.index == state.index {
            node.display = Display::Flex;
            img.color.set_alpha(0.0);
        } else {
            node.display = Display::None;
        }
    }
    state.alpha = 0.0;
}

fn handle_logo_skip(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
        || mouse.just_pressed(MouseButton::Left)
    {
        next.set(VnAppState::Title);
    }
}

fn despawn_brand_logo(mut commands: Commands, q: Query<Entity, With<BrandLogoScreen>>) {
    for e in q.iter() { commands.entity(e).despawn(); }
}
