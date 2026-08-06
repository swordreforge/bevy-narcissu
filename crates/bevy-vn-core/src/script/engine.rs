//! Pure-data script interpreter. No Bevy world access — fully testable.
//!
//! `Label` variants are metadata for jump/call targets.
//! `advance()` automatically skips them.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::script::cmd::{VnScript, ScriptCmd};

// ── Engine ──

#[derive(Resource, Debug, Clone)]
pub struct ScriptEngine {
    /// All loaded scripts, keyed by name.
    pub scripts: HashMap<String, VnScript>,
    /// Name of the currently executing script.
    pub current_script: String,
    /// Index into the current script's instructions.
    pub current_line: usize,
    /// Call stack: (script_name, return_line_at_caller).
    pub call_stack: Vec<(String, usize)>,
    /// Named flags (SetFlag, etc.).
    pub flags: HashMap<String, i32>,
    /// Global flags (SetGlobalFlag, route flags).
    pub global_flags: HashMap<u32, i32>,
    /// Currently active route, if any.
    pub current_route: Option<String>,
    /// Set to true by the runner on `Halt` or script exhaustion.
    pub finished: bool,
}

// ── Errors ──

#[derive(Debug, Clone)]
pub enum ScriptError {
    LabelNotFound(String),
    ScriptNotFound(String),
    NoCurrentScript,
    StackUnderflow,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::LabelNotFound(l) => write!(f, "label not found: {l}"),
            ScriptError::ScriptNotFound(s) => write!(f, "script not found: {s}"),
            ScriptError::NoCurrentScript => write!(f, "no current script loaded"),
            ScriptError::StackUnderflow => write!(f, "call stack underflow"),
        }
    }
}

impl std::error::Error for ScriptError {}

// ── Implementation ──

