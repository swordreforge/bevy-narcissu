use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlaySeEvent, StopSeEvent, SetVolumeEvent};
use bevy_vn_core::runner::flush_audio;

const MAX_CHANNELS: usize = 8;

#[derive(Resource)]
pub struct SeManager {
    pub entities: [Option<Entity>; MAX_CHANNELS],
    pub volume: f32,
}

impl Default for SeManager {
    fn default() -> Self {
        Self { entities: [None; MAX_CHANNELS], volume: 1.0 }
    }
}

pub struct SePlugin;
impl Plugin for SePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SeManager>()
            .add_systems(Update, (handle_play_se, handle_stop_se, handle_se_volume).after(flush_audio));
    }
}

fn handle_play_se(
    mut reader: MessageReader<PlaySeEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mgr: ResMut<SeManager>,
) {
    for event in reader.read() {
        let ch = event.channel.unwrap_or(0) % MAX_CHANNELS;
        if let Some(e) = mgr.entities[ch] { commands.entity(e).try_despawn(); }
        let path = format!("audio/se/{}.ogg", event.file);
        let vol = event.volume.unwrap_or(mgr.volume.max(0.01));
        mgr.entities[ch] = Some(commands.spawn((
            AudioPlayer(asset_server.load::<AudioSource>(&path)),
            PlaybackSettings { mode: PlaybackMode::Despawn, volume: Volume::Linear(vol), ..default() },
        )).id());
    }
}

fn handle_stop_se(
    mut reader: MessageReader<StopSeEvent>,
    mut commands: Commands,
    mut mgr: ResMut<SeManager>,
) {
    for event in reader.read() {
        if let Some(ch) = event.channel {
            let ch = ch % MAX_CHANNELS;
            if let Some(e) = mgr.entities[ch].take() { commands.entity(e).try_despawn(); }
        } else {
            for slot in mgr.entities.iter_mut() {
                if let Some(e) = slot.take() { commands.entity(e).try_despawn(); }
            }
        }
    }
}

fn handle_se_volume(
    mut reader: MessageReader<SetVolumeEvent>,
    mut mgr: ResMut<SeManager>,
    mut q_sink: Query<&mut AudioSink>,
) {
    for event in reader.read() {
        if let Some(vol) = event.se {
            mgr.volume = vol;
            for slot in mgr.entities.iter().flatten() {
                if let Ok(mut sink) = q_sink.get_mut(*slot) { sink.set_volume(Volume::Linear(vol)); }
            }
        }
    }
}
