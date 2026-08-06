//! Parser for Artemis `.ast` scenario scripts (Lua-table format, 水仙10周年).
//!
//! Format (verified from game files):
//! ```lua
//! astver = 2.0
//! ast = {
//!     block_00000 = {
//!         {"savetitle", text="水仙1-0"},        -- command: {"tag", key=val, ...}
//!         {"bg", id=1, file="black", time=1500, path=":bg/"},
//!         {"text"},
//!         text = {                              -- block-level text field
//!             pagebreak = true,
//!             vo  = {{"vo", ch="li", file="n002"},},
//!             ja  = {{"「…」"},},
//!         },
//!         linkback = "block_00000",
//!         linknext = "block_00002",
//!         line = 98
//!     },
//!     label = {                                -- named entry points
//!         top = { block="block_00000", label=1 },
//!     },
//! }
//! ```
//! Blocks form a strictly linear chain (`block[i+1] == linknext[i]`), no branches.

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct AstScript {
    pub blocks: Vec<AstBlock>,
    pub labels: HashMap<String, AstLabelRef>,
}

#[derive(Debug, Clone)]
pub struct AstBlock {
    pub name: String,
    pub commands: Vec<AstCommand>,
    /// Block-level `text = { ... }` field (dialog text + voice refs).
    pub text_field: Option<AstTextField>,
    pub linkback: Option<String>,
    pub linknext: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AstCommand {
    pub tag: String,
    /// Named attributes (e.g. `id=1`, `file="x"`, `path=":bg/"`). Values kept as strings.
    pub attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AstTextField {
    pub pagebreak: bool,
    /// Dialog lines (ja = Japanese). Each entry is one string (may contain `\n`).
    pub ja: Vec<String>,
    /// Voice refs: `{"vo", ch="li", file="n002"}` → ch + file.
    pub vo: Vec<AstVoice>,
}

#[derive(Debug, Clone)]
pub struct AstVoice {
    pub ch: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct AstLabelRef {
    pub block: String,
    pub label: u32,
}

// ─── Line-based Lua table parser ──────────────────────────────────────────

#[derive(PartialEq)]
enum State {
    Top,
    InBlock,
    InTextField,
    InLabelTable,
}

pub fn parse_ast(path: &Path) -> Result<AstScript> {
    let content = std::fs::read_to_string(path)?;
    parse_ast_str(&content, &path.display().to_string())
}

pub fn parse_ast_str(content: &str, src_name: &str) -> Result<AstScript> {
    let mut script = AstScript::default();
    let mut state = State::Top;
    let mut cur_block: Option<AstBlock> = None;
    let mut cur_text: Option<AstTextField> = None;
    let mut block_stack: Vec<()> = Vec::new();

    for (lineno, raw) in content.lines().enumerate() {
        let line = strip_comment(raw);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let _ln = lineno + 1;

        match state {
            State::Top => {
                if line.starts_with("ast") && line.contains("=") && !line.starts_with("astver") {
                    if line.contains("{") {
                        block_stack.push(());
                        state = State::InBlock;
                    }
                }
            }
            State::InBlock => {
                if let Some(name) = parse_block_header(line) {
                    if let Some(b) = cur_block.take() {
                        script.blocks.push(b);
                    }
                    block_stack.push(());
                    cur_block = Some(AstBlock {
                        name,
                        commands: Vec::new(),
                        text_field: None,
                        linkback: None,
                        linknext: None,
                        line: None,
                    });
                    continue;
                }
                if line.starts_with("text") && line.contains("=") && line.contains("{") {
                    block_stack.push(());
                    cur_text = Some(AstTextField {
                        pagebreak: false,
                        ja: Vec::new(),
                        vo: Vec::new(),
                    });
                    state = State::InTextField;
                    continue;
                }
                if line.starts_with("label") && line.contains("=") && line.contains("{") {
                    if let Some(b) = cur_block.take() {
                        script.blocks.push(b);
                    }
                    block_stack.push(());
                    state = State::InLabelTable;
                    continue;
                }
                if let Some(v) = parse_quoted_assign(line, "linkback") {
                    if let Some(b) = cur_block.as_mut() { b.linkback = Some(v); }
                    continue;
                }
                if let Some(v) = parse_quoted_assign(line, "linknext") {
                    if let Some(b) = cur_block.as_mut() { b.linknext = Some(v); }
                    continue;
                }
                if let Some(v) = parse_num_assign(line, "line") {
                    if let Some(b) = cur_block.as_mut() { b.line = Some(v); }
                    continue;
                }
                if line.starts_with("{\"") {
                    if let Some(cmd) = parse_command(line) {
                        if let Some(b) = cur_block.as_mut() {
                            b.commands.push(cmd);
                        }
                    }
                    continue;
                }
                if line.starts_with('}') {
                    block_stack.pop();
                    if block_stack.is_empty() {
                        if let Some(b) = cur_block.take() {
                            script.blocks.push(b);
                        }
                        state = State::Top;
                    }
                }
            }
            State::InTextField => {
                if line.starts_with('}') {
                    block_stack.pop();
                    let tf = cur_text.take();
                    if let Some(b) = cur_block.as_mut() {
                        b.text_field = tf;
                    }
                    state = State::InBlock;
                    continue;
                }
                if let Some(tf) = cur_text.as_mut() {
                    if line.starts_with("pagebreak") {
                        tf.pagebreak = true;
                    } else if line.starts_with("ja") {
                        let mut parts = extract_strings(line);
                        if let Some(idx) = parts.iter().position(|s| s.is_empty()) {
                            parts.remove(idx);
                        }
                        tf.ja.extend(parts);
                    } else if line.starts_with("vo") {
                        tf.vo.extend(parse_voice_entries(line));
                    }
                }
            }
            State::InLabelTable => {
                if line.starts_with('}') {
                    block_stack.pop();
                    if block_stack.is_empty() {
                        state = State::Top;
                    }
                    continue;
                }
                if let Some((name, block, label)) = parse_label_entry(line) {
                    script.labels.insert(name, AstLabelRef { block, label });
                }
            }
        }
    }

    if let Some(b) = cur_block.take() {
        script.blocks.push(b);
    }

    if script.blocks.is_empty() {
        bail!("{}: no blocks parsed (not an .ast file?)", src_name);
    }
    Ok(script)
}

// ─── Line helpers ─────────────────────────────────────────────────────────

/// Remove Lua `--` comments. Cuts at first `--` that is not inside a string.
fn strip_comment(line: &str) -> String {
    let mut in_str = false;
    let mut esc = false;
    let mut idx = line.len();
    for (i, c) in line.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        if in_str {
            if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '-' && line[i + 1..].starts_with('-') {
            idx = i;
            break;
        }
    }
    line[..idx].to_string()
}

fn parse_block_header(line: &str) -> Option<String> {
    let line = line.trim_end_matches(',');
    let (name, rest) = line.split_once('=')?;
    let name = name.trim();
    if !name.starts_with("block_") {
        return None;
    }
    if !rest.trim().starts_with('{') {
        return None;
    }
    Some(name.to_string())
}

fn parse_quoted_assign(line: &str, key: &str) -> Option<String> {
    let line = line.trim_end_matches(',');
    let (k, v) = line.split_once('=')?;
    if k.trim() != key {
        return None;
    }
    let v = v.trim().trim_matches('"');
    if v.is_empty() {
        return None;
    }
    Some(v.to_string())
}

fn parse_num_assign(line: &str, key: &str) -> Option<u32> {
    let line = line.trim_end_matches(',');
    let (k, v) = line.split_once('=')?;
    if k.trim() != key {
        return None;
    }
    v.trim().parse().ok()
}

/// Parse a command line: `{"tag", key=val, key="val", ...},`
fn parse_command(line: &str) -> Option<AstCommand> {
    let line = line.trim_end_matches(',');
    let inner = line.trim().strip_prefix("{\"")?.strip_suffix('}')?;
    let (tag, rest) = inner.split_once('"')?;
    let tag = tag.to_string();
    let mut attrs = HashMap::new();
    for part in split_top_level(rest) {
        let part = part.trim().trim_end_matches(',');
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim().to_string();
            let v = v.trim().trim_matches('"').to_string();
            attrs.insert(k, v);
        }
    }
    Some(AstCommand { tag, attrs })
}

/// Split on commas at top level (ignoring commas inside quoted strings).
fn split_top_level(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        if in_str {
            cur.push(c);
            if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            cur.push(c);
            in_str = true;
        } else if c == ',' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Extract all quoted strings from a line (used for `ja = {{"a"},{"b"},}`).
fn extract_strings(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            let mut esc = false;
            for c2 in chars.by_ref() {
                if esc {
                    s.push(match c2 {
                        'n' => '\n',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    });
                    esc = false;
                } else if c2 == '\\' {
                    esc = true;
                } else if c2 == '"' {
                    break;
                } else {
                    s.push(c2);
                }
            }
            out.push(s);
        }
    }
    out
}

/// Parse `vo = {{"vo", ch="li", file="n002"},}` entries (one or many per line).
fn parse_voice_entries(line: &str) -> Vec<AstVoice> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find("{\"vo\"") {
        let after = &rest[pos + "{\"vo\"".len()..];
        let end = after.find('}').unwrap_or(after.len());
        let inner = &after[..end];
        let mut ch = String::new();
        let mut file = String::new();
        for attr in split_top_level(inner) {
            let attr = attr.trim().trim_end_matches(',');
            if let Some((k, v)) = attr.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"');
                if k == "ch" {
                    ch = v.to_string();
                } else if k == "file" {
                    file = v.to_string();
                }
            }
        }
        if !file.is_empty() {
            out.push(AstVoice { ch, file });
        }
        rest = &after[end..];
    }
    out
}

