use bevy::prelude::*;
use crate::messages::*;
use crate::script::{ScriptCmd, ScriptEngine};
use crate::state::{SaveLoadMode, VnAppState};

#[derive(Resource, Default)]
pub struct ScriptBlock { pub blocked: bool }

#[derive(Resource, Default)]
pub struct AutoSkip { pub enabled: bool, pub timer: Timer }

/// Auto-advance timer armed by `Wait` commands. When it elapses, an
/// `AdvanceEvent(Auto)` is emitted so the script continues without input.
#[derive(Resource, Default)]
pub struct WaitTimer { pub timer: Option<Timer> }

#[derive(Resource, Default)]
pub struct EventQueue { items: Vec<EventItem> }

impl EventQueue {
    /// Remove and return only the items matching `pred`, leaving the rest
    /// for other flush systems (each flush consumes only its own kind).
    fn take_where(&mut self, mut pred: impl FnMut(&EventItem) -> bool) -> Vec<EventItem> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.items.len() {
            if pred(&self.items[i]) {
                out.push(self.items.remove(i));
            } else {
                i += 1;
            }
        }
        out
    }
}

#[derive(Clone)]
enum EventItem {
    Dialogue(DialogueStateEvent),
    ClearDialogue,
    Choice(ChoiceStateEvent),
    Backlog(BacklogPushEvent),
    Hotspot(HotspotEvent),
    HotspotClear,
    Render(RenderEvent),
    Audio(AudioEvent),
    Video(VideoEvent),
    Other(OtherEvent),
    Custom(CustomTagEvent),
}
#[derive(Clone)] enum RenderEvent { SetBg(SetBgEvent), ShowFg(ShowFgEvent), HideFg(HideFgEvent), ShowFace(ShowFaceEvent), ShowCg(ShowCgEvent), HideCg(HideCgEvent), ScrollBg(ScrollBgEvent), Sprite(SpriteEvent), SpriteEffect(SpriteEffectEvent), ScreenEffect(ScreenEffectEvent) }
#[derive(Clone)] enum AudioEvent { PlayBgm(PlayBgmEvent), StopBgm(StopBgmEvent), PlaySe(PlaySeEvent), StopSe(StopSeEvent), PlayVoice(PlayVoiceEvent), SetVolume(SetVolumeEvent) }
#[derive(Clone)] enum VideoEvent { PlayMovie(PlayMovieEvent), StopMovie, SpriteVideo(SpriteVideoEvent), StopSpriteVideo(StopSpriteVideoEvent) }
#[derive(Clone)] enum OtherEvent { UnlockCg(UnlockCgEvent), UnlockBgm(UnlockBgmEvent), SavePoint(SavePointEvent) }

pub struct ScriptRunnerPlugin;
impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptBlock>()
            .init_resource::<AutoSkip>()
            .init_resource::<WaitTimer>()
            .init_resource::<EventQueue>()
            .add_systems(Update, unblock_on_choice)
            .add_systems(Update, process_advance.run_if(in_state(VnAppState::Gameplay)).run_if(not(save_load_active)))
            .add_systems(Update, auto_skip_tick.run_if(in_state(VnAppState::Gameplay)).run_if(not(save_load_active)))
            .add_systems(Update, wait_tick.run_if(in_state(VnAppState::Gameplay)).run_if(not(save_load_active)))
            .add_systems(Update, flush_render)
            .add_systems(Update, flush_hotspot)
            .add_systems(Update, flush_custom)
            .add_systems(Update, flush_audio)
            .add_systems(Update, flush_video)
            .add_systems(Update, flush_other);
    }
}

fn unblock_on_choice(mut r: MessageReader<ChoiceSelectedEvent>, mut blk: ResMut<ScriptBlock>) {
    for _ in r.read() { blk.blocked = false; }
}

fn save_load_active(mode: Res<SaveLoadMode>) -> bool {
    mode.active
}

#[derive(PartialEq)]
enum R { Continue, Block, Finished }

fn process_advance(
    mut reader: MessageReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    block: Res<ScriptBlock>,
    mut queue: ResMut<EventQueue>,
    mut wait: ResMut<WaitTimer>,
) {
    if block.blocked { return; }
    let mut skip = false;
    let mut had_event = false;
    for e in reader.read() {
        had_event = true;
        skip = e.source == AdvanceSource::Skip;
    }
    if !had_event { return; }
    loop {
        let Some(cmd) = engine.current().cloned() else { break };
        match dispatch(&cmd, &mut *engine, skip, &mut queue, &mut wait) {
            R::Continue => { engine.advance(); }
            R::Block => { engine.advance(); break; }
            R::Finished => { engine.finished = true; break; }
        }
        if block.blocked { break; }
    }
}

