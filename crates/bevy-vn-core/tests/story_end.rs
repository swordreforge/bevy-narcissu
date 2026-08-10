use bevy::app::App;
use bevy::prelude::*;
use bevy_vn_core::messages::*;
use bevy_vn_core::runner::*;
use bevy_vn_core::script::{ScriptEngine, VnScript};
use bevy_vn_core::state::{
    GameplayMenuMode, SaveLoadMode, SettingsOverlayMode, SkipMode, VnAppState,
};

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
    app.insert_resource(GameplayMenuMode::default());
    app.insert_resource(SettingsOverlayMode::default());
    app.insert_resource(SkipMode::default());
    app.insert_resource(SkipTimer(Timer::from_seconds(0.08, TimerMode::Repeating)));
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

#[test]
fn start_story01_plays_whole_story() {
    let mut app = build_app();

    let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
    engine.set_current("game-logic", Some("story01")).unwrap();
    drop(engine);
    app.world_mut().resource_mut::<NextState<VnAppState>>().set(VnAppState::Gameplay);

    let mut frames = 0;
    let mut scripts_seen: Vec<String> = Vec::new();
    let mut last_text = String::new();
    loop {
        frames += 1;
        assert!(frames < 60_000, "story01 never finished (frames={frames}, scripts={scripts_seen:?}, last={last_text:?})");

        {
            let e = app.world().resource::<ScriptEngine>();
            let s = e.current_script.clone();
            if scripts_seen.last() != Some(&s) { scripts_seen.push(s); }
        }
        app.world_mut().resource_mut::<ScriptBlock>().blocked = false;
        if app.world().resource::<WaitTimer>().timer.is_some() {
            app.world_mut().resource_mut::<WaitTimer>().timer = None;
        }
        if app.world().resource::<BgWait>().waiting {
            let _ = app.world_mut().write_message(SetBgDoneEvent);
        }
        let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
        app.update();

        if *app.world().resource::<State<VnAppState>>().get() == VnAppState::Title {
            println!("story01 returned to Title in {frames} frames; scripts={scripts_seen:?}");
            assert!(scripts_seen.len() >= 9, "expected all 9 nar1 chapters played, only saw: {scripts_seen:?}");
            return;
        }
    }
}

#[test]
fn state_after_return_to_title_is_stale() {
    let mut app = build_app();
    // Add the real タイトル handler like main.rs does
    app.add_systems(Update, handle_custom_tag_like);

    let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
    engine.set_current("game-logic", Some("story01")).unwrap();
    drop(engine);
    app.world_mut().resource_mut::<NextState<VnAppState>>().set(VnAppState::Gameplay);

    let mut frames = 0;
    loop {
        frames += 1;
        assert!(frames < 60_000, "never reached title");
        app.world_mut().resource_mut::<ScriptBlock>().blocked = false;
        if app.world().resource::<WaitTimer>().timer.is_some() {
            app.world_mut().resource_mut::<WaitTimer>().timer = None;
        }
        if app.world().resource::<BgWait>().waiting {
            let _ = app.world_mut().write_message(SetBgDoneEvent);
        }
        let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
        app.update();
        if *app.world().resource::<State<VnAppState>>().get() == VnAppState::Title {
            let e = app.world().resource::<ScriptEngine>();
            println!("at Title after {frames} frames:");
            println!("  current_script={:?} current_line={} call_stack={:?} finished={}",
                e.current_script, e.current_line, e.call_stack, e.finished);
            println!("  has_more={} current={:?}", e.has_more(), e.current().map(|c| format!("{c:?}")));
            println!("  flags keys: {:?}", e.flags.keys().collect::<Vec<_>>());
            return;
        }
    }
}

fn handle_custom_tag_like(
    mut reader: MessageReader<CustomTagEvent>,
    mut block: ResMut<ScriptBlock>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    for e in reader.read() {
        if e.tag == "タイトル" {
            block.blocked = true;
            next.set(VnAppState::Title);
        }
    }
}