impl ScriptEngine {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            current_script: String::new(),
            current_line: 0,
            call_stack: Vec::new(),
            flags: HashMap::new(),
            global_flags: HashMap::new(),
            current_route: None,
            finished: false,
        }
    }

    /// Load a script into the engine's script map.
    pub fn load_script(&mut self, name: String, script: VnScript) {
        self.scripts.insert(name, script);
    }

    /// Switch to a named script, optionally starting at a label.
    /// Clears the call stack — use for external entry points (route start).
    pub fn set_current(
        &mut self,
        name: &str,
        start_label: Option<&str>,
    ) -> Result<(), ScriptError> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| ScriptError::ScriptNotFound(name.to_owned()))?;
        let line = match start_label {
            Some(l) => script
                .instructions
                .iter()
                .position(|c| matches!(c, ScriptCmd::Label { name } if name == l))
                .ok_or_else(|| ScriptError::LabelNotFound(l.to_owned()))?,
            None => 0,
        };
        self.current_script = name.to_owned();
        self.current_line = line;
        self.call_stack.clear();
        self.finished = false;
        Ok(())
    }

    /// Peek at the current instruction.
    pub fn current(&self) -> Option<&ScriptCmd> {
        self.current_script_data()?
            .instructions
            .get(self.current_line)
    }

    /// Get the currently executing VnScript.
    pub fn current_script_data(&self) -> Option<&VnScript> {
        self.scripts.get(&self.current_script)
    }

    /// Move to the next non-Label instruction.
    /// Returns `None` when past the end of the script.
    pub fn advance(&mut self) -> Option<&ScriptCmd> {
        let instructions = &self.scripts.get(&self.current_script)?.instructions;
        let len = instructions.len();
        while self.current_line + 1 < len {
            self.current_line += 1;
            if !matches!(&instructions[self.current_line], ScriptCmd::Label { .. }) {
                return Some(&instructions[self.current_line]);
            }
        }
        self.current_line = len; // past end
        None
    }

    /// Look ahead without moving the instruction pointer.
    pub fn peek_next(&self) -> Option<&ScriptCmd> {
        let script = self.current_script_data()?;
        let mut i = self.current_line + 1;
        while i < script.instructions.len() {
            match &script.instructions[i] {
                ScriptCmd::Label { .. } => i += 1,
                other => return Some(other),
            }
        }
        None
    }

    /// Jump to a label in the current script.
    pub fn jump_to_label(&mut self, label: &str) -> Result<(), ScriptError> {
        let idx = self
            .label_index(label)
            .ok_or_else(|| ScriptError::LabelNotFound(label.to_owned()))?;
        self.current_line = idx;
        self.finished = false;
        Ok(())
    }

    /// Push current position onto the call stack, then jump to a label.
    pub fn call_label(&mut self, label: &str) -> Result<(), ScriptError> {
        let idx = self
            .label_index(label)
            .ok_or_else(|| ScriptError::LabelNotFound(label.to_owned()))?;
        self.call_stack
            .push((self.current_script.clone(), self.current_line));
        self.current_line = idx;
        self.finished = false;
        Ok(())
    }

    /// Push current position, switch to another script, optionally at a label.
    pub fn call_script(
        &mut self,
        script_name: &str,
        label: Option<&str>,
    ) -> Result<(), ScriptError> {
        let target = self
            .scripts
            .get(script_name)
            .ok_or_else(|| ScriptError::ScriptNotFound(script_name.to_owned()))?;
        let label_idx = match label {
            Some(l) => Some(
                target
                    .instructions
                    .iter()
                    .position(|c| matches!(c, ScriptCmd::Label { name } if name == l))
                    .ok_or_else(|| ScriptError::LabelNotFound(l.to_owned()))?,
            ),
            None => None,
        };
        self.call_stack
            .push((self.current_script.clone(), self.current_line));
        self.current_script = script_name.to_owned();
        self.current_line = label_idx.unwrap_or(0);
        self.finished = false;
        Ok(())
    }

    /// Pop the call stack, restoring the previous script and line.
    pub fn return_from_call(&mut self) -> Result<(), ScriptError> {
        let (script, line) = self.call_stack.pop().ok_or(ScriptError::StackUnderflow)?;
        self.current_script = script;
        self.current_line = line;
        self.finished = false;
        Ok(())
    }

    /// Whether the engine has more instructions to execute.
    pub fn has_more(&self) -> bool {
        !self.finished && self.current().is_some()
    }

    /// Get the next script name from the current script's metadata.
    pub fn next_script_name(&self) -> Option<&str> {
        self.current_script_data()?
            .meta
            .next_script
            .as_deref()
    }

    /// Find the index of a Label variant in the current script.
    pub fn label_index(&self, label: &str) -> Option<usize> {
        self.current_script_data()?.instructions.iter().position(|c| {
            matches!(c, ScriptCmd::Label { name } if name == label)
        })
    }

    /// Collect every voice file referenced by a script (its own instructions
    /// plus every script it may `call`/`call_script` transitively).
    /// Used for voice preloading so playback never waits on asset loading.
    pub fn collect_voice_files(&self, script_name: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![script_name.to_owned()];
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) { continue; }
            let Some(script) = self.scripts.get(&name) else { continue };
            for cmd in &script.instructions {
                match cmd {
                    ScriptCmd::Dialogue { voice: Some(f), .. } => {
                        if !out.contains(f) { out.push(f.clone()); }
                    }
                    ScriptCmd::PlayVoice { file, .. } => {
                        if !out.contains(file) { out.push(file.clone()); }
                    }
                    ScriptCmd::CallScript { script: s, .. } => stack.push(s.clone()),
                    ScriptCmd::SetNextScript { .. } => {}
                    _ => {}
                }
            }
            if let Some(next) = script.meta.next_script.as_deref() {
                stack.push(next.to_owned());
            }
        }
        out
    }
}

