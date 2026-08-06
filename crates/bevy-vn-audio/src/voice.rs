use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlayVoiceEvent, SetVolumeEvent};

#[derive(Resource)]
pub struct VoiceManager {
    pub entity: Option<Entity>,
    pub volume: f32,
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self { entity: None, volume: 1.0 }
    }
}

pub struct VoicePlugin;
impl Plugin for VoicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoiceManager>()
            .add_systems(Update, (handle_play_voice, handle_voice_volume));
    }
}

fn handle_play_voice(
    mut reader: MessageReader<PlayVoiceEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mgr: ResMut<VoiceManager>,
) {
    for event in reader.read() {
        if let Some(e) = mgr.entity { commands.entity(e).try_despawn(); }
        let path = format!("audio/voice/{}.ogg", event.file);
        let vol = event.volume.unwrap_or(mgr.volume.max(0.01));
        bevy::log::info!("[voice] play event: file={} path={} vol={} mgr_vol={}", event.file, path, vol, mgr.volume);
        let handle = asset_server.load::<AudioSource>(&path);
        bevy::log::info!("[voice] handle state: {:?}", asset_server.get_load_state(&handle));
        mgr.entity = Some(commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings { mode: PlaybackMode::Despawn, volume: Volume::Linear(vol), ..default() },
        )).id());
    }
}

fn handle_voice_volume(
    mut reader: MessageReader<SetVolumeEvent>,
    mut mgr: ResMut<VoiceManager>,
    mut q_sink: Query<&mut AudioSink>,
) {
    for event in reader.read() {
        if let Some(vol) = event.voice {
            mgr.volume = vol;
            if let Some(e) = mgr.entity {
                if let Ok(mut sink) = q_sink.get_mut(e) { sink.set_volume(Volume::Linear(vol)); }
            }
        }
    }
}
