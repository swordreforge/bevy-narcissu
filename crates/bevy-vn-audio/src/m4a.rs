//! M4A / MP4 audio support — including xHE-AAC (USAC).
//!
//! Two pieces are needed, and Bevy 0.19 provides neither out of the box:
//!
//! 1. **Container demuxing** — extract the `AudioSpecificConfig` (ASC) from
//!    `esds` and the raw AAC access units from `mdat`. Done with symphonia's
//!    pure-Rust `isomp4` demuxer.
//! 2. **Decoding** — AAC-LC / HE-AAC / HE-AACv2 / USAC(xHE-AAC). Done with
//!    [`fdk_aac_rs`], a bit-exact pure-Rust port of Fraunhofer FDK-AAC 2.0.3.
//!    `symphonia`/rodio only decode AAC-LC and reject xHE-AAC (`mp4a.40.42`).
//!
//! FDK's `TT_MP4_RAW` transport consumes exactly the (ASC, access unit) pair
//! that MP4 gives us, so the playback path is:
//!
//! ```text
//! .m4a bytes ──symphonia isomp4──> ASC + [access unit]
//!                                        │
//!                          fdk-aac-rs Decoder (raw)
//!                                        │
//!                                  i16 PCM → f32 Source
//! ```
//!
//! Everything is pure Rust, so this compiles for native and
//! `wasm32-unknown-unknown`, mirroring `bevy-opus-audio`.
//!
//! License note: the underlying FDK AAC port is distributed under the
//! Fraunhofer FDK AAC license (see the `fdk-aac-rs` crate's `LICENSE.txt`);
//! it grants no patent rights. Redistributions must retain that license text.

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::audio::{AddAudioSource, ChannelCount, Decodable, SampleRate, Source};
use bevy::prelude::*;
use fdk_aac_rs::decoder::{DecodeFlags, Decoder, DecoderError};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// A loaded `.m4a` / `.mp4` file: the ASC plus every raw AAC access unit,
/// decoded lazily during playback.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct M4aAudio {
    /// `AudioSpecificConfig` from the MP4 `esds` box.
    asc: Arc<[u8]>,
    /// One raw AAC access unit per MP4 sample packet.
    units: Arc<[Box<[u8]>]>,
    sample_rate: u32,
    channels: u16,
    /// Frames per channel to drop at the start (`elst.media_time`), i.e. the
    /// encoder delay. Without this a looping BGM restarts with silence.
    skip_frames: u64,
    /// Frames per channel to emit (`elst.segment_duration`). Trims the
    /// encoder padding at the end so loops join cleanly.
    play_frames: Option<u64>,
    /// Samples per channel advertised by the container (fallback duration).
    total_frames: Option<u64>,
}

impl Decodable for M4aAudio {
    type Decoder = M4aSource;

    fn decoder(&self) -> Self::Decoder {
        M4aSource::new(self)
    }
}

/// Streaming decoder: one FDK raw access unit per `fill`, then one
/// `decode_frame`, yielding interleaved `f32` samples.
pub struct M4aSource {
    units: Arc<[Box<[u8]>]>,
    next_unit: usize,
    decoder: Decoder,
    pcm: Vec<i16>,
    pcm_len: usize,
    pos: usize,
    sample_rate: u32,
    channels: u16,
    /// Interleaved samples still to drop at the start.
    skip_samples: usize,
    /// Interleaved samples still to emit before stopping.
    remaining_samples: Option<usize>,
    total_duration: Option<Duration>,
    done: bool,
}

impl M4aSource {
    fn new(audio: &M4aAudio) -> Self {
        let decoder =
            Decoder::with_config(&audio.asc).expect("ASC validated when the asset was loaded");
        let channels = audio.channels as usize;
        let played = audio
            .play_frames
            .unwrap_or_else(|| audio.total_frames.unwrap_or(0));
        Self {
            units: audio.units.clone(),
            next_unit: 0,
            decoder,
            pcm: vec![0i16; 8 * 2048],
            pcm_len: 0,
            pos: 0,
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            skip_samples: audio.skip_frames as usize * channels,
            remaining_samples: audio.play_frames.map(|frames| frames as usize * channels),
            total_duration: (played > 0)
                .then(|| Duration::from_secs_f64(played as f64 / audio.sample_rate.max(1) as f64)),
            done: false,
        }
    }

