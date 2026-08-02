//! Global volume control — bridges SetVolumeEvent across all audio subsystems.
//!
//! The BGM/SE/Voice managers each consume SetVolumeEvent independently
//! for their own volume fields. This module exists as a central registration
//! point and could be extended for master volume.

use bevy::prelude::*;
use bevy_vn_core::messages::SetVolumeEvent;

#[derive(Resource, Default)]
pub struct MasterVolume {
    pub master: f32,
    pub bgm: f32,
    pub se: f32,
    pub voice: f32,
}

pub struct VolumePlugin;
impl Plugin for VolumePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MasterVolume>()
            .add_systems(Update, handle_set_volume);
    }
}

fn handle_set_volume(
    mut reader: MessageReader<SetVolumeEvent>,
    mut master: ResMut<MasterVolume>,
) {
    for event in reader.read() {
        if let Some(v) = event.bgm { master.bgm = v; }
        if let Some(v) = event.se { master.se = v; }
        if let Some(v) = event.voice { master.voice = v; }
    }
}