/// Parse label table entry: `top = { block="block_00000", label=1 },`
fn parse_label_entry(line: &str) -> Option<(String, String, u32)> {
    let line = line.trim_end_matches(',');
    let (name, rest) = line.split_once('=')?;
    let name = name.trim();
    if name.starts_with('{') || name.is_empty() {
        return None;
    }
    let mut block = String::new();
    let mut label = 0u32;
    for part in split_top_level(rest) {
        let part = part.trim().trim_start_matches('{').trim_end_matches('}');
        let part = part.trim_end_matches(',');
        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "block" => block = v.trim().trim_matches('"').to_string(),
                "label" => label = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if block.is_empty() {
        return None;
    }
    Some((name.to_string(), block, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"------------已审核---------
astver = 2.0
astname = "ast"
ast = {
    block_00000 = {
        {"savetitle", text="水仙1-0"},
        {"user", mode="autosave", no=0},
        {"eval", exp="g.chap01=1"},
        {"msg", no=99, mode="novel"},
        {"cgdel", id=-1},
        {"fg", mode=-2},
        {"bg", id=1, lv=5, file="black", time=1500, path=":bg/", sync=0},
        {"ex", time=1500, func="wait"},
        {"text"},
        text = {
            pagebreak = true,
            vo = {{"vo", ch="li", file="n002"},},
            ja = {{"「…的确，从小时候起，我的身体就不是很好…」"},},
        },
        linkback = "block_00000",
        linknext = "block_00001",
        line = 98
    },
    block_00001 = {
        {"text"},
        text = {
            pagebreak = true,
            ja = {{"但也和别人一样上了小学，\n也有过暑假的时候一直玩耍到皮肤被晒黑。"},},
        },
        linkback = "block_00000",
        line = 104
    },
    label = {
        z00 = { block="block_00000", label=2 },
        top = { block="block_00000", label=1 },
    },
}"#;

    #[test]
    fn parses_blocks_and_commands() {
        let s = parse_ast_str(SAMPLE, "test").unwrap();
        assert_eq!(s.blocks.len(), 2);
        assert_eq!(s.blocks[0].name, "block_00000");
        assert_eq!(s.blocks[0].commands.len(), 9);
        assert_eq!(s.blocks[0].commands[0].tag, "savetitle");
        assert_eq!(s.blocks[0].commands[0].attrs.get("text").unwrap(), "水仙1-0");
        assert_eq!(s.blocks[0].commands[6].tag, "bg");
        assert_eq!(s.blocks[0].commands[6].attrs.get("path").unwrap(), ":bg/");
        assert_eq!(s.blocks[0].linknext.as_deref(), Some("block_00001"));
        assert_eq!(s.blocks[0].line, Some(98));
    }

    #[test]
    fn parses_text_field() {
        let s = parse_ast_str(SAMPLE, "test").unwrap();
        let tf = s.blocks[0].text_field.as_ref().unwrap();
        assert!(tf.pagebreak);
        assert_eq!(tf.ja.len(), 1);
        assert!(tf.ja[0].contains("从小时候起"));
        assert_eq!(tf.vo.len(), 1);
        assert_eq!(tf.vo[0].ch, "li");
        assert_eq!(tf.vo[0].file, "n002");
    }

    #[test]
    fn parses_multiline_ja_escape() {
        let s = parse_ast_str(SAMPLE, "test").unwrap();
        let tf = s.blocks[1].text_field.as_ref().unwrap();
        assert_eq!(tf.ja.len(), 1);
        assert!(tf.ja[0].contains('\n'), "\\n should become newline: {:?}", tf.ja[0]);
    }

    #[test]
    fn parses_labels() {
        let s = parse_ast_str(SAMPLE, "test").unwrap();
        assert_eq!(s.labels.len(), 2);
        assert_eq!(s.labels["top"].block, "block_00000");
        assert_eq!(s.labels["top"].label, 1);
    }

    #[test]
    fn handles_comments() {
        assert_eq!(strip_comment("-- comment"), "");
        assert_eq!(strip_comment("  -- spaced comment").trim(), "");
        assert_eq!(strip_comment("{\"se\", id=1, file=\"extra/n001\"}, -- comment").trim(),
            "{\"se\", id=1, file=\"extra/n001\"},");
        let l = "{\"eval\", exp=\"a -- b\"}";
        assert_eq!(strip_comment(l), l);
    }
}
