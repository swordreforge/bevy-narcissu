//! Map 水仙10周年 `.ast` scripts to the new bevy-vn-engine `ScriptCmd` IR.
//!
//! .ast uses named attributes (unlike asb's positional attrs). Verified facts:
//! - blocks are a strictly linear chain (block[i+1] == linknext[i]) — emit in order,
//!   insert a `Label` for every block + every named label-table entry.
//! - `{"text"}` command carries no attrs; dialog text lives in the block-level
//!   `text.ja` field, voice in `text.vo` (`{"vo", ch="li", file="n002"}`).
//! - `bg path=":bg/"` → SetBg; `bg path=":fg/"` (with id/x/y) → Sprite (FG layer).
//! - `fg mode=-2` → clear all FG layers (HideFg all).
//! - `bgm file="bgm102"` → PlayBgm with id "102" (assets reorganized to audio/bgm/bgm102.ogg).
//! - `excall file="artemis脚本/1+2合集/nar1_00" call=1` → CallScript("nar1_00").
//! - `eval exp="g.chap01=1"` → SetFlag("chap01", 1); `exreturn` → Return.
//!
//! Unsupported-but-known tags (msgoff, タイトル, brandlogo, keyskip, msg) become
//! `Custom` so the info survives conversion and a game plugin may act on them.

use crate::ast::{AstCommand, AstLabelRef, AstScript, AstTextField};
use bevy_vn_core::script::cmd::{ScriptCmd, VnScript, ScriptMeta, ScriptVersion};
use std::collections::{HashMap, HashSet};

/// Map a whole .ast script. `script_name` is the output script id (e.g. "nar1_00").
pub fn map_ast_script(ast: &AstScript, script_name: &str, verbose: bool) -> VnScript {
    let mut instructions = Vec::new();

    for block in &ast.blocks {
        instructions.push(ScriptCmd::Label { name: block.name.clone() });
        let text = block.text_field.as_ref();
        for cmd in &block.commands {
            if let Some(mapped) = map_command(cmd, text) {
                instructions.push(mapped);
            } else if verbose {
                eprintln!("  [skip] {}.{}", block.name, cmd.tag);
            }
        }
    }

    // Named entry points from the label table, appended right after their block label.
    let block_names: HashSet<&str> = ast.blocks.iter().map(|b| b.name.as_str()).collect();
    let mut named: Vec<(String, AstLabelRef)> = ast.labels.clone().into_iter().collect();
    named.sort_by_key(|(_, r)| (block_index(&r.block, ast), r.label));

    let mut out = Vec::new();
    for cmd in &instructions {
        out.push(cmd.clone());
        if let ScriptCmd::Label { name } = cmd {
            let pending: Vec<String> = named
                .iter()
                .filter(|(_, r)| r.block == *name)
                .filter(|(n, _)| !block_names.contains(n.as_str()))
                .map(|(n, _)| n.clone())
                .collect();
            for n in pending {
                out.push(ScriptCmd::Label { name: n });
            }
        }
    }

    let has_terminal = out.iter().any(|c| matches!(c, ScriptCmd::Halt | ScriptCmd::Return));
    if !has_terminal {
        out.push(ScriptCmd::Halt);
    }

    VnScript {
        version: ScriptVersion::V1,
        meta: ScriptMeta { name: Some(script_name.to_string()), next_script: None },
        instructions: out,
    }
}

fn block_index(name: &str, ast: &AstScript) -> usize {
    ast.blocks.iter().position(|b| b.name == name).unwrap_or(usize::MAX)
}