fn wait_tick(
    time: Res<Time>,
    mut wait: ResMut<WaitTimer>,
    mut writer: MessageWriter<AdvanceEvent>,
) {
    if let Some(timer) = &mut wait.timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            wait.timer = None;
            let _ = writer.write(AdvanceEvent { source: AdvanceSource::Auto });
        }
    }
}

fn dispatch(
    cmd: &ScriptCmd,
    eng: &mut ScriptEngine,
    skip: bool,
    q: &mut EventQueue,
    wt: &mut WaitTimer,
) -> R {
    use crate::script::cmd::ConditionOp;
    match cmd {
        ScriptCmd::Label { .. } => R::Continue,
        ScriptCmd::Jump { label } => { let _ = eng.jump_to_label(label); R::Continue }
        ScriptCmd::Call { label } => { let _ = eng.call_label(label); R::Continue }
        ScriptCmd::CallScript { script, label } => { let _ = eng.call_script(script, label.as_deref()); R::Continue }
        ScriptCmd::Return => { let _ = eng.return_from_call(); R::Continue }
        ScriptCmd::Condition { expression, goto_true, goto_false } => {
            match crate::script::evaluate_condition(expression, &eng.flags) {
                Ok(true) => { let _ = eng.jump_to_label(goto_true); }
                Ok(false) => { if let Some(l) = goto_false { let _ = eng.jump_to_label(l); } }
                Err(_) => {}
            }; R::Continue
        }
        ScriptCmd::IfFlag { flag_key, op, value, goto } => {
            let fv = eng.flags.get(flag_key).copied().unwrap_or(0);
            let rhs: i32 = value.parse().unwrap_or(0);
            let ok = match op {
                ConditionOp::Eq => fv == rhs, ConditionOp::Ne => fv != rhs,
                ConditionOp::Gt => fv > rhs, ConditionOp::Ge => fv >= rhs,
                ConditionOp::Lt => fv < rhs, ConditionOp::Le => fv <= rhs,
            }; if ok { let _ = eng.jump_to_label(goto); }; R::Continue
        }
        ScriptCmd::Halt => R::Finished,
        ScriptCmd::Dialogue { speaker, text, voice } => {
            if skip { return R::Continue; }
            q.items.push(EventItem::Backlog(BacklogPushEvent { entry: BacklogEntry { speaker: speaker.clone(), text: text.clone(), voice_file: voice.clone() }}));
            if let Some(file) = voice {
                q.items.push(EventItem::Audio(AudioEvent::PlayVoice(PlayVoiceEvent { file: file.clone(), volume: None })));
            }
            q.items.push(EventItem::Dialogue(DialogueStateEvent { speaker: speaker.clone(), text: text.clone() }));
            R::Block
        }
        ScriptCmd::ClearDialogue => { q.items.push(EventItem::ClearDialogue); R::Continue }
        ScriptCmd::ChoiceBegin => R::Continue,
        ScriptCmd::ChoiceOption { text, goto, affection } => {
            q.items.push(EventItem::Choice(ChoiceStateEvent { options: vec![ChoiceOption { text: text.clone(), goto: goto.clone(), affection: affection.clone() }]}));
            R::Block
        }
        ScriptCmd::ChoiceEnd { .. } => R::Continue,
        ScriptCmd::Hotspot { id, x, y, width, height } => {
            q.items.push(EventItem::Hotspot(HotspotEvent { id: id.clone(), x: *x, y: *y, width: *width, height: *height }));
            R::Continue
        }
        ScriptCmd::HotspotClear => { q.items.push(EventItem::HotspotClear); R::Continue }
        ScriptCmd::SetBg { image, transition } => { q.items.push(EventItem::Render(RenderEvent::SetBg(SetBgEvent { image: image.clone(), transition: *transition }))); R::Continue }
        ScriptCmd::ShowFg { char_id, expression, position, transition } => {
            q.items.push(EventItem::Render(RenderEvent::ShowFg(ShowFgEvent { char_id: char_id.clone(), expression: expression.clone(), position: *position, transition: *transition }))); R::Continue
        }
        ScriptCmd::HideFg { char_id, transition } => { q.items.push(EventItem::Render(RenderEvent::HideFg(HideFgEvent { char_id: char_id.clone(), transition: *transition }))); R::Continue }
        ScriptCmd::ShowFace { char_id, expression } => { q.items.push(EventItem::Render(RenderEvent::ShowFace(ShowFaceEvent { char_id: char_id.clone(), expression: expression.clone() }))); R::Continue }
        ScriptCmd::ShowCg { image, transition } => { q.items.push(EventItem::Render(RenderEvent::ShowCg(ShowCgEvent { image: image.clone(), transition: *transition }))); R::Continue }
        ScriptCmd::HideCg { transition } => { q.items.push(EventItem::Render(RenderEvent::HideCg(HideCgEvent { transition: *transition }))); R::Continue }
        ScriptCmd::ScrollBg { speed_x, speed_y, time_ms } => { q.items.push(EventItem::Render(RenderEvent::ScrollBg(ScrollBgEvent { speed_x: *speed_x, speed_y: *speed_y, time_ms: *time_ms }))); R::Continue }
        ScriptCmd::Sprite { id, image, x, y, anchor_x, anchor_y, z } => {
            q.items.push(EventItem::Render(RenderEvent::Sprite(SpriteEvent { id: id.clone(), image: image.clone(), x: *x, y: *y, anchor_x: *anchor_x, anchor_y: *anchor_y, z: *z }))); R::Continue
        }
        ScriptCmd::SpriteFade { id, opacity, duration_ms } => {
            q.items.push(EventItem::Render(RenderEvent::SpriteEffect(SpriteEffectEvent { id: id.clone(), effect: SpriteEffectKind::Fade { opacity: *opacity, duration_ms: *duration_ms } }))); R::Continue
        }
        ScriptCmd::SpriteMove { id, x, y, duration_ms } => {
            q.items.push(EventItem::Render(RenderEvent::SpriteEffect(SpriteEffectEvent { id: id.clone(), effect: SpriteEffectKind::Move { x: *x, y: *y, duration_ms: *duration_ms } }))); R::Continue
        }
        ScriptCmd::SpriteRemove { id } => { q.items.push(EventItem::Render(RenderEvent::SpriteEffect(SpriteEffectEvent { id: id.clone(), effect: SpriteEffectKind::Remove }))); R::Continue }
        ScriptCmd::ScreenEffect { kind, color, duration_ms } => { q.items.push(EventItem::Render(RenderEvent::ScreenEffect(ScreenEffectEvent { kind: *kind, color: color.clone(), duration_ms: *duration_ms }))); R::Continue }
        ScriptCmd::Shake { .. } => R::Continue,
        ScriptCmd::PlayBgm { id, volume, fade_ms } => { q.items.push(EventItem::Audio(AudioEvent::PlayBgm(PlayBgmEvent { id: id.clone(), volume: *volume, fade_ms: *fade_ms }))); R::Continue }
        ScriptCmd::StopBgm { fade_ms } => { q.items.push(EventItem::Audio(AudioEvent::StopBgm(StopBgmEvent { fade_ms: *fade_ms }))); R::Continue }
        ScriptCmd::PlaySe { file, channel, volume } => {
            if skip { return R::Continue; }
            q.items.push(EventItem::Audio(AudioEvent::PlaySe(PlaySeEvent { file: file.clone(), channel: *channel, volume: *volume }))); R::Continue
        }
        ScriptCmd::StopSe { channel } => { q.items.push(EventItem::Audio(AudioEvent::StopSe(StopSeEvent { channel: *channel }))); R::Continue }
        ScriptCmd::PlayVoice { file, volume } => { q.items.push(EventItem::Audio(AudioEvent::PlayVoice(PlayVoiceEvent { file: file.clone(), volume: *volume }))); R::Continue }
        ScriptCmd::SetVolume { bgm, se, voice } => { q.items.push(EventItem::Audio(AudioEvent::SetVolume(SetVolumeEvent { bgm: *bgm, se: *se, voice: *voice }))); R::Continue }
        ScriptCmd::Wait { time_ms } => {
            if skip { R::Continue }
            else {
                wt.timer = Some(Timer::from_seconds(*time_ms as f32 / 1000.0, TimerMode::Once));
                R::Block
            }
        }
        ScriptCmd::SetFlag { key, value } => { eng.flags.insert(key.clone(), *value); R::Continue }
        ScriptCmd::SetGlobalFlag { flag_id, value } => { eng.global_flags.insert(*flag_id, *value); R::Continue }
        ScriptCmd::UnlockCg { image } => { q.items.push(EventItem::Other(OtherEvent::UnlockCg(UnlockCgEvent { image: image.clone() }))); R::Continue }
        ScriptCmd::UnlockBgm { id } => { q.items.push(EventItem::Other(OtherEvent::UnlockBgm(UnlockBgmEvent { id: id.clone() }))); R::Continue }
        ScriptCmd::SavePoint { id } => { q.items.push(EventItem::Other(OtherEvent::SavePoint(SavePointEvent { id: id.clone() }))); R::Continue }
        ScriptCmd::SetNextScript { .. } | ScriptCmd::RouteFlag { .. } | ScriptCmd::SetMode { .. } => R::Continue,
        ScriptCmd::PlayMovie { file, blocking } => {
            q.items.push(EventItem::Video(VideoEvent::PlayMovie(PlayMovieEvent { file: file.clone(), blocking: *blocking })));
            if *blocking { R::Block } else { R::Continue }
        }
        ScriptCmd::StopMovie => { q.items.push(EventItem::Video(VideoEvent::StopMovie)); R::Continue }
        ScriptCmd::SpriteVideo { id, file, x, y } => { q.items.push(EventItem::Video(VideoEvent::SpriteVideo(SpriteVideoEvent { id: id.clone(), file: file.clone(), x: *x, y: *y }))); R::Continue }
        ScriptCmd::StopSpriteVideo { id } => { q.items.push(EventItem::Video(VideoEvent::StopSpriteVideo(StopSpriteVideoEvent { id: id.clone() }))); R::Continue }
        ScriptCmd::Custom { tag, data } => match tag.as_str() {
            "タイトル" => {
                q.items.push(EventItem::Custom(CustomTagEvent { tag: tag.clone(), data: data.clone() }));
                R::Block
            }
            "brandlogo" => {
                q.items.push(EventItem::Custom(CustomTagEvent { tag: tag.clone(), data: data.clone() }));
                R::Continue
            }
            "msg" | "msgoff" | "keyskip" | _ => R::Continue,
        },
        ScriptCmd::Unknown | ScriptCmd::HideFace { .. } | ScriptCmd::UnlockScene { .. } | ScriptCmd::ScrollView { .. } => R::Continue,
    }
}

