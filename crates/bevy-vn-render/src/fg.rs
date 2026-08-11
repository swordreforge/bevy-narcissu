//! Foreground character sprite slots.

use bevy::prelude::*;

use bevy_vn_core::messages::{HideFgEvent, ShowFaceEvent, ShowFgEvent};
use bevy_vn_core::script::cmd::FgPosition;
use crate::AssetPathProvider;

#[derive(Component)] pub struct FgSlotMarker(pub usize);
#[derive(Component)] pub struct FgChar { pub char_id: String }

#[derive(Resource)]
pub struct FgSlotConfig { pub slot_count: usize }

#[derive(Resource, Default)]
pub struct FgSlotState {
    pub occupied: Vec<Option<(String, Entity)>>,
    pub initialized: bool,
}

pub struct FgPlugin;
impl Plugin for FgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FgSlotState>()
            .add_systems(Startup, init_slots)
            .add_systems(Update, (handle_show_fg, handle_hide_fg, handle_show_face));
    }
}

fn init_slots(mut commands: Commands, config: Res<FgSlotConfig>, mut state: ResMut<FgSlotState>) {
    state.occupied = vec![None; config.slot_count];
    for i in 0..config.slot_count {
        commands.spawn((
            FgSlotMarker(i),
            ImageNode { image: Handle::default(), ..default() },
            Node {
                position_type: PositionType::Absolute, bottom: Val::Px(0.0),
                width: percent(100), height: percent(100),
                display: Display::None, ..default()
            },
            ZIndex(1),
        ));
    }
    state.initialized = true;
}

fn fg_x(p: FgPosition) -> Val {
    match p {
        FgPosition::Left => Val::Percent(16.6),
        FgPosition::Center => Val::Percent(50.0),
        FgPosition::Right => Val::Percent(83.3),
        FgPosition::Custom { x } => Val::Percent(x * 100.0),
    }
}

fn slot_for_char(state: &FgSlotState, cid: &str) -> Option<usize> {
    state.occupied.iter().position(|o| o.as_ref().is_some_and(|(c, _)| c == cid))
}

fn empty_slot(state: &FgSlotState) -> Option<usize> {
    state.occupied.iter().position(|o| o.is_none())
}

fn handle_show_fg(
    mut reader: MessageReader<ShowFgEvent>,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    mut state: ResMut<FgSlotState>,
    mut q: Query<(&FgSlotMarker, &mut ImageNode, &mut Node)>,
) {
    for event in reader.read() {
        let path = AssetPathProvider::resolve(provider.as_deref(), |p| p.fg(&event.char_id, &event.expression));
        let handle = asset_server.load::<Image>(&path);

        let slot = slot_for_char(&state, &event.char_id).or_else(|| empty_slot(&state));
        let Some(idx) = slot else { warn!("no free FG slot for {}", event.char_id); continue; };

        for (marker, mut img, mut node) in q.iter_mut() {
            if marker.0 == idx {
                img.image = handle;
                node.display = Display::Flex;
                node.left = fg_x(event.position);
                state.occupied[idx] = Some((event.char_id.clone(), Entity::PLACEHOLDER));
                break;
            }
        }
    }
}

fn handle_hide_fg(
    mut reader: MessageReader<HideFgEvent>,
    mut state: ResMut<FgSlotState>,
    mut q: Query<(&FgSlotMarker, &mut ImageNode, &mut Node)>,
) {
    for event in reader.read() {
        if let Some(idx) = slot_for_char(&state, &event.char_id) {
            state.occupied[idx] = None;
            for (marker, mut img, mut node) in q.iter_mut() {
                if marker.0 == idx {
                    node.display = Display::None;
                    img.image = Handle::default();
                    break;
                }
            }
        }
    }
}

fn handle_show_face(
    mut reader: MessageReader<ShowFaceEvent>,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    state: Res<FgSlotState>,
    mut q: Query<(&FgSlotMarker, &mut ImageNode, &mut Node)>,
) {
    for event in reader.read() {
        let path = AssetPathProvider::resolve(provider.as_deref(), |p| p.fg(&event.char_id, &event.expression));
        let handle = asset_server.load::<Image>(&path);

        if let Some(idx) = slot_for_char(&state, &event.char_id) {
            for (marker, mut img, mut node) in q.iter_mut() {
                if marker.0 == idx && state.occupied[idx].is_some() {
                    img.image = handle;
                    node.display = Display::Flex;
                    break;
                }
            }
        }
    }
}
