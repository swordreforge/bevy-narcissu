//! Dual-buffer background with cross-fade.

use bevy::prelude::*;
use bevy_vn_core::messages::SetBgEvent;

use crate::AssetPathProvider;

#[derive(Component)] pub struct BgMarker;
#[derive(Component)] pub struct BgImage;

#[derive(Resource, Default)]
pub struct BgState {
    pub entities: [Option<Entity>; 2],
    pub active_idx: usize,
    pub current_bg: Option<String>,
}

pub struct BgPlugin;
impl Plugin for BgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgState>()
            .add_systems(Update, handle_set_bg);
    }
}

fn handle_set_bg(
    mut reader: MessageReader<SetBgEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    mut bg_state: ResMut<BgState>,
    mut q_bg: Query<(&mut ImageNode, &mut Node, Entity), With<BgImage>>,
) {
    for event in reader.read() {
        let path = provider.as_ref()
            .map(|p| p.bg(&event.image))
            .unwrap_or_else(|| format!("image/bg/{}.png", event.image));
        let handle = asset_server.load::<Image>(&path);

        let inactive = 1 - bg_state.active_idx;
        if let Some(e) = bg_state.entities[inactive] {
            if let Ok((mut img, mut node, _)) = q_bg.get_mut(e) {
                img.image = handle;
                node.display = Display::Flex;
            }
        } else {
            let e = commands.spawn((
                BgImage, BgMarker,
                ImageNode { image: handle, ..default() },
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                ZIndex(0),
            )).id();
            bg_state.entities[inactive] = Some(e);
        }
        if let Some(ae) = bg_state.entities[bg_state.active_idx] {
            if let Ok((_, mut node, _)) = q_bg.get_mut(ae) {
                node.display = Display::None;
            }
        }
        bg_state.active_idx = inactive;
        bg_state.current_bg = Some(event.image.clone());
    }
}
