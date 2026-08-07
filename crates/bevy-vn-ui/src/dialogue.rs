//! Dialogue box UI — text reveal, speaker name, themed via VnTheme.

use bevy::prelude::*;
use bevy_vn_core::messages::{ClearDialogueEvent, DialogueStateEvent};
use bevy_vn_core::state::VnAppState;
use bevy_vn_core::theme::VnTheme;

#[derive(Component)]
struct SpeakerText;

#[derive(Component)]
struct DialogueText;

#[derive(Component)]
pub struct DialogueRoot;

#[derive(Resource, Default)]
pub struct DialogueUiState {
    pub root: Option<Entity>,
}

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueUiState>()
            .init_resource::<TextReveal>()
            .add_systems(Startup, spawn_dialogue_ui)
            .add_systems(Update, (handle_dialogue, handle_clear, reveal_text));
    }
}

fn spawn_dialogue_ui(
    mut commands: Commands,
    theme: Option<Res<VnTheme>>,
    mut state: ResMut<DialogueUiState>,
) {
    let t = theme.map(|t| t.clone()).unwrap_or_default();
    let dt = &t.dialogue;

    let root = commands.spawn((
        DialogueRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(dt.margin_bottom),
            left: Val::Px(20.0),
            width: Val::Px(1240.0),
            height: Val::Px(dt.height),
            padding: UiRect::all(Val::Px(dt.padding[0])),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(
            dt.background_color[0], dt.background_color[1],
            dt.background_color[2], dt.background_color[3],
        )),
        ZIndex(10),
    ))
    .with_children(|parent| {
        parent.spawn((
            SpeakerText,
            Text::new(""),
            TextFont { font_size: FontSize::Px(dt.speaker_font_size), ..default() },
            TextColor(Color::srgba(dt.speaker_color[0], dt.speaker_color[1], dt.speaker_color[2], dt.speaker_color[3])),
            Node { width: Val::Px(dt.speaker_box_width), height: Val::Px(dt.speaker_font_size + 4.0), ..default() },
        ));
        parent.spawn((
            DialogueText,
            Text::new(""),
            TextFont { font_size: FontSize::Px(dt.font_size), ..default() },
            TextColor(Color::srgba(dt.text_color[0], dt.text_color[1], dt.text_color[2], dt.text_color[3])),
            Node { width: percent(100), flex_grow: 1.0, ..default() },
        ));
    }).id();
    state.root = Some(root);
}

#[derive(Resource)]
struct TextReveal {
    full_text: String,
    revealed: usize,
    timer: Timer,
    fade: f32,
    base_alpha: f32,
}

impl Default for TextReveal {
    fn default() -> Self {
        Self {
            full_text: String::new(),
            revealed: 0,
            timer: Timer::from_seconds(0.02, TimerMode::Repeating),
            fade: 1.0,
            base_alpha: 1.0,
        }
    }
}

const DIALOGUE_FADE: f32 = 0.25;

fn handle_dialogue(
    state: Res<State<VnAppState>>,
    mut reader: MessageReader<DialogueStateEvent>,
    mut q_root: Query<&mut Node, (With<DialogueRoot>, Without<SpeakerText>)>,
    mut q_speaker: Query<(&mut Text, &mut Node), (With<SpeakerText>, Without<DialogueText>)>,
    _q_text: Query<&mut Text, With<DialogueText>>,
    mut reveal: ResMut<TextReveal>,
    theme: Option<Res<VnTheme>>,
) {
    // 迟到的对话事件(离开 Gameplay 的同帧/次帧 flush)不得重新显示对话框
    if *state.get() != VnAppState::Gameplay {
        return;
    }
    for event in reader.read() {
        for mut node in q_root.iter_mut() { node.display = Display::Flex; }

        let text = event.text.clone();
        let speed = theme.as_ref()
            .and_then(|t| t.dialogue.text_speed)
            .unwrap_or(50.0);

        reveal.full_text = text;
        reveal.revealed = 0;
        reveal.fade = 0.0;
        reveal.base_alpha = theme.as_ref()
            .and_then(|t| t.dialogue.text_color.get(3).copied())
            .unwrap_or(1.0);
        reveal.timer = Timer::from_seconds((1.0 / speed) as f32, TimerMode::Repeating);

        // Speaker
        for (mut t, mut n) in q_speaker.iter_mut() {
            if let Some(ref s) = event.speaker { **t = s.clone(); n.display = Display::Flex; }
            else { n.display = Display::None; }
        }
    }
}

fn reveal_text(
    time: Res<Time>,
    mut reveal: ResMut<TextReveal>,
    mut q: Query<(&mut Text, &mut TextColor), With<DialogueText>>,
) {
    if reveal.full_text.is_empty() { return; }
    if reveal.fade < 1.0 {
        reveal.fade = (reveal.fade + time.delta_secs() / DIALOGUE_FADE).min(1.0);
    }
    reveal.timer.tick(time.delta());
    let chars = reveal.timer.times_finished_this_tick() as usize;
    reveal.revealed = (reveal.revealed + chars).min(reveal.full_text.chars().count());
    for (mut t, mut c) in q.iter_mut() {
        let visible: String = reveal.full_text.chars().take(reveal.revealed).collect();
        **t = visible;
        c.0.set_alpha(reveal.fade * reveal.base_alpha);
    }
}

fn handle_clear(
    mut reader: MessageReader<ClearDialogueEvent>,
    mut q_root: Query<&mut Node, With<DialogueRoot>>,
    mut reveal: ResMut<TextReveal>,
) {
    for _ in reader.read() {
        for mut node in q_root.iter_mut() { node.display = Display::None; }
        reveal.full_text.clear();
        reveal.revealed = 0;
        reveal.fade = 1.0;
    }
}