// ── Flush systems (split to avoid Bevy param count limit) ──

fn flush_other(mut q: ResMut<EventQueue>, mut wuc: MessageWriter<UnlockCgEvent>, mut wub: MessageWriter<UnlockBgmEvent>, mut wsv: MessageWriter<SavePointEvent>) {
    for evt in q.take_where(|e| matches!(e, EventItem::Other(_))) {
        match evt {
            EventItem::Other(OtherEvent::UnlockCg(e)) => { let _ = wuc.write(e); }
            EventItem::Other(OtherEvent::UnlockBgm(e)) => { let _ = wub.write(e); }
            EventItem::Other(OtherEvent::SavePoint(e)) => { let _ = wsv.write(e); }
            _ => {}
        }
    }
}

fn flush_video(mut q: ResMut<EventQueue>, mut wmv: MessageWriter<PlayMovieEvent>, mut wms: MessageWriter<StopMovieEvent>, mut wsvid: MessageWriter<SpriteVideoEvent>, mut wsvs: MessageWriter<StopSpriteVideoEvent>) {
    for evt in q.take_where(|e| matches!(e, EventItem::Video(_))) {
        match evt {
            EventItem::Video(VideoEvent::PlayMovie(e)) => { let _ = wmv.write(e); }
            EventItem::Video(VideoEvent::StopMovie) => { let _ = wms.write(StopMovieEvent); }
            EventItem::Video(VideoEvent::SpriteVideo(e)) => { let _ = wsvid.write(e); }
            EventItem::Video(VideoEvent::StopSpriteVideo(e)) => { let _ = wsvs.write(e); }
            _ => {}
        }
    }
}

