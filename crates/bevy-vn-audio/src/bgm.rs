use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlayBgmEvent, SetVolumeEvent, StopBgmEvent};
use bevy_vn_core::runner::flush_audio;

use crate::channel::audio_channel_impl;
use crate::opus::OpusAudio;
use crate::voice::VoicePreloadQueue;

audio_channel_impl! {
    pub struct BgmManager;
    channels: 1,
    path: "audio/bgm/",
    mode: PlaybackMode::Loop,
    play: PlayBgmEvent,
    file: |event: &PlayBgmEvent| event.id.clone(),
    slot: |_: &PlayBgmEvent| 0,
    volume: |event: &SetVolumeEvent| event.bgm,
    track: |event: &PlayBgmEvent| Some(event.id.clone()),
    stop: StopBgmEvent, stop_slot: |_: &StopBgmEvent| -> Option<usize> { None },
    asset: OpusAudio,
    handle: |_: &PlayBgmEvent, _: &Option<Res<VoicePreloadQueue>>, server: &AssetServer, path: String| -> Handle<OpusAudio> {
        server.load::<OpusAudio>(path)
    },
}

pub struct BgmPlugin;
impl Plugin for BgmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgmManager>()
            .add_systems(Update, (handle_play, handle_stop, handle_volume).after(flush_audio));
    }
}
