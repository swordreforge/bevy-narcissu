//! Minimal bevy-vn-engine example — 水仙10周年 real scripts.
//!
//! Flow: Splash (brand logo) → Title (image-based) → RouteSelect (story pick)
//! → Gameplay (script-driven). Loads every `*.vnscript.ron` from assets/scripts
//! into the ScriptEngine at startup; scripts start only after a story is chosen.

use std::path::Path;

use bevy::prelude::*;
use bevy_vn_core::prelude::*;
use bevy_vn_core::runner::ScriptBlock;
use bevy_vn_core::script::ScriptEngine;
use bevy_vn_core::state::{
    GameplayMenuMode, SaveLoadMode, SettingsOverlayMode, VnAppState,
};
use bevy_vn_audio::VnAudioPlugin;
use bevy_vn_render::{AssetPathProvider, VnRenderPlugin};
use bevy_vn_save::VnSavePlugin;
use bevy_vn_ui::backlog::BacklogState;
use bevy_vn_ui::VnUiPlugin;
use bevy_vn_video::VnVideoPlugin;

const SCRIPT_DIR: &str = "assets/scripts";
const FONT_PATH: &str = "fonts/font-2.otf";

#[derive(Resource)]
struct GameFont(Handle<Font>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "水仙10周年".into(),
                resolution: [960, 540].into(),
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
        .add_systems(Startup, (spawn_camera, load_font, load_scripts, start_at_splash))
        .add_systems(OnEnter(VnAppState::Title), play_title_bgm)
        .add_systems(OnExit(VnAppState::Title), stop_title_bgm)
        .add_systems(Update, (user_input, apply_font, handle_story_select, handle_custom_tag, return_to_title_on_story_end))
        .run();
}

/// Title BGM — the original game plays `bgm01` (disc1/01-Scarlet -arranged)
/// on the title screen (list_windows.tbl `titlebgm_title={"bgm01","bgm22"}`,
/// normal version takes index 1). Only active in Title state.
fn play_title_bgm(mut writer: MessageWriter<PlayBgmEvent>) {
    writer.write(PlayBgmEvent { id: "1".into(), volume: None, fade_ms: None });
}

/// Stop the title BGM as soon as we leave the title screen.
fn stop_title_bgm(mut writer: MessageWriter<StopBgmEvent>) {
    writer.write(StopBgmEvent { fade_ms: None });
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn load_font(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(GameFont(asset_server.load::<Font>(FONT_PATH)));
}

/// Begin the opening sequence: brand logo first.
fn start_at_splash(mut next: ResMut<NextState<VnAppState>>) {
    next.set(VnAppState::Splash);
}

/// Load every `*.vnscript.ron` from assets/scripts into the ScriptEngine.
/// No script is started — the user picks a story from the RouteSelect UI.
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

/// Fires AdvanceEvent on Space — only while actually playing a script and
/// no overlay (save/load, system menu, settings, backlog) is open.
/// Mouse clicks on dialogue are handled by HotspotPlugin.
fn user_input(
    state: Res<State<VnAppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<AdvanceEvent>,
    engine: Res<ScriptEngine>,
    mode: Res<SaveLoadMode>,
    menu: Res<GameplayMenuMode>,
    settings_overlay: Res<SettingsOverlayMode>,
    backlog: Res<BacklogState>,
) {
    if *state.get() != VnAppState::Gameplay { return; }
    if mode.active || menu.active || settings_overlay.active || backlog.visible { return; }
    if !engine.has_more() { return; }
    if keys.just_pressed(KeyCode::Space) {
        writer.write(AdvanceEvent { source: AdvanceSource::UserInput });
    }
}

/// RouteSelect chose a story → jump the engine to that script/label, play.
fn handle_story_select(
    mut reader: MessageReader<StorySelectEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut block: ResMut<ScriptBlock>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    for e in reader.read() {
        match engine.set_current(&e.script, Some(&e.label)) {
            Ok(()) => {
                block.blocked = false;
                next.set(VnAppState::Gameplay);
            }
            Err(err) => error!("story select {}.{} failed: {err}", e.script, e.label),
        }
    }
}

/// Gameplay script hit a custom tag. `タイトル` returns to the title screen;
/// `brandlogo` replays the opening (used by some scripts).
fn handle_custom_tag(
    mut reader: MessageReader<CustomTagEvent>,
    mut block: ResMut<ScriptBlock>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    for e in reader.read() {
        match e.tag.as_str() {
            "タイトル" => {
                block.blocked = true;
                next.set(VnAppState::Title);
            }
            "brandlogo" => {
                next.set(VnAppState::Splash);
            }
            _ => {}
        }
    }
}

/// Script reached its end — either a `halt` command (`ScriptEngine.finished`)
/// or the last instruction of a script entered directly from chapter select
/// (`has_more()` is false because `current()` is past the end). No more
/// `AdvanceEvent` will be produced, so leave Gameplay and return to the
/// title screen — matching the original game, where each of the six
/// short stories ends back at the title.
fn return_to_title_on_story_end(
    state: Res<State<VnAppState>>,
    engine: Res<ScriptEngine>,
    mut block: ResMut<ScriptBlock>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    if *state.get() != VnAppState::Gameplay { return; }
    if engine.has_more() { return; }
    block.blocked = true;
    next.set(VnAppState::Title);
}
