//! artemis-converter — Converts Artemis engine .asb/.iet scripts to .vnscript.ron.
//!
//! Usage: artemis-converter --input <dir> --output <dir>
//!
//! Scans `<input>/scenario/*.asb` and `<input>/scenario/*.iet`,
//! converts them to `<output>/*.vnscript.ron`.

use bevy_vn_core::script::cmd::{
    FgPosition, ScreenEffectKind, ScriptCmd, ScriptMeta, ScriptVersion, VnScript,
};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// ── CLI ──

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "game_data")]
    input: PathBuf,
    #[arg(long, default_value = "assets/scripts")]
    output: PathBuf,
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    fs::create_dir_all(&args.output)?;

    let scenario_dir = args.input.join("scenario");
    if !scenario_dir.is_dir() {
        eprintln!("scenario/ not found in input dir. Expected: {}", scenario_dir.display());
        std::process::exit(1);
    }

    // ── Convert .asb files ──
    for entry in fs::read_dir(&scenario_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "asb") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            if args.verbose { eprintln!("Converting ASB: {name}"); }
            let raw = fs::read(&path)?;
            let cmds = parse_asb(&raw);
            let script = VnScript {
                version: ScriptVersion::V1,
                meta: ScriptMeta { name: Some(name.into()), next_script: None },
                instructions: cmds,
            };
            let ron = ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::default())?;
            let out_path = args.output.join(format!("{name}.vnscript.ron"));
            fs::write(out_path, ron)?;
        }
    }

    // ── Convert .iet files ──
    for entry in fs::read_dir(&scenario_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "iet") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            let is_main = name.eq_ignore_ascii_case("main");
            if args.verbose { eprintln!("Converting IET: {name}"); }
            let cmds = parse_iet(&path);
            let script = VnScript {
                version: ScriptVersion::V1,
                meta: ScriptMeta { name: Some(name.into()), next_script: None },
                instructions: cmds,
            };
            let ron = ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::default())?;
            let out_name = if is_main { "main.vnscript.ron".into() } else { format!("{name}.vnscript.ron") };
            fs::write(args.output.join(out_name), ron)?;
        }
    }

    println!("Done — output in {}", args.output.display());
    Ok(())
}

// ── ASB binary parser ──

fn read_u32_le(data: &[u8], pos: &mut usize) -> u32 {
    let v = u32::from_le_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    v
}

fn read_string(data: &[u8], pos: &mut usize) -> String {
    let len = read_u32_le(data, pos) as usize;
    let s = String::from_utf8_lossy(&data[*pos..*pos + len]).to_string();
    *pos += len + 1; // skip null terminator
    s
}

fn parse_asb(raw: &[u8]) -> Vec<ScriptCmd> {
    let mut cmds = Vec::new();
    if raw.len() < 9 { return cmds; }
    if &raw[0..4] != b"ASB\0" { return cmds; }
    let mut pos = 5;
    let count = read_u32_le(raw, &mut pos) as usize;

    let mut label = String::new();
    for _ in 0..count {
        let item_type = raw[pos]; pos += 1;
        match item_type {
            1 => {
                // Label
                label = read_string(raw, &mut pos);
                cmds.push(ScriptCmd::Label { name: label.clone() });
            }
            0 => {
                // Command
                let tag = read_string(raw, &mut pos);
                let _line = read_u32_le(raw, &mut pos);
                let attr_count = read_u32_le(raw, &mut pos) as usize;
                let mut attrs: HashMap<String, String> = HashMap::new();
                for _ in 0..attr_count {
                    let key = read_string(raw, &mut pos);
                    let val = read_string(raw, &mut pos);
                    attrs.insert(key, val);
                }
                if let Some(cmd) = map_tag(&tag, &attrs) {
                    cmds.push(cmd);
                }
            }
            _ => {}
        }
    }
    cmds
}

// ── IET text parser ──

fn parse_iet(path: &Path) -> Vec<ScriptCmd> {
    let mut cmds = Vec::new();
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return cmds,
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => continue };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if let Some(label) = trimmed.strip_prefix('*') {
            cmds.push(ScriptCmd::Label { name: label.to_string() });
        } else if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let parts: Vec<&str> = inner.splitn(2, char::is_whitespace).collect();
            let tag = parts[0].trim();
            let rest = if parts.len() > 1 { parts[1] } else { "" };
            let mut attrs = HashMap::new();
            // Simple key=value parsing
            for kv in rest.split_whitespace() {
                if let Some((k, v)) = kv.split_once('=') {
                    let v = v.trim_matches('"');
                    attrs.insert(k.to_string(), v.to_string());
                } else {
                    attrs.insert("arg".to_string(), kv.to_string());
                }
            }
            if let Some(cmd) = map_iet_tag(tag, &attrs) {
                cmds.push(cmd);
            }
        }
    }
    cmds
}

