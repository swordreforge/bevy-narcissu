use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlaySeEvent, SetVolumeEvent, StopSeEvent};
use bevy_vn_core::runner::flush_audio;

use crate::channel::audio_channel_impl;
use crate::voice::VoicePreloadQueue;

const MAX_CHANNELS: usize = 8;

audio_channel_impl! {
    pub struct SeManager;
    channels: MAX_CHANNELS,
    path: "audio/se/",
    mode: PlaybackMode::Despawn,
    play: PlaySeEvent,
    file: |event: &PlaySeEvent| event.file.clone(),
    slot: |event: &PlaySeEvent| event.channel.unwrap_or(0) % MAX_CHANNELS,
    volume: |event: &SetVolumeEvent| event.se,
    stop: StopSeEvent, stop_slot: |event: &StopSeEvent| event.channel.map(|c| c % MAX_CHANNELS),
    handle: |_: &PlaySeEvent, _: &Option<Res<VoicePreloadQueue>>, server: &AssetServer, path: String| -> Handle<AudioSource> {
        server.load::<AudioSource>(path)
    },
}

pub struct SePlugin;
impl Plugin for SePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SeManager>()
            .add_systems(Update, (handle_play, handle_stop, handle_volume).after(flush_audio));
    }
}