// ── Default ──

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::cmd::{ScriptMeta, ScriptVersion};

    fn make_script(name: &str, next: Option<&str>, cmds: Vec<ScriptCmd>) -> VnScript {
        VnScript {
            version: ScriptVersion::V1,
            meta: ScriptMeta {
                name: Some(name.into()),
                next_script: next.map(String::from),
            },
            instructions: cmds,
        }
    }

    fn dummy_dialogue(text: &str) -> ScriptCmd {
        ScriptCmd::Dialogue {
            speaker: None,
            text: text.into(),
            voice: None,
        }
    }

    #[test]
    fn new_engine_is_empty() {
        let e = ScriptEngine::new();
        assert!(!e.has_more());
        assert!(e.current().is_none());
    }

    #[test]
    fn set_current_from_start() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script("test", None, vec![dummy_dialogue("hi"), dummy_dialogue("there")]),
        );
        e.set_current("test", None).unwrap();
        // current_line = 0 → points at first instruction
        assert!(e.has_more());
        let cmd = e.current().unwrap();
        assert!(matches!(cmd, ScriptCmd::Dialogue { text, .. } if text == "hi"));
    }

    #[test]
    fn set_current_with_label() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script(
                "test",
                None,
                vec![
                    dummy_dialogue("skip"),
                    ScriptCmd::Label { name: "start".into() },
                    dummy_dialogue("target"),
                ],
            ),
        );
        e.set_current("test", Some("start")).unwrap();
        // current_line = 1 → Label, advance() will skip it
        assert!(matches!(e.current().unwrap(), ScriptCmd::Label { .. }));
        let after = e.advance().unwrap();
        assert!(matches!(after, ScriptCmd::Dialogue { text, .. } if text == "target"));
    }

    #[test]
    fn set_current_script_not_found() {
        let mut e = ScriptEngine::new();
        assert!(matches!(
            e.set_current("missing", None),
            Err(ScriptError::ScriptNotFound(_))
        ));
    }

    #[test]
    fn advance_skips_labels() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script(
                "test",
                None,
                vec![
                    dummy_dialogue("a"),
                    ScriptCmd::Label { name: "mid".into() },
                    dummy_dialogue("b"),
                    ScriptCmd::Label { name: "end".into() },
                ],
            ),
        );
        e.set_current("test", None).unwrap();

        // advance from line 0 → should reach line 2 (Dialogue "b")
        let cmd = e.advance().unwrap();
        assert!(matches!(cmd, ScriptCmd::Dialogue { text, .. } if text == "b"));

        // advance again → past end
        let after = e.advance();
        assert!(after.is_none());
        assert!(!e.has_more());
    }

    #[test]
    fn jump_to_label_finds_correct_position() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script(
                "test",
                None,
                vec![
                    dummy_dialogue("a"),
                    ScriptCmd::Label { name: "target".into() },
                    dummy_dialogue("b"),
                ],
            ),
        );
        e.set_current("test", None).unwrap();
        e.jump_to_label("target").unwrap();
        assert_eq!(e.current_line, 1);
    }

    #[test]
    fn call_label_roundtrip() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script(
                "test",
                None,
                vec![
                    dummy_dialogue("before"),
                    ScriptCmd::Label { name: "func".into() },
                    dummy_dialogue("inside"),
                    ScriptCmd::Return,
                ],
            ),
        );
        e.set_current("test", None).unwrap();

        // Save position at "before" (line 0), jump to "func" (line 1)
        e.call_label("func").unwrap();
        assert_eq!(e.current_line, 1);
        // call_stack = [("test", 0)]
        assert_eq!(e.call_stack.len(), 1);
        assert_eq!(e.call_stack[0].0, "test");
        assert_eq!(e.call_stack[0].1, 0);

        // Advance past label to "inside"
        let inside = e.advance().unwrap();
        assert!(matches!(inside, ScriptCmd::Dialogue { text, .. } if text == "inside"));

        // Return → back to "before" (line 0)
        e.return_from_call().unwrap();
        assert_eq!(e.current_line, 0);
        assert!(e.call_stack.is_empty());
    }

    #[test]
    fn call_script_switches_and_returns() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "main".into(),
            make_script(
                "main",
                None,
                vec![dummy_dialogue("main_a"), ScriptCmd::Return],
            ),
        );
        e.load_script(
            "sub".into(),
            make_script(
                "sub",
                None,
                vec![dummy_dialogue("sub_a"), ScriptCmd::Return],
            ),
        );
        e.set_current("main", None).unwrap();

        // Call sub
        e.call_script("sub", None).unwrap();
        assert_eq!(e.current_script, "sub");

        // Return to main
        e.return_from_call().unwrap();
        assert_eq!(e.current_script, "main");
    }

    #[test]
    fn call_script_with_label() {
        let mut e = ScriptEngine::new();
        e.load_script("main".into(), make_script("main", None, vec![dummy_dialogue("m")]));
        e.load_script(
            "sub".into(),
            make_script(
                "sub",
                None,
                vec![
                    dummy_dialogue("skip"),
                    ScriptCmd::Label { name: "entry".into() },
                    dummy_dialogue("target"),
                ],
            ),
        );
        e.set_current("main", None).unwrap();
        e.call_script("sub", Some("entry")).unwrap();
        assert_eq!(e.current_line, 1); // Label "entry"
        let after = e.advance().unwrap();
        assert!(matches!(after, ScriptCmd::Dialogue { text, .. } if text == "target"));
    }

    #[test]
    fn return_from_empty_stack_is_error() {
        let mut e = ScriptEngine::new();
        assert!(matches!(
            e.return_from_call(),
            Err(ScriptError::StackUnderflow)
        ));
    }

    #[test]
    fn next_script_name_from_meta() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "ch1".into(),
            make_script("ch1", Some("ch2"), vec![dummy_dialogue("x")]),
        );
        e.set_current("ch1", None).unwrap();
        assert_eq!(e.next_script_name(), Some("ch2"));
    }

    #[test]
    fn peek_next_sees_past_labels() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "test".into(),
            make_script(
                "test",
                None,
                vec![
                    dummy_dialogue("a"),
                    ScriptCmd::Label { name: "x".into() },
                    dummy_dialogue("b"),
                ],
            ),
        );
        e.set_current("test", None).unwrap();
        let next = e.peek_next().unwrap();
        assert!(matches!(next, ScriptCmd::Dialogue { text, .. } if text == "b"));
    }

    #[test]
    fn collect_voice_files_includes_dialogue_playvoice_and_calls() {
        let mut e = ScriptEngine::new();
        e.load_script(
            "main".into(),
            make_script(
                "main",
                Some("next"),
                vec![
                    ScriptCmd::Dialogue { speaker: None, text: "a".into(), voice: Some("li/a1".into()) },
                    ScriptCmd::Dialogue { speaker: None, text: "b".into(), voice: Some("li/a2".into()) },
                    ScriptCmd::PlayVoice { file: "li/a3".into(), volume: None },
                    ScriptCmd::CallScript { script: "sub".into(), label: None },
                ],
            ),
        );
        e.load_script(
            "sub".into(),
            make_script(
                "sub",
                None,
                vec![ScriptCmd::Dialogue { speaker: None, text: "c".into(), voice: Some("li/b1".into()) }],
            ),
        );
        e.load_script(
            "next".into(),
            make_script(
                "next",
                None,
                vec![ScriptCmd::Dialogue { speaker: None, text: "d".into(), voice: Some("li/c1".into()) }],
            ),
        );
        let files = e.collect_voice_files("main");
        assert!(files.contains(&"li/a1".to_string()));
        assert!(files.contains(&"li/a2".to_string()));
        assert!(files.contains(&"li/a3".to_string()));
        assert!(files.contains(&"li/b1".to_string()));
        assert!(files.contains(&"li/c1".to_string()));
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn collect_voice_files_handles_missing_script() {
        let e = ScriptEngine::new();
        assert!(e.collect_voice_files("nope").is_empty());
    }
}