    fn fill_from_next_unit(&mut self) -> bool {
        while self.next_unit < self.units.len() {
            let unit = &self.units[self.next_unit];
            self.next_unit += 1;

            if self.decoder.fill(unit).is_err() {
                continue;
            }
            match self
                .decoder
                .decode_frame(&mut self.pcm, DecodeFlags::empty())
            {
                Ok(frame) => {
                    // Copy the length before the borrow of `self.pcm` ends.
                    let n = frame.samples().len();
                    if n > 0 {
                        self.pcm_len = n;
                        self.pos = 0;
                        return true;
                    }
                }
                // Raw transport: a call without new input has no frame.
                Err(DecoderError::NotEnoughBits) => continue,
                // Concealed/corrupt frames are skipped; audio keeps playing.
                Err(_) => continue,
            }
        }
        self.done = true;
        false
    }
}

impl Iterator for M4aSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            // Drop the encoder delay (and anything before it).
            while self.skip_samples > 0 && self.pos < self.pcm_len {
                let take = (self.pcm_len - self.pos).min(self.skip_samples);
                self.pos += take;
                self.skip_samples -= take;
            }

            if self.remaining_samples == Some(0) {
                self.done = true;
                return None;
            }

            if self.pos < self.pcm_len {
                let sample = self.pcm[self.pos];
                self.pos += 1;
                if let Some(rem) = &mut self.remaining_samples {
                    *rem -= 1;
                }
                // FDK outputs interleaved i16; rodio samples are f32 in [-1, 1].
                return Some(sample as f32 / 32768.0);
            }

            if self.done || !self.fill_from_next_unit() {
                return None;
            }
        }
    }
}

impl Source for M4aSource {
    fn current_span_len(&self) -> Option<usize> {
        if (self.done && self.pos >= self.pcm_len) || self.remaining_samples == Some(0) {
            Some(0)
        } else {
            None
        }
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(self.channels).expect("validated at load")
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.sample_rate).expect("validated at load")
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}

// ── MP4 demuxing ──

struct ParsedM4a {
    asc: Box<[u8]>,
    units: Vec<Box<[u8]>>,
    sample_rate: u32,
    channels: u16,
    skip_frames: u64,
    play_frames: Option<u64>,
    total_frames: Option<u64>,
}

/// Iterate the ISO-BMFF boxes in `buf`, returning `(type, payload)`.
/// Handles 32-bit, 64-bit (`size == 1`) and "to end of file" (`size == 0`).
fn mp4_boxes(mut buf: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = Vec::new();
    while buf.len() >= 8 {
        let size32 = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64;
        let kind = [buf[4], buf[5], buf[6], buf[7]];
        let (size, header) = match size32 {
            0 => (buf.len() as u64, 8usize),
            1 if buf.len() >= 16 => {
                let size = u64::from_be_bytes([
                    buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
                ]);
                (size, 16)
            }
            n => (n, 8),
        };
        if size < header as u64 || size > buf.len() as u64 {
            break;
        }
        out.push((kind, &buf[header..size as usize]));
        buf = &buf[size as usize..];
    }
    out
}

fn mp4_child<'a>(buf: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    mp4_boxes(buf)
        .into_iter()
        .find(|(k, _)| k == kind)
        .map(|(_, payload)| payload)
}