fn flush_hotspot(mut q: ResMut<EventQueue>, mut wh: MessageWriter<HotspotEvent>, mut wc: MessageWriter<HotspotClearEvent>) {
    for evt in q.take_where(|e| matches!(e, EventItem::Hotspot(_) | EventItem::HotspotClear)) {
        match evt {
            EventItem::Hotspot(e) => { let _ = wh.write(e); }
            EventItem::HotspotClear => { let _ = wc.write(HotspotClearEvent); }
            _ => {}
        }
    }
}

fn flush_custom(mut q: ResMut<EventQueue>, mut wc: MessageWriter<CustomTagEvent>) {
    for evt in q.take_where(|e| matches!(e, EventItem::Custom(_))) {
        if let EventItem::Custom(e) = evt { let _ = wc.write(e); }
    }
}

pub fn flush_audio(mut q: ResMut<EventQueue>, mut wbm: MessageWriter<PlayBgmEvent>, mut wbs: MessageWriter<StopBgmEvent>, mut wse: MessageWriter<PlaySeEvent>, mut wss: MessageWriter<StopSeEvent>, mut wvo: MessageWriter<PlayVoiceEvent>, mut wvl: MessageWriter<SetVolumeEvent>) {
    for evt in q.take_where(|e| matches!(e, EventItem::Audio(_))) {
        match evt {
            EventItem::Audio(AudioEvent::PlayBgm(e)) => { let _ = wbm.write(e); }
            EventItem::Audio(AudioEvent::StopBgm(e)) => { let _ = wbs.write(e); }
            EventItem::Audio(AudioEvent::PlaySe(e)) => { let _ = wse.write(e); }
            EventItem::Audio(AudioEvent::StopSe(e)) => { let _ = wss.write(e); }
            EventItem::Audio(AudioEvent::PlayVoice(e)) => { let _ = wvo.write(e); }
            EventItem::Audio(AudioEvent::SetVolume(e)) => { let _ = wvl.write(e); }
            _ => {}
        }
    }
}

