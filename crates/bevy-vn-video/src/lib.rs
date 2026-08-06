//! bevy-vn-video — Video playback plugin for the Bevy VN engine.
//!
//! Consumes PlayMovieEvent / StopMovieEvent / SpriteVideoEvent / StopSpriteVideoEvent
//! from bevy-vn-core. Platform backends:
//! - Desktop: GStreamer (optional feature `gstreamer`)
//! - Android: FFmpeg (optional feature `ffmpeg`)
//!
//! On EOS, sends AdvanceEvent to resume script execution.

use bevy::prelude::*;
use bevy_vn_core::messages::{
    AdvanceEvent, AdvanceSource, PlayMovieEvent, SpriteVideoEvent,
    StopMovieEvent, StopSpriteVideoEvent,
};

// ── Resources ──

#[derive(Resource, Default)]
pub struct PendingVideo {
    pub playing: bool,
    pub file: Option<String>,
}

#[derive(Resource, Default)]
pub struct SpriteVideoManager {
    pub active: Vec<SpriteVideoState>,
}

pub struct SpriteVideoState {
    pub id: String,
    pub file: String,
    pub x: f32,
    pub y: f32,
}

// ── Plugin ──

pub struct VnVideoPlugin;

impl Plugin for VnVideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingVideo>()
            .init_resource::<SpriteVideoManager>()
            .add_systems(Update, (
                handle_play_movie,
                handle_stop_movie,
                handle_sprite_video,
                handle_stop_sprite_video,
                check_video_eos,
            ));
    }
}

// ── Systems ──

fn handle_play_movie(
    mut reader: MessageReader<PlayMovieEvent>,
    mut pending: ResMut<PendingVideo>,
) {
    for event in reader.read() {
        // TODO: platform-specific video playback
        pending.playing = event.blocking;
        pending.file = Some(event.file.clone());
    }
}

fn handle_stop_movie(
    mut reader: MessageReader<StopMovieEvent>,
    mut pending: ResMut<PendingVideo>,
) {
    for _ in reader.read() {
        pending.playing = false;
        pending.file = None;
    }
}

fn handle_sprite_video(
    mut reader: MessageReader<SpriteVideoEvent>,
    mut mgr: ResMut<SpriteVideoManager>,
) {
    for event in reader.read() {
        // Remove existing sprite video with same id
        mgr.active.retain(|s| s.id != event.id);
        mgr.active.push(SpriteVideoState {
            id: event.id.clone(),
            file: event.file.clone(),
            x: event.x,
            y: event.y,
        });
    }
}

fn handle_stop_sprite_video(
    mut reader: MessageReader<StopSpriteVideoEvent>,
    mut mgr: ResMut<SpriteVideoManager>,
) {
    for event in reader.read() {
        mgr.active.retain(|s| s.id != event.id);
    }
}

/// Simulated EOS detection — in production, the video backend would send this.
fn check_video_eos(
    mut pending: ResMut<PendingVideo>,
    mut writer: MessageWriter<AdvanceEvent>,
) {
    if pending.playing {
        // TODO: check actual video EOS from GStreamer/FFmpeg
        // For now, treat as immediate completion
        pending.playing = false;
        pending.file = None;
        writer.write(AdvanceEvent { source: AdvanceSource::VideoEnd });
    }
}