/// `mvhd`/`mdhd` timescale (ticks per second).
fn mp4_timescale(buf: &[u8]) -> Option<u32> {
    let version = *buf.first()?;
    let offset = if version == 1 { 20 } else { 12 };
    let b = buf.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Read `edts/elst` for the audio track: `(media_time, segment_duration)`.
///
/// `media_time` is the encoder delay (frames to drop at the start) and
/// `segment_duration` is the intended playback length (frames), so the tail
/// encoder padding is trimmed too. This is what makes BGM loops seamless.
fn mp4_edit_list(bytes: &[u8]) -> Option<(u64, Option<u64>)> {
    let moov = mp4_child(bytes, b"moov")?;
    let movie_timescale = mp4_child(moov, b"mvhd").and_then(mp4_timescale);
    for (kind, trak) in mp4_boxes(moov) {
        if kind != *b"trak" {
            continue;
        }
        let Some(mdia) = mp4_child(trak, b"mdia") else {
            continue;
        };
        let Some(hdlr) = mp4_child(mdia, b"hdlr") else {
            continue;
        };
        // hdlr: version+flags (4), pre_defined (4), handler_type (4)
        if hdlr.len() < 12 || &hdlr[8..12] != b"soun" {
            continue;
        }
        let media_timescale = mp4_child(mdia, b"mdhd").and_then(mp4_timescale);
        let Some(edts) = mp4_child(trak, b"edts") else {
            continue;
        };
        let Some(elst) = mp4_child(edts, b"elst") else {
            continue;
        };
        if elst.len() < 8 {
            return None;
        }
        let version = elst[0];
        let count = u32::from_be_bytes([elst[4], elst[5], elst[6], elst[7]]) as usize;
        let entry_size = if version == 0 { 12 } else { 20 };
        for i in 0..count {
            let p = 8 + i * entry_size;
            if p + entry_size > elst.len() {
                break;
            }
            let (duration, media_time) = if version == 0 {
                (
                    u32::from_be_bytes([elst[p], elst[p + 1], elst[p + 2], elst[p + 3]]) as u64,
                    i32::from_be_bytes([elst[p + 4], elst[p + 5], elst[p + 6], elst[p + 7]]) as i64,
                )
            } else {
                (
                    u64::from_be_bytes([
                        elst[p],
                        elst[p + 1],
                        elst[p + 2],
                        elst[p + 3],
                        elst[p + 4],
                        elst[p + 5],
                        elst[p + 6],
                        elst[p + 7],
                    ]),
                    i64::from_be_bytes([
                        elst[p + 8],
                        elst[p + 9],
                        elst[p + 10],
                        elst[p + 11],
                        elst[p + 12],
                        elst[p + 13],
                        elst[p + 14],
                        elst[p + 15],
                    ]),
                )
            };
            // -1 marks an empty edit (a container-level delay); skip it.
            if media_time >= 0 {
                // `segment_duration` is in movie timescale; convert to media
                // frames (for audio the media timescale is the sample rate).
                let play_frames = match (movie_timescale, media_timescale) {
                    (Some(movie), Some(media)) if movie > 0 && duration > 0 => {
                        Some((duration as u128 * media as u128 / movie as u128) as u64)
                    }
                    _ => (duration > 0).then_some(duration),
                };
                return Some((media_time as u64, play_frames));
            }
        }
        return None;
    }
    None
}

/// Demux the default audio track of an MP4/M4A file.
///
/// Collects every access unit; a `.m4a` BGM is ~0.8 MB compressed, and the
/// decoded PCM is far larger, so the extra copy is negligible.
fn parse_mp4(bytes: Vec<u8>) -> Result<ParsedM4a, String> {
    let (skip_frames, play_frames) = mp4_edit_list(&bytes).unwrap_or((0, None));
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("m4a");

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("MP4 probe failed: {e}"))?;
    let mut format: Box<dyn FormatReader> = probed.format;

    let track = format
        .default_track()
        .ok_or_else(|| "MP4 has no default track".to_string())?;
    let track_id = track.id;
    let asc = track
        .codec_params
        .extra_data
        .clone()
        .ok_or_else(|| "MP4 audio track has no AudioSpecificConfig (esds)".to_string())?;
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let total_frames = track.codec_params.n_frames;

    let mut units: Vec<Box<[u8]>> = Vec::new();
    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => units.push(packet.data),
            Ok(_) => {}
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err("MP4 stream configuration changed mid-file".into());
            }
            Err(e) => return Err(format!("MP4 packet read failed: {e}")),
        }
    }

    if units.is_empty() {
        return Err("MP4 audio track contains no packets".into());
    }

    // Probe the real output format once: USAC may change the sample rate
    // (core 22.05 kHz → 44.1 kHz with eSBR) and the channel count is not in
    // the container metadata.
    let mut decoder =
        Decoder::with_config(&asc).map_err(|e| format!("FDK decoder rejected ASC: {e:?}"))?;
    let mut pcm = vec![0i16; 8 * 2048];
    let mut channels = 2u16;
    for unit in &units {
        if decoder.fill(unit).is_err() {
            continue;
        }
        match decoder.decode_frame(&mut pcm, DecodeFlags::empty()) {
            Ok(_) => {
                let info = decoder.stream_info();
                if info.sample_rate() > 0 {
                    sample_rate = info.sample_rate() as u32;
                }
                if info.num_channels() > 0 {
                    channels = info.num_channels() as u16;
                }
                break;
            }
            Err(DecoderError::NotEnoughBits) => continue,
            Err(_) => continue,
        }
    }

    Ok(ParsedM4a {
        asc,
        units,
        sample_rate,
        channels,
        skip_frames,
        play_frames,
        total_frames,
    })
}

