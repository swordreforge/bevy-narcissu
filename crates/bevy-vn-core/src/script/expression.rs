//! Expression evaluator for VN script conditions.
//!
//! Pure-Rust helpers for evaluating `if`/`condition` expressions and
//! simple flag arithmetic. No Bevy dependencies.

use std::collections::HashMap;

// ── Error type ──

#[derive(Debug, Clone)]
pub enum ExpressionError {
    ParseError(String),
    FlagNotFound(String),
}

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionError::ParseError(msg) => write!(f, "expression parse error: {msg}"),
            ExpressionError::FlagNotFound(name) => write!(f, "flag not found: {name}"),
        }
    }
}

impl std::error::Error for ExpressionError {}

// ── Operators ──

const CONDITION_OPERATORS: [&str; 6] = ["==", "!=", ">=", "<=", ">", "<"];
const WORK_OPERATORS: [char; 2] = ['+', '-'];

// ── Helpers ──

/// Look up a flag, tolerating a namespace prefix so that `t.tmp` also
/// resolves when the map stores the key as `tmp`.
fn lookup_flag(key: &str, flags: &HashMap<String, i32>) -> Result<i32, ExpressionError> {
    if let Some(&value) = flags.get(key) {
        return Ok(value);
    }
    // Fallback: strip the namespace prefix (everything before the last '.').
    if let Some((_, bare)) = key.rsplit_once('.') {
        if let Some(&value) = flags.get(bare) {
            return Ok(value);
        }
    }
    Err(ExpressionError::FlagNotFound(key.to_string()))
}

// ── Public API ──

/// Evaluate a condition expression against the given flag map.
///
/// Supported forms:
/// - `"flag_name op value"` — e.g. `"affection >= 5"`. Compares a flag
///   against an integer literal using `==`, `!=`, `>=`, `<=`, `>`, `<`.
/// - `"flag_name"` (bare) — truthy test: `true` iff the flag value is non-zero.
///
/// Whitespace around tokens is trimmed.
pub fn evaluate_condition(
    expression: &str,
    flags: &HashMap<String, i32>,
) -> Result<bool, ExpressionError> {
    let expr = expression.trim();
    if expr.is_empty() {
        return Err(ExpressionError::ParseError("expression is empty".into()));
    }

    // Find the left-most operator occurrence.
    let mut found: Option<(usize, &str)> = None;
    for op in CONDITION_OPERATORS {
        if let Some(idx) = expr.find(op) {
            if found.is_none_or(|(best, _)| idx < best) {
                found = Some((idx, op));
            }
        }
    }

    let Some((idx, op)) = found else {
        // Bare flag name — truthy test.
        let value = lookup_flag(expr, flags)?;
        return Ok(value != 0);
    };

    let left = expr[..idx].trim();
    let right = expr[idx + op.len()..].trim();

    if left.is_empty() {
        return Err(ExpressionError::ParseError(format!(
            "missing flag name before {op:?} in {expression:?}"
        )));
    }

    let flag_value = lookup_flag(left, flags)?;
    let operand: i32 = right.parse().map_err(|_| {
        ExpressionError::ParseError(format!(
            "expected integer on right side of {op:?}, got {right:?}"
        ))
    })?;

    match op {
        "==" => Ok(flag_value == operand),
        "!=" => Ok(flag_value != operand),
        ">=" => Ok(flag_value >= operand),
        "<=" => Ok(flag_value <= operand),
        ">" => Ok(flag_value > operand),
        "<" => Ok(flag_value < operand),
        _ => unreachable!(),
    }
}

