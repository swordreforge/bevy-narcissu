//! Generic sprite overlay system.

use bevy::prelude::*;
use bevy_vn_core::messages::{ScrollBgEvent, SpriteEffectEvent, SpriteEvent};
use crate::AssetPathProvider;

#[derive(Component)]
pub struct SpriteOverlayMarker { pub id: String }

pub struct SpriteOverlayPlugin;
impl Plugin for SpriteOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_sprite_spawn, handle_sprite_effect, handle_scroll_bg));
    }
}

fn handle_sprite_spawn(
    mut reader: MessageReader<SpriteEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    provider: Option<Res<AssetPathProvider>>,
    q_existing: Query<(Entity, &SpriteOverlayMarker)>,
) {
    for event in reader.read() {
        for (e, m) in q_existing.iter() { if m.id == event.id { commands.entity(e).despawn(); } }
        // "nobaby" 是 artemis 原版引擎的"清除立绘图层"哨兵值(原版脚本中紧随
        // "立绘代码-消失"注释、path=":fg/"),并非真实资源。仅清除同 id 立绘,
        // 不再加载不存在的 image/anime/nobaby.basisu.ktx2。
        if event.image == "nobaby" {
            continue;
        }
        let path = AssetPathProvider::resolve(provider.as_deref(), |p| p.sprite(&event.image));
        let handle = asset_server.load::<Image>(&path);
        let z = event.z.unwrap_or(5);
        commands.spawn((
            SpriteOverlayMarker { id: event.id.clone() },
            ImageNode { image: handle, ..default() },
            Node { position_type: PositionType::Absolute, left: Val::Px(event.x), bottom: Val::Px(event.y), ..default() },
            ZIndex(z),
        ));
    }
}

fn handle_sprite_effect(
    mut reader: MessageReader<SpriteEffectEvent>,
    mut commands: Commands,
    mut q: Query<(Entity, &SpriteOverlayMarker, &mut Node)>,
) {
    for event in reader.read() {
        let target: Vec<Entity> = q.iter().filter(|(_, m, _)| m.id == event.id).map(|(e, ..)| e).collect();
        for e in target {
            match &event.effect {
                bevy_vn_core::messages::SpriteEffectKind::Move { x, y, .. } => {
                    if let Ok((_, _, mut node)) = q.get_mut(e) { node.left = Val::Px(*x); node.bottom = Val::Px(*y); }
                }
                bevy_vn_core::messages::SpriteEffectKind::Remove => { commands.entity(e).despawn(); }
                _ => {}
            }
        }
    }
}

fn handle_scroll_bg(mut reader: MessageReader<ScrollBgEvent>) {
    for _ in reader.read() { /* TODO: scroll implementation */ }
}