/// [`AssetLoader`] for `.m4a` / `.mp4` files.
#[derive(TypePath)]
pub struct M4aAudioLoader;

impl AssetLoader for M4aAudioLoader {
    type Asset = M4aAudio;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let parsed = parse_mp4(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(M4aAudio {
            asc: parsed.asc.into(),
            units: parsed.units.into(),
            sample_rate: parsed.sample_rate,
            channels: parsed.channels,
            skip_frames: parsed.skip_frames,
            play_frames: parsed.play_frames,
            total_frames: parsed.total_frames,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["m4a", "mp4"]
    }
}

/// Registers [`M4aAudio`] as a decodable source and its loader.
pub struct M4aAudioPlugin;

impl Plugin for M4aAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<M4aAudio>()
            .register_asset_loader(M4aAudioLoader);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    /// `mvhd` / `mdhd`: version+flags, creation, modification, timescale, duration.
    fn timed_header(kind: &[u8; 4], timescale: u32) -> Vec<u8> {
        let mut payload = vec![0u8; 20];
        payload[12..16].copy_from_slice(&timescale.to_be_bytes());
        boxed(kind, &payload)
    }

    #[test]
    fn edit_list_gives_encoder_delay_and_play_length() {
        let mdia = boxed(
            b"mdia",
            &[
                timed_header(b"mdhd", 44_100),
                boxed(b"hdlr", &[0, 0, 0, 0, 0, 0, 0, 0, b's', b'o', b'u', b'n']),
            ]
            .concat(),
        );
        // elst v0: version/flags, entry_count, segment_duration, media_time, rate.
        let mut elst = vec![0u8; 8];
        elst[4..8].copy_from_slice(&1u32.to_be_bytes()); // one entry
        elst.extend_from_slice(&12_345u32.to_be_bytes()); // segment duration
        elst.extend_from_slice(&2_048i32.to_be_bytes()); // media_time = delay
        elst.extend_from_slice(&0x0001_0000i32.to_be_bytes()); // rate 1.0
        let elst = boxed(b"elst", &elst);

        let trak = boxed(b"trak", &[mdia, boxed(b"edts", &elst)].concat());
        let moov = boxed(b"moov", &[timed_header(b"mvhd", 44_100), trak].concat());
        let file = [boxed(b"ftyp", b"mp42"), moov].concat();

        assert_eq!(mp4_edit_list(&file), Some((2_048, Some(12_345))));
    }

    #[test]
    fn missing_edit_list_is_not_an_error() {
        let moov = boxed(b"moov", &timed_header(b"mvhd", 44_100));
        let file = [boxed(b"ftyp", b"mp42"), moov].concat();
        assert_eq!(mp4_edit_list(&file), None);
    }
}
