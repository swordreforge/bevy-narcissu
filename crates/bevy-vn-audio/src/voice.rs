use std::collections::HashMap;

use bevy::prelude::*;
use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};

use bevy_vn_core::messages::{PlayVoiceEvent, SetVolumeEvent, StorySelectEvent};
use bevy_vn_core::runner::flush_audio;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_core::state::VnAppState;

use crate::channel::audio_channel_impl;

/// Bounded-concurrency voice preload queue.
///
/// A story transitively references thousands of voice files; firing them all
/// in one frame exhausts the browser connection pool on WASM
/// (`ERR_INSUFFICIENT_RESOURCES`). Loads are capped per frame / in flight,
/// strong handles are retained so preloaded voices play instantly, entries
/// stalling past [`VOICE_LOAD_TIMEOUT`] are dropped so a missing file cannot
/// stall the queue, and the queue resets on story select / exit so the
/// previous story's decoded audio is freed.
#[derive(Resource, Default)]
pub struct VoicePreloadQueue {
    pending: Vec<String>,
    loading: Vec<(String, Handle<AudioSource>, f32)>,
    loaded: HashMap<String, Handle<AudioSource>>,
}

const PRELOAD_PER_FRAME: usize = 4;
const PRELOAD_IN_FLIGHT: usize = 6;
const VOICE_LOAD_TIMEOUT: f32 = 15.0;

impl VoicePreloadQueue {
    fn reset_with(&mut self, files: Vec<String>) {
        self.pending = files;
        self.loading.clear();
        self.loaded.clear();
    }

    fn cached(&self, file: &str) -> Option<Handle<AudioSource>> {
        self.loaded
            .get(file)
            .cloned()
            .or_else(|| self.loading.iter().find(|(f, _, _)| f == file).map(|(_, h, _)| h.clone()))
    }
}

audio_channel_impl! {
    pub struct VoiceManager;
    channels: 1,
    path: "audio/voice/",
    mode: PlaybackMode::Despawn,
    play: PlayVoiceEvent,
    file: |event: &PlayVoiceEvent| event.file.clone(),
    slot: |_: &PlayVoiceEvent| 0,
    volume: |event: &SetVolumeEvent| event.voice,
    handle: |event: &PlayVoiceEvent, queue: &Option<Res<VoicePreloadQueue>>, server: &AssetServer, path: String| -> Handle<AudioSource> {
        let queue = queue.as_ref().expect("VoicePreloadQueue not initialized");
        queue.cached(&event.file).unwrap_or_else(|| server.load::<AudioSource>(path))
    },
}

pub struct VoicePlugin;
impl Plugin for VoicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoiceManager>()
            .init_resource::<VoicePreloadQueue>()
            .add_systems(
                Update,
                (
                    handle_play,
                    handle_volume,
                    queue_story_voices,
                    drive_voice_preload,
                )
                    .after(flush_audio),
            )
            .add_systems(
                OnExit(VnAppState::Gameplay),
                (stop_voice_on_exit, clear_voice_preload_on_exit),
            );
    }
}

fn stop_voice_on_exit(mut mgr: ResMut<VoiceManager>, mut commands: Commands) {
    if let Some(e) = mgr.entities[0] {
        commands.entity(e).try_despawn();
        mgr.entities[0] = None;
    }
}

fn clear_voice_preload_on_exit(mut queue: ResMut<VoicePreloadQueue>) {
    queue.reset_with(Vec::new());
}

fn queue_story_voices(
    mut reader: MessageReader<StorySelectEvent>,
    engine: Res<ScriptEngine>,
    mut queue: ResMut<VoicePreloadQueue>,
) {
    for event in reader.read() {
        let files = engine.collect_voice_files(&event.script);
        queue.reset_with(files);
    }
}

fn drive_voice_preload(
    time: Res<Time>,
    mut queue: ResMut<VoicePreloadQueue>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<AudioSource>>,
) {
    let now = time.elapsed_secs();
    let mut i = 0;
    while i < queue.loading.len() {
        let (_, handle, issued_at) = &queue.loading[i];
        if assets.get(handle).is_some() {
            let (file, handle, _) = queue.loading.swap_remove(i);
            queue.loaded.insert(file, handle);
        } else if now - *issued_at > VOICE_LOAD_TIMEOUT {
            queue.loading.swap_remove(i);
        } else {
            i += 1;
        }
    }

    let in_flight = queue.loading.len();
    if in_flight >= PRELOAD_IN_FLIGHT {
        return;
    }
    let mut issued = 0;
    while issued < PRELOAD_PER_FRAME && in_flight + issued < PRELOAD_IN_FLIGHT {
        let Some(file) = queue.pending.pop() else { break };
        let handle = asset_server.load::<AudioSource>(&format!("audio/voice/{file}.ogg"));
        queue.loading.push((file, handle, now));
        issued += 1;
    }
}
