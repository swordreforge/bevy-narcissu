use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlayBgmEvent, StopBgmEvent, SetVolumeEvent};

#[derive(Resource, Default)]
pub struct BgmManager {
    pub entity: Option<Entity>,
    pub current_id: Option<String>,
    pub volume: f32,
}

pub struct BgmPlugin;
impl Plugin for BgmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgmManager>()
            .add_systems(Update, (handle_play_bgm, handle_stop_bgm, handle_bgm_volume));
    }
}

fn handle_play_bgm(
    mut reader: MessageReader<PlayBgmEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mgr: ResMut<BgmManager>,
) {
    for event in reader.read() {
        if let Some(e) = mgr.entity { commands.entity(e).despawn(); }
        let path = format!("audio/bgm/{}.ogg", event.id);
        let vol = event.volume.unwrap_or(mgr.volume.max(0.01));
        mgr.entity = Some(commands.spawn((
            AudioPlayer(asset_server.load::<AudioSource>(&path)),
            PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(vol), ..default() },
        )).id());
        mgr.current_id = Some(event.id.clone());
    }
}

fn handle_stop_bgm(
    mut reader: MessageReader<StopBgmEvent>,
    mut commands: Commands,
    mut mgr: ResMut<BgmManager>,
) {
    for _ in reader.read() {
        if let Some(e) = mgr.entity.take() { commands.entity(e).despawn(); }
        mgr.current_id = None;
    }
}

fn handle_bgm_volume(
    mut reader: MessageReader<SetVolumeEvent>,
    mut mgr: ResMut<BgmManager>,
    mut q_sink: Query<&mut AudioSink>,
) {
    for event in reader.read() {
        if let Some(vol) = event.bgm {
            mgr.volume = vol;
            if let Some(e) = mgr.entity {
                if let Ok(mut sink) = q_sink.get_mut(e) { sink.set_volume(Volume::Linear(vol)); }
            }
        }
    }
}