/// Evaluate a flag arithmetic expression.
///
/// Supported form: `"flag_name op literal"` — e.g. `"t.tmp + 3"`.
/// The left side is the flag key (dots preserved); the right side is an
/// integer literal. `+` adds, `-` subtracts. The result is returned without
/// writing back to the map.
///
/// If no operator is present, the flag value is returned directly.
pub fn evaluate_work_expression(
    expression: &str,
    flags: &HashMap<String, i32>,
) -> Result<i32, ExpressionError> {
    let expr = expression.trim();
    if expr.is_empty() {
        return Err(ExpressionError::ParseError("expression is empty".into()));
    }

    // Left-most `+` or `-` separates the flag key from the operand.
    let op_entry = expr
        .char_indices()
        .find(|(_, c)| WORK_OPERATORS.contains(c))
        .map(|(idx, c)| (idx, c));

    let Some((idx, op)) = op_entry else {
        // No operator — treat as bare flag reference.
        return lookup_flag(expr, flags);
    };

    let left = expr[..idx].trim();
    let right = expr[idx + 1..].trim();

    if left.is_empty() {
        return Err(ExpressionError::ParseError(format!(
            "missing flag name before {op:?} in {expression:?}"
        )));
    }

    let flag_value = lookup_flag(left, flags)?;
    let operand: i32 = right.parse().map_err(|_| {
        ExpressionError::ParseError(format!(
            "expected integer on right side of {op:?}, got {right:?}"
        ))
    })?;

    match op {
        '+' => Ok(flag_value + operand),
        '-' => Ok(flag_value - operand),
        _ => unreachable!(),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> HashMap<String, i32> {
        let mut m = HashMap::new();
        m.insert("affection".into(), 5);
        m.insert("alive".into(), 1);
        m.insert("dead".into(), 0);
        m.insert("t.tmp".into(), 10);
        m.insert("gold".into(), -3);
        m
    }

    #[test]
    fn condition_eq() {
        let f = flags();
        assert!(evaluate_condition("affection == 5", &f).unwrap());
        assert!(!evaluate_condition("affection == 4", &f).unwrap());
    }

    #[test]
    fn condition_ne() {
        let f = flags();
        assert!(evaluate_condition("affection != 4", &f).unwrap());
        assert!(!evaluate_condition("affection != 5", &f).unwrap());
    }

    #[test]
    fn condition_ordering() {
        let f = flags();
        assert!(evaluate_condition("affection >= 5", &f).unwrap());
        assert!(!evaluate_condition("affection >= 6", &f).unwrap());
        assert!(evaluate_condition("affection <= 5", &f).unwrap());
        assert!(!evaluate_condition("affection <= 4", &f).unwrap());
        assert!(evaluate_condition("affection > 4", &f).unwrap());
        assert!(!evaluate_condition("affection > 5", &f).unwrap());
        assert!(evaluate_condition("affection < 6", &f).unwrap());
    }

    #[test]
    fn condition_negative_operand() {
        let f = flags();
        assert!(evaluate_condition("gold == -3", &f).unwrap());
        assert!(evaluate_condition("gold < 0", &f).unwrap());
    }

    #[test]
    fn condition_bare_flag() {
        let f = flags();
        assert!(evaluate_condition("alive", &f).unwrap());
        assert!(!evaluate_condition("dead", &f).unwrap());
    }

    #[test]
    fn condition_whitespace_is_trimmed() {
        let f = flags();
        assert!(evaluate_condition("  affection   >=   5  ", &f).unwrap());
        assert!(evaluate_condition("  alive  ", &f).unwrap());
    }

    #[test]
    fn condition_dotted_flag_key() {
        let f = flags();
        assert!(evaluate_condition("t.tmp >= 10", &f).unwrap());
    }

    #[test]
    fn condition_flag_not_found() {
        let f = flags();
        assert!(matches!(
            evaluate_condition("unknown == 3", &f),
            Err(ExpressionError::FlagNotFound(_))
        ));
    }

    #[test]
    fn condition_parse_errors() {
        let f = flags();
        assert!(matches!(
            evaluate_condition("", &f),
            Err(ExpressionError::ParseError(_))
        ));
        assert!(matches!(
            evaluate_condition("affection >= five", &f),
            Err(ExpressionError::ParseError(_))
        ));
        assert!(matches!(
            evaluate_condition("== 5", &f),
            Err(ExpressionError::ParseError(_))
        ));
    }

    #[test]
    fn condition_fallback_strips_namespace() {
        let mut f = HashMap::new();
        f.insert("tmp".into(), 7);
        assert!(evaluate_condition("t.tmp == 7", &f).unwrap());
    }

    #[test]
    fn work_add() {
        let f = flags();
        assert_eq!(evaluate_work_expression("t.tmp + 3", &f).unwrap(), 13);
        assert_eq!(evaluate_work_expression("t.tmp+3", &f).unwrap(), 13);
    }

    #[test]
    fn work_subtract() {
        let f = flags();
        assert_eq!(evaluate_work_expression("t.tmp - 3", &f).unwrap(), 7);
    }

    #[test]
    fn work_negative_rhs() {
        let f = flags();
        assert_eq!(evaluate_work_expression("t.tmp + -3", &f).unwrap(), 7);
        assert_eq!(evaluate_work_expression("gold + 1", &f).unwrap(), -2);
    }

    #[test]
    fn work_bare_flag() {
        let f = flags();
        assert_eq!(evaluate_work_expression("t.tmp", &f).unwrap(), 10);
    }

    #[test]
    fn work_whitespace() {
        let f = flags();
        assert_eq!(
            evaluate_work_expression("  t.tmp  +  3  ", &f).unwrap(),
            13
        );
    }

    #[test]
    fn work_flag_not_found() {
        let f = flags();
        assert!(matches!(
            evaluate_work_expression("unknown + 1", &f),
            Err(ExpressionError::FlagNotFound(_))
        ));
    }

    #[test]
    fn work_parse_errors() {
        let f = flags();
        assert!(matches!(
            evaluate_work_expression("", &f),
            Err(ExpressionError::ParseError(_))
        ));
        assert!(matches!(
            evaluate_work_expression("t.tmp + abc", &f),
            Err(ExpressionError::ParseError(_))
        ));
        assert!(matches!(
            evaluate_work_expression("+ 3", &f),
            Err(ExpressionError::ParseError(_))
        ));
    }

    #[test]
    fn error_is_displayable() {
        let f = flags();
        let err = evaluate_condition("affection >= five", &f).unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
