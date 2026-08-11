//! Shared single-track / multi-slot audio channel skeleton.
//!
//! BGM, SE and voice managers are the same play/stop/volume loop; they only
//! differ in path prefix, `PlaybackMode`, the event field carrying the file
//! id, the slot selection (SE routes on `channel`, BGM/voice use slot 0) and
//! whether the played id is tracked (`BgmManager::current_id`). The
//! [`audio_channel_impl!`] macro expands the manager struct, `Default` and
//! the three message handlers from one compact call site.

/// Generates an `AudioChannel`-style manager resource plus its message
/// handlers. Every call site expands to the same shape; the parameters pin
/// down the per-channel differences:
/// - `channels`: slot count (BGM/voice 1, SE 8)
/// - `path`: asset directory prefix, e.g. `"audio/bgm/"`
/// - `mode`: `PlaybackMode` for spawned players
/// - `file`: `|event| ...` extracting the file-id string from the play event
/// - `slot`: `|event| ...` picking the entity slot to replace
/// - `volume`: `|event| ...` extracting this channel's `SetVolumeEvent` field
/// - `track` (optional): `|event| ...` returning `Option<String>` stored in
///   `current_id` (BGM only)
/// - `stop` (optional): stop event type + `|event| ...` slot selector; `None`
///   clears every slot
/// - `handle`: `|event, queue, server, path: String| ...` returning the
///   `AudioSource` handle (voice consults its preload queue first, others
///   just load)
macro_rules! audio_channel_impl {
    (
        pub struct $name:ident;
        channels: $channels:expr,
        path: $path:literal,
        mode: $mode:expr,
        play: $play_ev:ty,
        file: $file:expr,
        slot: $slot:expr,
        volume: $vol:expr,
        $(track: $track:expr,)?
        $(stop: $stop_ev:ty, stop_slot: $stop_slot:expr,)?
        handle: $handle:expr,
    ) => {
        #[derive(Resource)]
        pub struct $name {
            pub entities: Vec<Option<Entity>>,
            pub current_id: Option<String>,
            pub volume: f32,
        }

        impl Default for $name {
            fn default() -> Self {
                Self { entities: vec![None; $channels], current_id: None, volume: 1.0 }
            }
        }

        fn handle_play(
            mut reader: MessageReader<$play_ev>,
            mut commands: Commands,
            asset_server: Res<AssetServer>,
            queue: Option<Res<crate::voice::VoicePreloadQueue>>,
            mut mgr: ResMut<$name>,
        ) {
            for event in reader.read() {
                let slot = ($slot)(&event);
                if let Some(e) = mgr.entities[slot] { commands.entity(e).try_despawn(); }
                let path = format!("{}{}.ogg", $path, ($file)(&event));
                let vol = event.volume.unwrap_or(mgr.volume.max(0.01));
                let handle = ($handle)(&event, &queue, &asset_server, path);
                mgr.entities[slot] = Some(commands.spawn((
                    AudioPlayer(handle),
                    PlaybackSettings { mode: $mode, volume: Volume::Linear(vol), ..default() },
                )).id());
                $(mgr.current_id = ($track)(&event);)?
            }
        }

        $(fn handle_stop(
            mut reader: MessageReader<$stop_ev>,
            mut commands: Commands,
            mut mgr: ResMut<$name>,
        ) {
            for event in reader.read() {
                match ($stop_slot)(&event) {
                    Some(idx) => {
                        if let Some(e) = mgr.entities.get_mut(idx).and_then(Option::take) {
                            commands.entity(e).try_despawn();
                        }
                    }
                    None => {
                        for slot in mgr.entities.iter_mut() {
                            let slot_ent: Option<Entity> = slot.take();
                            if let Some(e) = slot_ent { commands.entity(e).try_despawn(); }
                        }
                        mgr.current_id = None;
                    }
                }
            }
        })?

        fn handle_volume(
            mut reader: MessageReader<SetVolumeEvent>,
            mut mgr: ResMut<$name>,
            mut q_sink: Query<&mut AudioSink>,
        ) {
            for event in reader.read() {
                if let Some(vol) = ($vol)(&event) {
                    mgr.volume = vol;
                    for slot in mgr.entities.iter().flatten() {
                        if let Ok(mut sink) = q_sink.get_mut(*slot) { sink.set_volume(Volume::Linear(vol)); }
                    }
                }
            }
        }
    };
}
pub(crate) use audio_channel_impl;
