//! Terminal color palette and formatting helpers for all CLI output.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::style::{Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor};

// --- State ---

static JSON_MODE: AtomicBool = AtomicBool::new(false);

// --- Text colors ---

const DIM: Color = Color::Rgb { r: 119, g: 119, b: 119 };
const HIGHLIGHT: Color = Color::Rgb { r: 133, g: 183, b: 235 };
const OK: Color = Color::Rgb { r: 151, g: 196, b: 89 };
const WARN: Color = Color::Rgb { r: 239, g: 159, b: 39 };
const ERR: Color = Color::Rgb { r: 240, g: 149, b: 149 };
const WHITE: Color = Color::Rgb { r: 255, g: 255, b: 255 };

// --- Pill backgrounds ---

const PILL_DEFAULT_BG: Color = Color::Rgb { r: 50, g: 50, b: 50 };
const PILL_FACT_BG: Color = Color::Rgb { r: 42, g: 58, b: 26 };
const PILL_DECISION_BG: Color = Color::Rgb { r: 30, g: 58, b: 95 };
const PILL_PITFALL_BG: Color = Color::Rgb { r: 58, g: 26, b: 26 };
const PILL_QUESTION_BG: Color = Color::Rgb { r: 58, g: 42, b: 26 };

// --- Public API ---

/// Enable JSON mode (disables all color output).
pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

/// Whether color output is enabled.
pub fn colors_enabled() -> bool {
    !JSON_MODE.load(Ordering::Relaxed)
        && std::env::var("NO_COLOR").is_err()
        && std::io::stdout().is_terminal()
}

/// Format text as dim/muted.
pub fn dim(text: &str) -> String {
    styled(text, DIM)
}

/// Format text as highlighted.
pub fn highlight(text: &str) -> String {
    styled(text, HIGHLIGHT)
}

/// Format text as success.
pub fn ok(text: &str) -> String {
    styled(text, OK)
}

/// Format text as warning.
pub fn warn(text: &str) -> String {
    styled(text, WARN)
}

/// Format text as error.
pub fn err(text: &str) -> String {
    styled(text, ERR)
}

/// Format text as bold white.
pub fn bold(text: &str) -> String {
    if !colors_enabled() {
        return text.to_string();
    }
    format!(
        "{}{}{}{}",
        SetForegroundColor(WHITE),
        SetAttribute(Attribute::Bold),
        text,
        SetAttribute(Attribute::Reset),
    )
}

/// Render a kind pill badge with appropriate colors.
pub fn pill(kind: &str) -> String {
    let (fg, bg) = pill_colors(kind);
    if !colors_enabled() {
        return format!("[{}]", kind);
    }
    format!(
        "{}{} {} {}",
        SetBackgroundColor(bg),
        SetForegroundColor(fg),
        kind,
        SetAttribute(Attribute::Reset),
    )
}

// --- Private helpers ---

fn styled(text: &str, fg: Color) -> String {
    if !colors_enabled() {
        return text.to_string();
    }
    format!(
        "{}{}{}",
        SetForegroundColor(fg),
        text,
        SetAttribute(Attribute::Reset),
    )
}

fn pill_colors(kind: &str) -> (Color, Color) {
    match kind {
        "fact" => (OK, PILL_FACT_BG),
        "decision" => (HIGHLIGHT, PILL_DECISION_BG),
        "pitfall" => (ERR, PILL_PITFALL_BG),
        "open_question" => (WARN, PILL_QUESTION_BG),
        _ => (WHITE, PILL_DEFAULT_BG),
    }
}
