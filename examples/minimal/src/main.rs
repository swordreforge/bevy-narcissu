//! Minimal bevy-vn-engine example — 水仙10周年 real scripts.
//!
//! Loads every `*.vnscript.ron` from assets/scripts into the ScriptEngine
//! at startup, starts at `gamestart`, and lets ScriptRunner drive execution.
//! Space/click to advance; Wait commands auto-advance via the engine's WaitTimer.

use std::path::Path;

use bevy::prelude::*;
use bevy_vn_core::prelude::*;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_audio::VnAudioPlugin;
use bevy_vn_render::{AssetPathProvider, VnRenderPlugin};
use bevy_vn_save::VnSavePlugin;
use bevy_vn_ui::VnUiPlugin;
use bevy_vn_video::VnVideoPlugin;

const SCRIPT_DIR: &str = "assets/scripts";
const FONT_PATH: &str = "fonts/font-2.otf";
const ENTRY_SCRIPT: &str = "gamestart";
const ENTRY_LABEL: &str = "top";

#[derive(Resource)]
struct GameFont(Handle<Font>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "水仙10周年".into(),
                resolution: [1280, 720].into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VnCorePlugin {
            config: VnEngineConfig {
                default_font: FONT_PATH.into(),
                ..default()
            },
        })
        .add_plugins(VnRenderPlugin::default())
        .add_plugins(VnAudioPlugin)
        .add_plugins(VnUiPlugin)
        .add_plugins(VnSavePlugin::default())
        .add_plugins(VnVideoPlugin)
        .insert_resource(AssetPathProvider::default())
        .add_systems(Startup, (spawn_camera, load_font, load_scripts))
        .add_systems(Update, (user_input, apply_font))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn load_font(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(GameFont(asset_server.load::<Font>(FONT_PATH)));
}

/// Load every `*.vnscript.ron` from assets/scripts into the ScriptEngine,
/// then start at the `gamestart` script's `top` label.
fn load_scripts(mut engine: ResMut<ScriptEngine>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(SCRIPT_DIR);
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(Result::ok).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e| e.file_name());

    let mut loaded = 0usize;
    for entry in entries {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().into_owned();
        if !fname.ends_with(".vnscript.ron") {
            continue;
        }
        let stem = fname.trim_end_matches(".vnscript.ron").to_string();
        if stem == "pack" {
            // 宣传脚本：引用的资源几乎全部缺失，跳过
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                warn!("skip {fname}: read error {e}");
                continue;
            }
        };
        let script: VnScript = match ron::de::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                warn!("skip {fname}: parse error {e}");
                continue;
            }
        };
        let key = script.meta.name.clone().unwrap_or(stem);
        engine.load_script(key, script);
        loaded += 1;
    }
    info!("Loaded {loaded} scripts from {SCRIPT_DIR}");

    engine
        .set_current(ENTRY_SCRIPT, Some(ENTRY_LABEL))
        .unwrap_or_else(|e| error!("start failed: {e}"));
}

/// Bevy's default font has no CJK glyphs — force every UI TextFont to
/// use font-2.otf so Japanese text renders.
fn apply_font(font: Res<GameFont>, mut q: Query<&mut TextFont>) {
    for mut tf in &mut q {
        let handle = FontSource::Handle(font.0.clone());
        if tf.font != handle {
            tf.font = handle;
        }
    }
}

/// Fires AdvanceEvent on Space — mouse clicks are handled by HotspotPlugin
/// (which honors script-declared hotspot regions).
fn user_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<AdvanceEvent>,
    engine: Res<ScriptEngine>,
) {
    if !engine.has_more() { return; }
    if keys.just_pressed(KeyCode::Space) {
        writer.write(AdvanceEvent { source: AdvanceSource::UserInput });
    }
}