#[test]
fn chapter_then_start_plays_whole_story() {
    let mut app = build_app();
    app.add_systems(Update, handle_custom_tag_like);

    // Phase 1: read a single chapter from story detail (like chapter select)
    {
        let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
        engine.set_current("nar1_00", Some("top")).unwrap();
        drop(engine);
        app.world_mut().resource_mut::<NextState<VnAppState>>().set(VnAppState::Gameplay);
        let mut frames = 0;
        loop {
            frames += 1;
            assert!(frames < 30_000, "chapter never finished");
            app.world_mut().resource_mut::<ScriptBlock>().blocked = false;
            if app.world().resource::<WaitTimer>().timer.is_some() {
                app.world_mut().resource_mut::<WaitTimer>().timer = None;
            }
            if app.world().resource::<BgWait>().waiting {
                let _ = app.world_mut().write_message(SetBgDoneEvent);
            }
            let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
            app.update();
            if *app.world().resource::<State<VnAppState>>().get() == VnAppState::Title {
                println!("phase1 (chapter) reached Title in {frames} frames");
                break;
            }
        }
    }
    // print engine state after chapter
    {
        let e = app.world().resource::<ScriptEngine>();
        println!("after chapter: script={:?} line={} stack={:?} finished={} has_more={}",
            e.current_script, e.current_line, e.call_stack, e.finished, e.has_more());
        println!("flags: {:?}", e.flags);
    }

    // Phase 2: start a full story from RouteSelect
    {
        let mut engine = app.world_mut().resource_mut::<ScriptEngine>();
        engine.set_current("game-logic", Some("story01")).unwrap();
        drop(engine);
        app.world_mut().resource_mut::<NextState<VnAppState>>().set(VnAppState::Gameplay);
        let mut frames = 0;
        let mut scripts_seen: Vec<String> = Vec::new();
        loop {
            frames += 1;
            assert!(frames < 60_000, "story never finished");
            {
                let e = app.world().resource::<ScriptEngine>();
                let s = e.current_script.clone();
                if scripts_seen.last() != Some(&s) { scripts_seen.push(s); }
            }
            app.world_mut().resource_mut::<ScriptBlock>().blocked = false;
            if app.world().resource::<WaitTimer>().timer.is_some() {
                app.world_mut().resource_mut::<WaitTimer>().timer = None;
            }
            if app.world().resource::<BgWait>().waiting {
                let _ = app.world_mut().write_message(SetBgDoneEvent);
            }
            let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
            app.update();
            if *app.world().resource::<State<VnAppState>>().get() == VnAppState::Title {
                println!("phase2 (start) reached Title in {frames} frames; scripts={scripts_seen:?}");
                println!("phase2 script count: {}", scripts_seen.len());
                return;
            }
        }
    }
}

/// Real-flow harness: mirrors main.rs system wiring exactly.
/// handle_story_select clears blocked on entry; タイトル and story-end set it.
/// The loop NEVER clears blocked manually — only the systems do.
fn build_real_flow_app() -> App {
    let mut app = build_app();
    app.add_systems(Update, (handle_story_select_like, handle_custom_tag_like));
    app
}

fn handle_story_select_like(
    mut reader: MessageReader<StorySelectEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut block: ResMut<ScriptBlock>,
    mut next: ResMut<NextState<VnAppState>>,
) {
    for e in reader.read() {
        match engine.set_current(&e.script, Some(&e.label)) {
            Ok(()) => { block.blocked = false; next.set(VnAppState::Gameplay); }
            Err(err) => panic!("story select {}.{} failed: {err}", e.script, e.label),
        }
    }
}