fn map_command(cmd: &AstCommand, text: Option<&AstTextField>) -> Option<ScriptCmd> {
    match cmd.tag.as_str() {
        "text" => {
            let mut voice = None;
            let mut dialog = String::new();
            if let Some(tf) = text {
                if let Some(v) = tf.vo.first() {
                    voice = Some(if v.ch.is_empty() {
                        v.file.clone()
                    } else {
                        format!("{}/{}", v.ch, v.file)
                    });
                }
                dialog = tf.ja.join("\n");
            }
            if dialog.trim().is_empty() {
                if let Some(jp) = cmd.attrs.get("jp").or_else(|| cmd.attrs.get("text")) {
                    dialog = jp.clone();
                }
            }
            if dialog.trim().is_empty() {
                return None;
            }
            Some(ScriptCmd::Dialogue { speaker: None, text: dialog, voice })
        }
        "bg" => {
            let path = cmd.attrs.get("path").cloned().unwrap_or_default();
            let image = cmd.attrs.get("file").cloned()?;
            if path.contains("fg") || path.contains("obj") {
                let id = cmd.attrs.get("id").cloned().unwrap_or_else(|| "fg0".into());
                let x: f32 = cmd.attrs.get("x").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let y: f32 = cmd.attrs.get("y").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                Some(ScriptCmd::Sprite { id, image, x, y, anchor_x: None, anchor_y: None, z: None })
            } else if path.contains("ev") {
                Some(ScriptCmd::ShowCg { image, transition: None })
            } else {
                Some(ScriptCmd::SetBg { image, transition: None })
            }
        }
        "fg" => {
            let mode = cmd.attrs.get("mode").map(|s| s.as_str()).unwrap_or("0");
            if mode == "-2" {
                Some(ScriptCmd::HideFg { char_id: "all".into(), transition: None })
            } else {
                None
            }
        }
        "bgm" => {
            if cmd.attrs.get("stop").map(|s| s == "1").unwrap_or(false) {
                let fade_ms = cmd.attrs.get("time").and_then(|s| s.parse::<u64>().ok());
                return Some(ScriptCmd::StopBgm { fade_ms });
            }
            let file = cmd.attrs.get("file").cloned()?;
            let id = file.strip_prefix("bgm").unwrap_or(&file).to_string();
            let volume = cmd
                .attrs
                .get("vol")
                .and_then(|s| s.parse::<f32>().ok())
                .map(|v| (v / 255.0).clamp(0.0, 1.0));
            let fade_ms = cmd.attrs.get("time").and_then(|s| s.parse::<u64>().ok());
            Some(ScriptCmd::PlayBgm { id, volume, fade_ms })
        }
        "se" => {
            if cmd.attrs.get("stop").map(|s| s == "1").unwrap_or(false) {
                let channel = cmd.attrs.get("id").and_then(|s| s.parse().ok());
                return Some(ScriptCmd::StopSe { channel });
            }
            let file = cmd.attrs.get("file").cloned()?;
            let volume = cmd
                .attrs
                .get("vol")
                .and_then(|s| s.parse::<f32>().ok())
                .map(|v| (v / 255.0).clamp(0.0, 1.0));
            let channel = cmd.attrs.get("id").and_then(|s| s.parse().ok());
            Some(ScriptCmd::PlaySe { file, channel, volume })
        }
        "ex" => {
            let time_ms = cmd.attrs.get("time").and_then(|s| s.parse().ok()).unwrap_or(500);
            Some(ScriptCmd::Wait { time_ms })
        }
        "exreturn" => Some(ScriptCmd::Return),
        "excall" => {
            let file = cmd.attrs.get("file").cloned();
            let label = cmd.attrs.get("label").cloned();
            if let Some(f) = file {
                let script = f.rsplit('/').next().unwrap_or(&f).to_string();
                Some(ScriptCmd::CallScript { script, label })
            } else if let Some(l) = label {
                Some(ScriptCmd::Call { label: l })
            } else {
                None
            }
        }
        "eval" => {
            let exp = cmd.attrs.get("exp").cloned().unwrap_or_default();
            if let Some((key, value)) = parse_global_assign(&exp) {
                Some(ScriptCmd::SetFlag { key, value })
            } else {
                Some(ScriptCmd::Custom { tag: format!("eval:{}", exp), data: HashMap::new() })
            }
        }
        "cgdel" => Some(ScriptCmd::HideCg { transition: None }),
        "savetitle" => {
            let id = cmd.attrs.get("text").cloned().unwrap_or_else(|| "save".into());
            Some(ScriptCmd::SavePoint { id })
        }
        "user" => Some(ScriptCmd::SavePoint { id: "autosave".into() }),
        "msgoff" | "msgon" | "タイトル" | "brandlogo" | "keyskip" | "msg" => {
            Some(ScriptCmd::Custom { tag: cmd.tag.clone(), data: cmd.attrs.clone() })
        }
        _ => None,
    }
}

