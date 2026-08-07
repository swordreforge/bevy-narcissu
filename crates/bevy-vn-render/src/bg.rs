//! Dual-buffer background. Transitions fade to black and back in, matching
//! the original engine's bg_fade behavior (black scenes are intentional).

use bevy::prelude::*;
use bevy_vn_core::messages::SetBgEvent;
use bevy_vn_core::script::cmd::Transition;

use crate::AssetPathProvider;

#[derive(Component)] pub struct BgMarker;
#[derive(Component)] pub struct BgImage;

#[derive(Resource, Default)]
pub struct BgState {
    pub entities: [Option<Entity>; 2],
    pub active_idx: usize,
    pub current_bg: Option<String>,
    fading: Option<BgFade>,
}

struct BgFade {
    elapsed: f32,
    duration: f32,
}

pub struct BgPlugin;
impl Plugin for BgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgState>()
            .add_systems(Update, (handle_set_bg, update_bg_fade));
    }
}

fn handle_set_bg(
    mut reader: MessageReader<SetBgEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    mut bg_state: ResMut<BgState>,
    mut q_bg: Query<(&mut ImageNode, &mut Node), With<BgImage>>,
) {
    for event in reader.read() {
        let path = provider.as_ref()
            .map(|p| p.bg(&event.image))
            .unwrap_or_else(|| format!("image/bg/{}.png", event.image));
        let handle = asset_server.load::<Image>(&path);

        let inactive = 1 - bg_state.active_idx;
        match bg_state.entities[inactive] {
            Some(e) => {
                if let Ok((mut img, mut node)) = q_bg.get_mut(e) {
                    img.image = handle.clone();
                    img.color.set_alpha(0.0);
                    node.display = Display::Flex;
                }
            }
            None => {
                let e = commands.spawn((
                    BgImage, BgMarker,
                    ImageNode { image: handle.clone(), color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..default() },
                    Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                    ZIndex(0),
                )).id();
                bg_state.entities[inactive] = Some(e);
            }
        }

        match event.transition {
            Some(Transition::Fade { duration }) if duration > 0.0 => {
                bg_state.fading = Some(BgFade { elapsed: 0.0, duration });
            }
            _ => {
                if let Some(ae) = bg_state.entities[bg_state.active_idx] {
                    if let Ok((_, mut node)) = q_bg.get_mut(ae) {
                        node.display = Display::None;
                    }
                }
                if let Some(ne) = bg_state.entities[inactive] {
                    if let Ok((mut img, _)) = q_bg.get_mut(ne) {
                        img.color.set_alpha(1.0);
                    }
                }
                bg_state.active_idx = inactive;
                bg_state.fading = None;
            }
        }
        bg_state.current_bg = Some(event.image.clone());
    }
}

fn update_bg_fade(
    time: Res<Time>,
    mut bg_state: ResMut<BgState>,
    mut q_bg: Query<(&mut ImageNode, &mut Node), With<BgImage>>,
) {
    let Some(mut fade) = bg_state.fading.take() else { return };
    fade.elapsed += time.delta_secs();
    let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
    let inactive = 1 - bg_state.active_idx;

    if t < 0.5 {
        // Fade old background out to black.
        let a = 1.0 - t * 2.0;
        if let Some(e) = bg_state.entities[bg_state.active_idx] {
            if let Ok((mut img, _)) = q_bg.get_mut(e) {
                img.color.set_alpha(a);
            }
        }
        if let Some(e) = bg_state.entities[inactive] {
            if let Ok((mut img, _)) = q_bg.get_mut(e) {
                img.color.set_alpha(0.0);
            }
        }
    } else {
        // Fade the new background in from black.
        if let Some(e) = bg_state.entities[bg_state.active_idx] {
            if let Ok((_, mut node)) = q_bg.get_mut(e) {
                node.display = Display::None;
            }
        }
        let a = (t - 0.5) * 2.0;
        if let Some(e) = bg_state.entities[inactive] {
            if let Ok((mut img, _)) = q_bg.get_mut(e) {
                img.color.set_alpha(a);
            }
        }
    }

    if t >= 1.0 {
        bg_state.active_idx = inactive;
    } else {
        bg_state.fading = Some(fade);
    }
}
