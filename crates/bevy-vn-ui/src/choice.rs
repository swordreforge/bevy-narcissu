//! Choice UI — renders option buttons, emits ChoiceSelectedEvent on click.

use bevy::prelude::*;
use bevy_vn_core::messages::{ChoiceSelectedEvent, ChoiceStateEvent};
use bevy_vn_core::theme::VnTheme;

#[derive(Component)]
struct ChoiceRoot;

#[derive(Component)]
struct ChoiceButton(usize);

pub struct ChoicePlugin;

impl Plugin for ChoicePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_choice_state, handle_choice_click));
    }
}

fn handle_choice_state(
    mut reader: MessageReader<ChoiceStateEvent>,
    mut commands: Commands,
    theme: Option<Res<VnTheme>>,
    q_old: Query<Entity, With<ChoiceRoot>>,
) {
    for event in reader.read() {
        // Remove old choice UI
        for e in q_old.iter() { commands.entity(e).despawn(); }
        if event.options.is_empty() { continue; }

        let ct = theme.as_ref()
            .map(|t| t.choice.clone())
            .unwrap_or_default();

        commands.spawn((
            ChoiceRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(200.0),
                left: Val::Px(80.0),
                width: Val::Px(1120.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ZIndex(20),
        ))
        .with_children(|parent| {
            for (i, opt) in event.options.iter().enumerate() {
                parent.spawn((
                    ChoiceButton(i),
                    Button,
                    Node {
                        width: Val::Px(1100.0),
                        height: Val::Px(ct.item_height),
                        padding: UiRect {
                            left: Val::Px(ct.padding[0]), right: Val::Px(ct.padding[1]),
                            top: Val::Px(ct.padding[2]), bottom: Val::Px(ct.padding[3]),
                        },
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
                ))
                .with_child((
                    Text::new(&opt.text),
                    TextFont { font_size: FontSize::Px(ct.font_size), ..default() },
                    TextColor(Color::WHITE),
                ));
            }
        });
    }
}

fn handle_choice_click(
    q: Query<(&ChoiceButton, &Interaction), Changed<Interaction>>,
    mut writer: MessageWriter<ChoiceSelectedEvent>,
) {
    for (btn, interaction) in q.iter() {
        if *interaction == Interaction::Pressed {
            writer.write(ChoiceSelectedEvent { index: btn.0 });
        }
    }
}