/// Parse `g.chap01=1` style Lua global assignments → (key, value).
fn parse_global_assign(exp: &str) -> Option<(String, i32)> {
    let exp = exp.trim();
    if exp.contains("if ") || exp.contains("then") || exp.contains("collectgarbage") {
        return None;
    }
    let (lhs, rhs) = exp.split_once('=')?;
    let key = lhs.trim().trim_start_matches("g.").to_string();
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    let value = rhs.trim().trim_end_matches(',').parse::<i32>().ok()?;
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AstBlock, AstVoice};

    fn cmd(tag: &str, attrs: Vec<(&str, &str)>) -> AstCommand {
        let mut map = HashMap::new();
        for (k, v) in attrs {
            map.insert(k.to_string(), v.to_string());
        }
        AstCommand { tag: tag.to_string(), attrs: map }
    }

    fn tf(ja: Vec<&str>, vo: Vec<(&str, &str)>) -> AstTextField {
        AstTextField {
            pagebreak: true,
            ja: ja.iter().map(|s| s.to_string()).collect(),
            vo: vo.iter().map(|(c, f)| AstVoice { ch: c.to_string(), file: f.to_string() }).collect(),
        }
    }

    #[test]
    fn bg_with_bg_path_is_setbg() {
        let c = cmd("bg", vec![("path", ":bg/"), ("file", "sora_ame01"), ("time", "800")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::SetBg { image, transition: None } if image == "sora_ame01"));
    }

    #[test]
    fn bg_with_fg_path_is_sprite() {
        let c = cmd("bg", vec![("path", ":fg/"), ("file", "s_akar_01"), ("x", "420"), ("y", "-20")]);
        let r = map_command(&c, None).unwrap();
        match &r {
            ScriptCmd::Sprite { id, image, x, y, .. } => {
                assert_eq!(id, "fg0");
                assert_eq!(image, "s_akar_01");
                assert_eq!(*x, 420.0);
                assert_eq!(*y, -20.0);
            }
            other => panic!("expected Sprite, got {:?}", other),
        }
    }

    #[test]
    fn bg_with_ev_path_is_showcg() {
        let c = cmd("bg", vec![("path", ":ev/"), ("file", "cg_ts01_02"), ("set", "ts01")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::ShowCg { image, transition: None } if image == "cg_ts01_02"));
    }

    #[test]
    fn fg_mode_minus2_hides_all() {
        let c = cmd("fg", vec![("mode", "-2")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::HideFg { char_id, .. } if char_id == "all"));
    }

    #[test]
    fn bgm_maps_id_without_prefix() {
        let c = cmd("bgm", vec![("file", "bgm102"), ("vol", "200"), ("time", "500")]);
        let r = map_command(&c, None).unwrap();
        match &r {
            ScriptCmd::PlayBgm { id, volume, fade_ms } => {
                assert_eq!(id, "102");
                assert!((volume.unwrap() - 0.784).abs() < 0.01);
                assert_eq!(*fade_ms, Some(500));
            }
            other => panic!("expected PlayBgm, got {:?}", other),
        }
    }

    #[test]
    fn se_stop_maps_channel() {
        let c = cmd("se", vec![("stop", "1"), ("id", "2")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::StopSe { channel: Some(2) }));
    }

    #[test]
    fn se_loop_maps_playse_channel() {
        let c = cmd("se", vec![("file", "rain"), ("loop", "1"), ("id", "3")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::PlaySe { file, channel: Some(3), .. } if file == "rain"));
    }

    #[test]
    fn text_uses_block_field_with_voice() {
        let c = cmd("text", vec![]);
        let t = tf(vec!["「こんにちは」"], vec![("li", "n002")]);
        let r = map_command(&c, Some(&t)).unwrap();
        match &r {
            ScriptCmd::Dialogue { speaker: None, text, voice } => {
                assert_eq!(text, "「こんにちは」");
                assert_eq!(voice.as_deref(), Some("li/n002"));
            }
            other => panic!("expected Dialogue, got {:?}", other),
        }
    }

    #[test]
    fn text_inline_jp_form() {
        let c = cmd("text", vec![("jp", "选择要开始的故事：")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Dialogue { text, .. } if text == "选择要开始的故事："));
    }

    #[test]
    fn excall_file_maps_callscript() {
        let c = cmd("excall", vec![("file", "artemis脚本/1+2合集/nar1_00"), ("call", "1")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::CallScript { script, label: None } if script == "nar1_00"));
    }

    #[test]
    fn excall_label_maps_call() {
        let c = cmd("excall", vec![("label", "story01")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Call { label } if label == "story01"));
    }

    #[test]
    fn eval_global_assign() {
        let c = cmd("eval", vec![("exp", "g.chap01=1")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::SetFlag { key, value } if key == "chap01" && *value == 1));
    }

    #[test]
    fn eval_complex_is_custom() {
        let c = cmd("eval", vec![("exp", "if game.os == 'vita' then collectgarbage('step', 200) end")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Custom { .. }));
    }

    #[test]
    fn ex_maps_wait() {
        let c = cmd("ex", vec![("time", "1500"), ("func", "wait")]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Wait { time_ms: 1500 }));
    }

    #[test]
    fn exreturn_maps_return() {
        let c = cmd("exreturn", vec![]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Return));
    }

    #[test]
    fn unsupported_tags_become_custom() {
        let c = cmd("msgoff", vec![]);
        let r = map_command(&c, None).unwrap();
        assert!(matches!(&r, ScriptCmd::Custom { tag, .. } if tag == "msgoff"));
    }

    #[test]
    fn full_script_linear_chain_with_labels() {
        let ast = AstScript {
            blocks: vec![
                AstBlock {
                    name: "block_00000".into(),
                    commands: vec![cmd("savetitle", vec![("text", "水仙1-0")]), cmd("ex", vec![("time", "400")])],
                    text_field: None,
                    linkback: None,
                    linknext: Some("block_00001".into()),
                    line: None,
                },
                AstBlock {
                    name: "block_00001".into(),
                    commands: vec![cmd("exreturn", vec![])],
                    text_field: None,
                    linkback: Some("block_00000".into()),
                    linknext: None,
                    line: None,
                },
            ],
            labels: HashMap::new(),
        };
        let out = map_ast_script(&ast, "nar1_00", false);
        let cmds = &out.instructions;
        assert_eq!(cmds.len(), 5);
        assert!(matches!(&cmds[0], ScriptCmd::Label { name } if name == "block_00000"));
        assert!(matches!(&cmds[1], ScriptCmd::SavePoint { id } if id == "水仙1-0"));
        assert!(matches!(&cmds[4], ScriptCmd::Return));
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use crate::ast::{AstBlock, AstVoice};

    fn cmd(tag: &str, attrs: Vec<(&str, &str)>) -> AstCommand {
        let mut map = HashMap::new();
        for (k, v) in attrs {
            map.insert(k.to_string(), v.to_string());
        }
        AstCommand { tag: tag.to_string(), attrs: map }
    }

    fn tf(ja: Vec<&str>, vo: Vec<(&str, &str)>) -> AstTextField {
        AstTextField {
            pagebreak: true,
            ja: ja.iter().map(|s| s.to_string()).collect(),
            vo: vo.iter().map(|(c, f)| AstVoice { ch: c.to_string(), file: f.to_string() }).collect(),
        }
    }

    #[test]
    fn mapped_script_roundtrips_through_ron() {
        let ast = AstScript {
            blocks: vec![
                AstBlock {
                    name: "block_00000".into(),
                    commands: vec![
                        cmd("bg", vec![("path", ":bg/"), ("file", "sora_ame01"), ("time", "800")]),
                        cmd("bgm", vec![("file", "bgm102"), ("vol", "200"), ("time", "500")]),
                        cmd("text", vec![]),
                    ],
                    text_field: Some(tf(vec!["「こんにちは」"], vec![("li", "n002")])),
                    linkback: None,
                    linknext: None,
                    line: None,
                },
            ],
            labels: HashMap::new(),
        };
        let vn = map_ast_script(&ast, "test_roundtrip", false);
        let ron = ron::ser::to_string_pretty(&vn, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: VnScript = ron::de::from_str(&ron).unwrap();
        assert_eq!(parsed.instructions.len(), vn.instructions.len());
        assert!(matches!(parsed.instructions[1], ScriptCmd::SetBg { .. }));
        assert!(matches!(parsed.instructions[2], ScriptCmd::PlayBgm { .. }));
        assert!(matches!(&parsed.instructions[3], ScriptCmd::Dialogue { voice: Some(v), .. } if v == "li/n002"));
    }
}
