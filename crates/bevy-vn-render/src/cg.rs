//! CG / event image display.

use bevy::prelude::*;
use bevy_vn_core::messages::{HideCgEvent, ShowCgEvent};
use crate::AssetPathProvider;

#[derive(Component)] pub struct CgMarker;

#[derive(Resource, Default)]
pub struct CgState { pub entity: Option<Entity>, pub current: Option<String> }

pub struct CgPlugin;
impl Plugin for CgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CgState>()
            .add_systems(Update, (handle_show_cg, handle_hide_cg));
    }
}

fn handle_show_cg(
    mut reader: MessageReader<ShowCgEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    mut state: ResMut<CgState>,
    mut q: Query<(&mut ImageNode, &mut Node), With<CgMarker>>,
) {
    for event in reader.read() {
        let path = provider.as_ref().map(|p| p.cg(&event.image))
            .unwrap_or_else(|| format!("image/ev/{}.png", event.image));
        let handle = asset_server.load::<Image>(&path);
        if let Some(e) = state.entity {
            if let Ok((mut img, mut node)) = q.get_mut(e) { img.image = handle; node.display = Display::Flex; }
        } else {
            let e = commands.spawn((
                CgMarker,
                ImageNode { image: handle, ..default() },
                Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
                ZIndex(2),
            )).id();
            state.entity = Some(e);
        }
        state.current = Some(event.image.clone());
    }
}

fn handle_hide_cg(
    mut reader: MessageReader<HideCgEvent>,
    state: Res<CgState>,
    mut q: Query<&mut Node, With<CgMarker>>,
) {
    if reader.is_empty() { return; }
    reader.clear();
    if let Some(e) = state.entity {
        if let Ok(mut node) = q.get_mut(e) { node.display = Display::None; }
    }
}
