//! Interaction hotspots — clickable screen regions declared by scripts.
//!
//! Scripts emit `Hotspot { x, y, width, height }` to define which screen
//! areas respond to clicks. While any hotspot is active, a click only
//! advances the script if it lands inside a hotspot; otherwise it is
//! ignored (mirrors the original game's image-based touch areas).

use bevy::prelude::*;
use bevy_vn_core::messages::{AdvanceEvent, AdvanceSource, HotspotClearEvent, HotspotEvent};
use bevy_vn_core::state::SaveLoadMode;

use crate::responsive::{UiCanvasScale, LOGICAL_HEIGHT, LOGICAL_WIDTH};

#[derive(Debug, Clone)]
pub struct HotspotRect {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Resource, Default)]
pub struct ActiveHotspots(pub Vec<HotspotRect>);

pub struct HotspotPlugin;

impl Plugin for HotspotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveHotspots>()
            .add_systems(Update, (apply_hotspot_events, hotspot_click_input).chain());
    }
}

fn apply_hotspot_events(
    mut reader: MessageReader<HotspotEvent>,
    mut clear_reader: MessageReader<HotspotClearEvent>,
    mut active: ResMut<ActiveHotspots>,
) {
    for _ in clear_reader.read() {
        active.0.clear();
    }
    for evt in reader.read() {
        if let Some(slot) = active.0.iter_mut().find(|h| h.id == evt.id) {
            slot.x = evt.x;
            slot.y = evt.y;
            slot.width = evt.width;
            slot.height = evt.height;
        } else {
            active.0.push(HotspotRect {
                id: evt.id.clone(),
                x: evt.x,
                y: evt.y,
                width: evt.width,
                height: evt.height,
            });
        }
    }
}

fn hotspot_click_input(
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    scale: Res<UiCanvasScale>,
    active: Res<ActiveHotspots>,
    mode: Res<SaveLoadMode>,
    mut writer: MessageWriter<AdvanceEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if mode.active {
        return;
    }
    let Some(window) = windows.iter().find(|w| w.focused) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // Hotspots are declared in script pixel coords (960x540 logical space,
    // origin top-left); cursor positions are window logical pixels.
    // Convert window coords -> canvas coords via the responsive canvas scale
    // (and account for the centered letterbox offset when aspect differs).
    let s = scale.0.max(f32::EPSILON);
    let (win_w, win_h) = (window.width(), window.height());
    let canvas_w = LOGICAL_WIDTH * s;
    let canvas_h = LOGICAL_HEIGHT * s;
    let off_x = (win_w - canvas_w) / 2.0;
    let off_y = (win_h - canvas_h) / 2.0;
    let pos = Vec2::new((cursor.x - off_x) / s, (cursor.y - off_y) / s);

    if active.0.is_empty() {
        // No hotspots declared — whole screen advances (default VN behavior).
        let _ = writer.write(AdvanceEvent { source: AdvanceSource::UserInput });
        return;
    }
    let hit = active.0.iter().any(|h| {
        pos.x >= h.x && pos.x <= h.x + h.width && pos.y >= h.y && pos.y <= h.y + h.height
    });
    if hit {
        let _ = writer.write(AdvanceEvent { source: AdvanceSource::UserInput });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::WindowResolution;

    #[derive(Resource, Default)]
    struct AdvanceCount(usize);

    fn count_advances(mut r: MessageReader<AdvanceEvent>, mut c: ResMut<AdvanceCount>) {
        for _ in r.read() {
            c.0 += 1;
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveHotspots>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<UiCanvasScale>();
        app.init_resource::<SaveLoadMode>();
        app.init_resource::<AdvanceCount>();
        app.add_message::<AdvanceEvent>();
        app.add_message::<HotspotEvent>();
        app.add_message::<HotspotClearEvent>();
        app.add_systems(Update, (apply_hotspot_events, hotspot_click_input, count_advances).chain());
        app
    }

    fn add_window(app: &mut App, x: f32, y: f32) -> Entity {
        let mut window = Window::default();
        window.focused = true;
        window.resolution = WindowResolution::new(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32);
        window.set_cursor_position(Some(Vec2::new(x, y)));
        app.world_mut().spawn(window).id()
    }

    fn click(app: &mut App) {
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
    }

    fn advance_count(app: &App) -> usize {
        app.world().resource::<AdvanceCount>().0
    }

    fn add_hotspot(app: &mut App, x: f32, y: f32, w: f32, h: f32) {
        let mut writer = app.world_mut().resource_mut::<Messages<HotspotEvent>>();
        writer.write(HotspotEvent { id: "t".into(), x, y, width: w, height: h });
    }

    #[test]
    fn no_hotspot_advances_anywhere() {
        let mut app = make_app();
        add_window(&mut app, 100.0, 100.0);
        click(&mut app);
        app.update();
        assert_eq!(advance_count(&app), 1);
    }

    #[test]
    fn click_inside_hotspot_advances() {
        let mut app = make_app();
        add_window(&mut app, 100.0, 100.0);
        add_hotspot(&mut app, 0.0, 0.0, 200.0, 200.0);
        click(&mut app);
        app.update();
        assert_eq!(advance_count(&app), 1);
    }

    #[test]
    fn click_outside_hotspot_ignored() {
        let mut app = make_app();
        add_window(&mut app, 300.0, 300.0);
        add_hotspot(&mut app, 0.0, 0.0, 200.0, 200.0);
        click(&mut app);
        app.update();
        assert_eq!(advance_count(&app), 0);
    }

    #[test]
    fn hotspot_clear_restores_fullscreen_advance() {
        let mut app = make_app();
        add_window(&mut app, 300.0, 300.0);
        add_hotspot(&mut app, 0.0, 0.0, 200.0, 200.0);
        app.update(); // apply_hotspot_events consumes HotspotEvent
        app.world_mut().resource_mut::<Messages<HotspotClearEvent>>().write(HotspotClearEvent);
        app.update(); // applies clear, then click should advance
        click(&mut app);
        app.update();
        assert_eq!(advance_count(&app), 1);
    }

    #[test]
    fn scaled_window_converts_coords() {
        let mut app = make_app();
        app.world_mut().resource_mut::<UiCanvasScale>().0 = 2.0;
        let mut window = Window::default();
        window.focused = true;
        window.resolution = WindowResolution::new(1920, 1080);
        window.set_cursor_position(Some(Vec2::new(300.0, 300.0)));
        app.world_mut().spawn(window);
        add_hotspot(&mut app, 0.0, 0.0, 200.0, 200.0);
        app.update(); // apply_hotspot_events consumes HotspotEvent
        click(&mut app);
        app.update();
        assert_eq!(advance_count(&app), 1);
    }
}
