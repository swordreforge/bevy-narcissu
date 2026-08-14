//! bevy-vn-audio — Audio plugin for the Bevy VN engine.
//!
//! Consumes audio Message types from bevy-vn-core:
//! - PlayBgmEvent / StopBgmEvent  → BGM manager
//! - PlaySeEvent / StopSeEvent    → SE manager
//! - PlayVoiceEvent               → Voice manager
//! - SetVolumeEvent               → Global volume

pub mod bgm;
mod channel;
pub mod se;
pub mod voice;
pub mod volume;

use bevy::prelude::*;
use bevy_opus_audio::OpusAudioPlugin;
use bgm::BgmPlugin;
use se::SePlugin;
use voice::VoicePlugin;
use volume::VolumePlugin;

/// Audio plugin. Must be added AFTER VnCorePlugin.
pub struct VnAudioPlugin;

impl Plugin for VnAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((OpusAudioPlugin, BgmPlugin, SePlugin, VoicePlugin, VolumePlugin));
    }
}