// ── Tag mapping ──

fn map_tag(tag: &str, attrs: &HashMap<String, String>) -> Option<ScriptCmd> {
    let a = |i: &str| attrs.get(i).cloned().unwrap_or_default();
    match tag {
        "Tati" | "TatiFa" => Some(ScriptCmd::ShowFg {
            char_id: a("id"), expression: a("face"),
            position: FgPosition::Center, transition: None,
        }),
        "ClrTati" => Some(ScriptCmd::HideFg { char_id: a("id"), transition: None }),
        "Face" => Some(ScriptCmd::ShowFace { char_id: a("id"), expression: a("face") }),
        "Back" | "set_bg" => Some(ScriptCmd::SetBg { image: a("0"), transition: None }),
        "Event" | "EventMN" => Some(ScriptCmd::ShowCg { image: a("0"), transition: None }),
        "ClrEvent" => Some(ScriptCmd::HideCg { transition: None }),
        "BgmPlay" | "bgm_play" => Some(ScriptCmd::PlayBgm { id: a("0"), volume: None, fade_ms: None }),
        "BgmStop" => Some(ScriptCmd::StopBgm { fade_ms: None }),
        "SEPlay" | "se_play" => Some(ScriptCmd::PlaySe {
            file: a("0"), channel: a("ch").parse().ok(), volume: None,
        }),
        "Voice" => Some(ScriptCmd::PlayVoice { file: a("0"), volume: None }),
        "Text" | "Txt" | "Message" => Some(ScriptCmd::Dialogue {
            speaker: attrs.get("name").cloned(),
            text: a("0"),
            voice: None,
        }),
        "Fadeout" | "FadeIn" | "Blackout" => Some(ScriptCmd::ScreenEffect {
            kind: ScreenEffectKind::Fade,
            color: Some("Black".into()), duration_ms: 1000,
        }),
        "FadeFilm" => Some(ScriptCmd::ScreenEffect {
            kind: ScreenEffectKind::Fade, color: Some("0".into()), duration_ms: 500,
        }),
        "Quake" | "Jishin" => Some(ScriptCmd::Shake {
            intensity: a("0").parse().unwrap_or(10.0), duration_frames: 30,
        }),
        "Wait" | "wait" => Some(ScriptCmd::Wait {
            time_ms: a("0").parse().unwrap_or(1000),
        }),
        "Jump" => Some(ScriptCmd::Jump { label: a("0") }),
        "Call" => Some(ScriptCmd::Call { label: a("0") }),
        "CallScript" => Some(ScriptCmd::CallScript { script: a("0"), label: None }),
        "Return" | "return" => Some(ScriptCmd::Return),
        "Halt" => Some(ScriptCmd::Halt),
        "SavePoint" | "quicksave" => Some(ScriptCmd::SavePoint { id: a("0") }),
        "RouteFlag" => Some(ScriptCmd::RouteFlag { route_key: a("0") }),
        "PlayMovie" => Some(ScriptCmd::PlayMovie { file: a("0"), blocking: true }),
        "StopMovie" => Some(ScriptCmd::StopMovie),
        _ => {
            // Log unmapped tags
            eprintln!("  unmapped tag: {tag}");
            Some(ScriptCmd::Custom {
                tag: tag.to_string(),
                data: attrs.clone(),
            })
        }
    }
}

fn map_iet_tag(tag: &str, attrs: &HashMap<String, String>) -> Option<ScriptCmd> {
    let a = |i: &str| attrs.get(i).cloned().unwrap_or_default();
    match tag {
        "CallScript" => Some(ScriptCmd::CallScript {
            script: a("arg"), label: attrs.get("label").cloned(),
        }),
        "return" => Some(ScriptCmd::Return),
        "if" => {
            let expr = attrs.get("estimate").cloned().unwrap_or_default();
            Some(ScriptCmd::Condition {
                expression: expr, goto_true: a("true"), goto_false: attrs.get("false").cloned(),
            })
        }
        "affection_change" => Some(ScriptCmd::Custom {
            tag: "affection_change".into(), data: attrs.clone(),
        }),
        _ => map_tag(tag, attrs),
    }
}