fn flush_render(mut q: ResMut<EventQueue>, mut wd: MessageWriter<DialogueStateEvent>, mut wc: MessageWriter<ClearDialogueEvent>, mut wch: MessageWriter<ChoiceStateEvent>, mut wbl: MessageWriter<BacklogPushEvent>,
    mut wbg: MessageWriter<SetBgEvent>, mut wfg_s: MessageWriter<ShowFgEvent>, mut wfg_h: MessageWriter<HideFgEvent>, mut wfa: MessageWriter<ShowFaceEvent>,
    mut wcg_s: MessageWriter<ShowCgEvent>, mut wcg_h: MessageWriter<HideCgEvent>, mut wsc: MessageWriter<ScrollBgEvent>,
    mut wsp: MessageWriter<SpriteEvent>, mut wspf: MessageWriter<SpriteEffectEvent>, mut wov: MessageWriter<ScreenEffectEvent>,
) {
    for evt in q.take_where(|e| {
        matches!(e,
            EventItem::Dialogue(_) | EventItem::ClearDialogue | EventItem::Choice(_) |
            EventItem::Backlog(_) | EventItem::Render(_))
    }) {
        match evt {
            EventItem::Dialogue(e) => { let _ = wd.write(e); }
            EventItem::ClearDialogue => { let _ = wc.write(ClearDialogueEvent); }
            EventItem::Choice(e) => { let _ = wch.write(e); }
            EventItem::Backlog(e) => { let _ = wbl.write(e); }
            EventItem::Render(RenderEvent::SetBg(e)) => { let _ = wbg.write(e); }
            EventItem::Render(RenderEvent::ShowFg(e)) => { let _ = wfg_s.write(e); }
            EventItem::Render(RenderEvent::HideFg(e)) => { let _ = wfg_h.write(e); }
            EventItem::Render(RenderEvent::ShowFace(e)) => { let _ = wfa.write(e); }
            EventItem::Render(RenderEvent::ShowCg(e)) => { let _ = wcg_s.write(e); }
            EventItem::Render(RenderEvent::HideCg(e)) => { let _ = wcg_h.write(e); }
            EventItem::Render(RenderEvent::ScrollBg(e)) => { let _ = wsc.write(e); }
            EventItem::Render(RenderEvent::Sprite(e)) => { let _ = wsp.write(e); }
            EventItem::Render(RenderEvent::SpriteEffect(e)) => { let _ = wspf.write(e); }
            EventItem::Render(RenderEvent::ScreenEffect(e)) => { let _ = wov.write(e); }
            _ => {}
        }
    }
}

fn auto_skip_tick(time: Res<Time>, mut skip: ResMut<AutoSkip>, mut w: MessageWriter<AdvanceEvent>) {
    skip.timer.tick(time.delta());
    if skip.timer.just_finished() { w.write(AdvanceEvent { source: AdvanceSource::Auto }); }
}
