use bevy::app::App;
use bevy::prelude::*;
use bevy_vn_core::messages::*;
use bevy_vn_core::runner::*;
use bevy_vn_core::script::{ScriptEngine, VnScript};
use bevy_vn_core::state::{SaveLoadMode, VnAppState};

fn load_all_scripts(engine: &mut ScriptEngine, dir: &std::path::Path) -> usize {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(Result::ok).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e| e.file_name());
    let mut loaded = 0usize;
    for entry in entries {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().into_owned();
        if !fname.ends_with(".vnscript.ron") { continue; }
        let stem = fname.trim_end_matches(".vnscript.ron").to_string();
        if stem == "pack" { continue; }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok(script) = ron::de::from_str::<VnScript>(&text) else { continue };
        let key = script.meta.name.clone().unwrap_or(stem);
        engine.load_script(key, script);
        loaded += 1;
    }
    loaded
}

fn register_messages(app: &mut App) {
    app.add_message::<AdvanceEvent>();
    app.add_message::<CustomTagEvent>();
    app.add_message::<DialogueStateEvent>();
    app.add_message::<ClearDialogueEvent>();
    app.add_message::<ChoiceStateEvent>();
    app.add_message::<ChoiceSelectedEvent>();
    app.add_message::<BacklogPushEvent>();
    app.add_message::<SetBgEvent>();
    app.add_message::<SetBgDoneEvent>();
    app.add_message::<ShowFgEvent>();
    app.add_message::<HideFgEvent>();
    app.add_message::<ShowFaceEvent>();
    app.add_message::<HideFaceEvent>();
    app.add_message::<ShowCgEvent>();
    app.add_message::<HideCgEvent>();
    app.add_message::<ScrollBgEvent>();
    app.add_message::<SpriteEvent>();
    app.add_message::<SpriteEffectEvent>();
    app.add_message::<ScreenEffectEvent>();
    app.add_message::<PlayBgmEvent>();
    app.add_message::<StopBgmEvent>();
    app.add_message::<PlaySeEvent>();
    app.add_message::<StopSeEvent>();
    app.add_message::<PlayVoiceEvent>();
    app.add_message::<SetVolumeEvent>();
    app.add_message::<PlayMovieEvent>();
    app.add_message::<StopMovieEvent>();
    app.add_message::<SpriteVideoEvent>();
    app.add_message::<StopSpriteVideoEvent>();
    app.add_message::<UnlockCgEvent>();
    app.add_message::<UnlockBgmEvent>();
    app.add_message::<AffectionChangeEvent>();
    app.add_message::<SavePointEvent>();
    app.add_message::<HotspotEvent>();
    app.add_message::<HotspotClearEvent>();
    app.add_message::<TransitionRequest>();
    app.add_message::<TransitionComplete>();
    app.add_message::<StorySelectEvent>();
}

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    register_messages(&mut app);
    app.init_state::<VnAppState>();
    app.insert_resource(SaveLoadMode::default());
    app.insert_resource(ScriptEngine::new());
    app.add_plugins(ScriptRunnerPlugin);
    app.add_systems(Update, return_to_title_on_story_end_like);

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = manifest_dir.join("../../examples/minimal/assets/scripts");
    let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
    let n = load_all_scripts(&mut engine, &assets_dir);
    assert!(n > 50, "expected all real scripts loaded, got {n}");
    app
}

fn return_to_title_on_story_end_like(
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

#[test]
fn chapter_nar1_08_returns_to_title_on_end() {
    let mut app = build_app();

    let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
    engine.set_current("nar1_08", Some("top")).unwrap();
    drop(engine);
    app.world_mut().resource_mut::<NextState<VnAppState>>().set(VnAppState::Gameplay);

    let mut dialogue_cursor = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<DialogueStateEvent>>()
        .get_cursor();

    // Simulate: user presses Space repeatedly + wait timer completes.
    let mut frames = 0;
    let mut last_text = String::new();
    loop {
        frames += 1;
        assert!(frames < 20_000, "nar1_08 never finished (frames={frames}, last={last_text:?})");

        app.world_mut().resource_mut::<ScriptBlock>().blocked = false;
        if app.world().resource::<WaitTimer>().timer.is_some() {
            // pretend the wait finished: clear it, then the runner advances past it
            app.world_mut().resource_mut::<WaitTimer>().timer = None;
        }
        if app.world().resource::<BgWait>().waiting {
            // headless test: pretend the bg transition finished immediately
            let _ = app.world_mut().write_message(SetBgDoneEvent);
        }
        let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
        app.update();

        if *app.world().resource::<State<VnAppState>>().get() == VnAppState::Title {
            println!("nar1_08 returned to Title in {frames} frames");
            return;
        }
        {
            let w = app.world_mut();
            let msgs = w.resource::<bevy::ecs::message::Messages<DialogueStateEvent>>();
            for d in dialogue_cursor.read(msgs) {
                if !d.text.is_empty() { last_text = d.text.clone(); }
            }
        }
        let engine = app.world().resource::<ScriptEngine>();
        if engine.finished && !engine.has_more() {
            println!("engine finished at frame {frames}: has_more={}", engine.has_more());
        }
        if frames % 5000 == 0 {
            let cur = engine.current();
            let cmd = cur.map(|c| format!("{c:?}")).unwrap_or_else(|| "None".into());
            let script = engine.current_script.clone();
            println!(
                "[frame {frames}] line={} script={script} cur={cmd} has_more={} finished={}",
                engine.current_line,
                engine.has_more(),
                engine.finished
            );
        }
    }
}
