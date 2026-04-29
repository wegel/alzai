//! Handler for `alzai reflect`: prompt memory logging decisions.

use anyhow::Result;
use serde::Serialize;

use crate::cmd_status;
use crate::colors;
use crate::repo::RepoPaths;
use crate::types::TopicStatus;

// --- Reflection policy ---

const SHOULD_LOG: &[&str] = &[
    "a durable design decision or tradeoff",
    "a pitfall that cost time or would cost future agents time",
    "a non-obvious invariant, constraint, or operational requirement",
    "an unresolved open question that future work should preserve",
    "a workaround or caveat future agents must know before changing code",
];

const SHOULD_NOT_LOG: &[&str] = &[
    "ordinary task progress",
    "facts obvious from reading nearby code",
    "speculation without evidence",
    "temporary commands, scratch paths, or one-off debugging notes",
];

// --- Public API ---

/// Run the reflect command: print a checkpoint, but do not write memory.
pub fn run(paths: &RepoPaths, json: bool) -> Result<()> {
    let statuses = cmd_status::collect_statuses(paths)?;

    if json {
        println!("{}", render_json(&statuses)?);
    } else {
        print!("{}", render_text(&statuses));
    }

    Ok(())
}

/// Render the human-readable reflection checkpoint.
pub fn render_text(statuses: &[TopicStatus]) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}\n\n", colors::bold("Memory reflection")));
    push_topics(&mut out, statuses);
    push_pending_sync(&mut out, statuses);

    out.push_str("Log something if this session produced:\n");
    for item in SHOULD_LOG {
        out.push_str(&format!("  - {}\n", item));
    }

    out.push_str("\nDo not log:\n");
    for item in SHOULD_NOT_LOG {
        out.push_str(&format!("  - {}\n", item));
    }

    out.push_str("\nUse:\n");
    out.push_str("  alzai log --topic <topic> --kind <kind> --title \"...\" --body \"...\"\n");

    out
}

/// Render the reflection checkpoint as machine-readable JSON.
pub fn render_json(statuses: &[TopicStatus]) -> Result<String> {
    let payload = ReflectPayload {
        topics: statuses,
        pending_events: statuses.iter().map(|s| s.pending_events).sum(),
        log_when: SHOULD_LOG,
        do_not_log: SHOULD_NOT_LOG,
        command: "alzai log --topic <topic> --kind <kind> --title \"...\" --body \"...\"",
    };

    Ok(serde_json::to_string(&payload)?)
}

// --- Private types ---

#[derive(Serialize)]
struct ReflectPayload<'a> {
    topics: &'a [TopicStatus],
    pending_events: u64,
    log_when: &'static [&'static str],
    do_not_log: &'static [&'static str],
    command: &'static str,
}

// --- Private helpers ---

fn push_topics(out: &mut String, statuses: &[TopicStatus]) {
    out.push_str("Available topics:\n");

    if statuses.is_empty() {
        out.push_str("  none\n\n");
        return;
    }

    for status in statuses {
        out.push_str(&format!("  - {}\n", status.name));
    }
    out.push('\n');
}

fn push_pending_sync(out: &mut String, statuses: &[TopicStatus]) {
    let pending: u64 = statuses.iter().map(|s| s.pending_events).sum();
    if pending == 0 {
        return;
    }

    out.push_str(&format!(
        "{} {} pending event(s); run `alzai sync` when synthesis is appropriate.\n\n",
        colors::warn("Note:"),
        pending,
    ));
}
