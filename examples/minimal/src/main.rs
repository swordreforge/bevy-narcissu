//! Minimal bevy-vn-engine example — 水仙10周年 real scripts.
//!
//! Flow: Splash (brand logo) → Title (image-based) → RouteSelect (story pick)
//! → Gameplay (script-driven). Scripts load lazily via `AssetServer::load_folder`
//! once the app starts; a script only runs after a story is chosen.

use bevy::prelude::*;
use bevy_basisu_loader::BasisuLoaderPlugin;
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

#[cfg(not(target_arch = "wasm32"))]
use bevy_vn_video::VnVideoPlugin;

const FONT_PATH: &str = "fonts/font-2.otf";
const SCRIPT_MANIFEST: &str = "scripts/manifest.list";

#[derive(Resource)]
struct GameFont(Handle<Font>);

#[derive(Resource)]
struct ScriptManifestHandle(Handle<ScriptManifest>);

/// Scripts listed in the manifest, loaded in small per-frame batches so
/// wasm never fires hundreds of HTTP requests in a single frame (browser
/// connection pool / `ERR_INSUFFICIENT_RESOURCES`).
#[derive(Resource, Default)]
struct ScriptLoadQueue {
    pending: Vec<String>,
    handles: Vec<Handle<VnScriptAsset>>,
}

const SCRIPT_LOADS_PER_FRAME: usize = 16;

fn main() {
    let mut app = App::new();

    #[cfg(target_arch = "wasm32")]
    {
        use bevy::asset::AssetMetaCheck;
        use bevy::log::LogPlugin;
        use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
        use bevy::render::RenderPlugin;
        app.add_plugins(DefaultPlugins
            .set(LogPlugin {
                level: bevy::log::Level::ERROR,
                ..default()
            })
            .set(AssetPlugin {
                // wasm 上跳过 .meta 查找:避免每个资产产生 404 请求;
                // 且 itch.io 等平台缺失文件返回 403 会被 Bevy 视为致命错误,
                // 直接导致整个资产加载失败。原生保留默认 Always。
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(RenderPlugin {
                // wasm 双后端:WebGPU 优先,浏览器无 WebGPU(如 Firefox Linux)时
                // 自动 fallback 到 WebGL2(wgpu-core)。Bevy 默认只给 BROWSER_WEBGPU,
                // 必须显式包含 GL,否则 fallback 后找不到 WebGL2 适配器。
                // priority 保持默认 Functionality:bevy 会取 adapter.limits()。
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(Backends::BROWSER_WEBGPU | Backends::GL),
                    ..default()
                })),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "水仙10周年".into(),
                    resolution: [960, 540].into(),
                    ..default()
                }),
                ..default()
            }));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "水仙10周年".into(),
                resolution: [960, 540].into(),
                ..default()
            }),
            ..default()
        }));
    }

    app.add_plugins(BasisuLoaderPlugin)
        .add_plugins(VnCorePlugin {
            config: VnEngineConfig {
                default_font: FONT_PATH.into(),
                ..default()
            },
        })
        .add_plugins(VnRenderPlugin::default())
        .add_plugins(VnAudioPlugin)
        .add_plugins(VnUiPlugin)
        .add_plugins(VnSavePlugin::default());

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(VnVideoPlugin);

    app.insert_resource(AssetPathProvider::default())
        .add_systems(Startup, (spawn_camera, load_font, load_scripts, start_at_splash))
        .add_systems(OnEnter(VnAppState::Title), play_title_bgm)
        .add_systems(OnExit(VnAppState::Title), stop_title_bgm)
        .add_systems(Update, (user_input, apply_font, handle_story_select, handle_custom_tag, return_to_title_on_story_end, request_scripts_from_manifest, drive_script_loads, ingest_scripts))
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

/// Kick off lazy loading of every script listed in `scripts/manifest.list`.
fn load_scripts(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(ScriptManifestHandle(asset_server.load(SCRIPT_MANIFEST)));
}

/// Once the manifest is loaded, seed the load queue with every listed script.
fn request_scripts_from_manifest(
    manifest: Res<ScriptManifestHandle>,
    manifests: Res<Assets<ScriptManifest>>,
    mut commands: Commands,
    mut started: Local<bool>,
) {
    if *started { return; }
    let Some(manifest_asset) = manifests.get(&manifest.0) else { return; };
    commands.insert_resource(ScriptLoadQueue {
        pending: manifest_asset.files.clone(),
        handles: Vec::new(),
    });
    info!("Script manifest loaded: {} files", manifest_asset.files.len());
    *started = true;
}

/// Issue script loads in small batches; `ingest_scripts` waits until every
/// batch has been requested and loaded.
fn drive_script_loads(
    queue: Option<ResMut<ScriptLoadQueue>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut queue) = queue else { return };
    let mut issued = 0;
    while issued < SCRIPT_LOADS_PER_FRAME {
        let Some(file) = queue.pending.pop() else { break };
        queue
            .handles
            .push(asset_server.load::<VnScriptAsset>(format!("scripts/{file}")));
        issued += 1;
    }
}

/// Once every listed script has finished loading, ingest them into the
/// ScriptEngine. Runs exactly once. A script is only *played* after the user
/// picks a story from the RouteSelect UI.
fn ingest_scripts(
    queue: Option<Res<ScriptLoadQueue>>,
    scripts: Res<Assets<VnScriptAsset>>,
    mut engine: ResMut<ScriptEngine>,
    mut done: Local<bool>,
) {
    if *done { return; }
    let Some(queue) = queue else { return; };
    if !queue.pending.is_empty() { return; }
    if queue.handles.iter().any(|h| scripts.get(h).is_none()) { return; }

    let mut loaded = 0usize;
    for handle in &queue.handles {
        let Some(asset) = scripts.get(handle) else { continue; };
        let Some(name) = asset.script.meta.name.clone() else {
            continue;
        };
        if name == "pack" {
            // 宣传脚本：引用的资源几乎全部缺失，跳过
            continue;
        }
        engine.load_script(name, asset.script.clone());
        loaded += 1;
    }
    info!("Loaded {loaded} scripts from manifest");
    *done = true;
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