/// Drive frames; write AdvanceEvent only when engine.has_more() (like user_input).
fn drive_to_title(
    app: &mut App,
    label: &str,
    max_frames: usize,
) -> Vec<String> {
    let mut scripts_seen: Vec<String> = Vec::new();
    let mut entered_gameplay = false;
    for frame in 1..=max_frames {
        {
            let e = app.world().resource::<ScriptEngine>();
            let s = e.current_script.clone();
            if scripts_seen.last() != Some(&s) { scripts_seen.push(s); }
        }
        if app.world().resource::<WaitTimer>().timer.is_some() {
            app.world_mut().resource_mut::<WaitTimer>().timer = None;
        }
        if app.world().resource::<BgWait>().waiting {
            let _ = app.world_mut().write_message(SetBgDoneEvent);
        }
        let has_more = app.world().resource::<ScriptEngine>().has_more();
        if has_more {
            let _ = app.world_mut().write_message(AdvanceEvent { source: AdvanceSource::UserInput });
        }
        let st_before = app.world().resource::<State<VnAppState>>().get().clone();
        app.update();
        let st_after = app.world().resource::<State<VnAppState>>().get().clone();
        let blocked = app.world().resource::<ScriptBlock>().blocked;
        let e = app.world().resource::<ScriptEngine>();
        if frame <= 4 || st_before != st_after {
            println!("[{label}] frame {frame}: state {st_before:?}->{st_after:?} blocked={blocked} script={} line={} has_more={} finished={}",
                e.current_script, e.current_line, e.has_more(), e.finished);
        }
        if frame == 1 && label == "start story01" {
            let n = app.world().resource::<NextState<VnAppState>>();
            println!("  NextState debug: {n:?}");
        }
        let cur = app.world().resource::<State<VnAppState>>().get().clone();
        if cur == VnAppState::Gameplay { entered_gameplay = true; }
        if cur == VnAppState::Title {
            if entered_gameplay {
                println!("[{label}] Title at frame {frame}; scripts={scripts_seen:?}");
                return scripts_seen;
            }
            // still the pre-entry Title (NextState not yet applied) — keep driving
        }
        if frame % 10_000 == 0 {
            let e = app.world().resource::<ScriptEngine>();
            println!("[{label}] frame {frame}: script={} line={} blocked={} has_more={}",
                e.current_script, e.current_line,
                app.world().resource::<ScriptBlock>().blocked, e.has_more());
        }
    }
    panic!("[{label}] never reached Title within {max_frames} frames");
}

#[test]
fn real_flow_chapter_then_start() {
    let mut app = build_real_flow_app();

    // Phase 1: chapter select → read LAST chapter of story 1 (nar1_08)
    {
        let _ = app.world_mut().write_message(StorySelectEvent { script: "nar1_08".into(), label: "top".into() });
        let seen = drive_to_title(&mut app, "chapter nar1_08", 30_000);
        let played: Vec<_> = seen.into_iter().filter(|s| !s.is_empty()).collect();
        assert_eq!(played, vec!["nar1_08".to_string()], "chapter should only play nar1_08");
    }
    // Print engine state + flags after chapter
    {
        let e = app.world().resource::<ScriptEngine>();
        println!("after chapter: script={:?} line={} stack={:?} finished={} has_more={} blocked={}",
            e.current_script, e.current_line, e.call_stack, e.finished, e.has_more(),
            app.world().resource::<ScriptBlock>().blocked);
        println!("flags: {:?}", e.flags);
    }

    // Phase 2: start → RouteSelect → story01 (full story)
    {
        let _ = app.world_mut().write_message(StorySelectEvent { script: "game-logic".into(), label: "story01".into() });
        let seen = drive_to_title(&mut app, "start story01", 60_000);
        let chapters: Vec<_> = seen.iter().filter(|s| s.starts_with("nar1_")).cloned().collect();
        println!("start story01 played chapters: {chapters:?}");
        assert!(chapters.len() >= 9,
            "BUG REPRODUCED: start after chapter select only played {}/9 chapters: {chapters:?}",
            chapters.len());
    }
}
