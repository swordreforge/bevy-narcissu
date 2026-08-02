//! Minimal bevy-vn-engine example.
//!
//! Demonstrates loading a script and using ScriptRunner to drive execution.
//! Press Space or click to advance — ScriptRunner handles all dispatch.

use bevy::prelude::*;
use bevy_vn_core::prelude::*;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_audio::VnAudioPlugin;
use bevy_vn_render::{AssetPathProvider, VnRenderPlugin};
use bevy_vn_save::VnSavePlugin;
use bevy_vn_ui::VnUiPlugin;
use bevy_vn_video::VnVideoPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VnCorePlugin { config: VnEngineConfig::default() })
        .add_plugins(VnRenderPlugin::default())
        .add_plugins(VnAudioPlugin)
        .add_plugins(VnUiPlugin)
        .add_plugins(VnSavePlugin::default())
        .add_plugins(VnVideoPlugin)
        .insert_resource(AssetPathProvider::default())
        .add_systems(Startup, load_scripts)
        .add_systems(Update, user_input)
        .run();
}

fn load_scripts(mut engine: ResMut<ScriptEngine>) {
    info!("Loading demo script...");
    use bevy_vn_core::script::cmd::*;
    let script = VnScript {
        version: ScriptVersion::V1,
        meta: ScriptMeta { name: Some("demo".into()), next_script: None },
        instructions: vec![
            ScriptCmd::Label { name: "start".into() },
            ScriptCmd::Dialogue { speaker: Some("Narrator".into()), text: "Hello, visual novel world!".into(), voice: None },
            ScriptCmd::Wait { time_ms: 2000 },
            ScriptCmd::Dialogue { speaker: None, text: "This is a minimal bevy-vn-engine example.".into(), voice: None },
            ScriptCmd::Wait { time_ms: 2000 },
            ScriptCmd::Halt,
        ],
    };
    engine.load_script("demo".to_string(), script);
    engine.set_current("demo", Some("start")).unwrap();
    info!("Script loaded. Press Space or click to advance.");
}

/// Fires AdvanceEvent on user input — ScriptRunner handles the rest.
fn user_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: MessageWriter<AdvanceEvent>,
    engine: Res<ScriptEngine>,
) {
    if !engine.has_more() { return; }
    if keys.just_pressed(KeyCode::Space) || mouse.just_pressed(MouseButton::Left) {
        writer.write(AdvanceEvent { source: AdvanceSource::UserInput });
    }
}
