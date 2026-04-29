//! Handler for `alzai context`: bootstrap agent memory usage.

use anyhow::Result;
use serde::Serialize;

use crate::cmd_status;
use crate::colors;
use crate::repo::RepoPaths;
use crate::types::TopicStatus;

// --- Session policy ---

const START_SESSION: &[&str] = &[
    "Read this output before starting work.",
    "Use the available topics below when choosing where to log memory.",
    "Read relevant `.agents/memory/facts/<topic>.md` files before changing code.",
];

const WHEN_TO_LOG: &[&str] = &[
    "durable architectural constraints or invariants",
    "decisions and tradeoffs",
    "pitfalls that cost time or would cost future agents time",
    "workarounds future agents must know",
    "unresolved open questions",
];

const DO_NOT_LOG: &[&str] = &[
    "transient task progress",
    "facts obvious from nearby code",
    "speculation without evidence",
    "temporary commands, scratch paths, or one-off debugging notes",
];

const KINDS: &[&str] = &[
    "fact",
    "decision",
    "pitfall",
    "open_question",
    "assumption",
    "workaround",
    "contradiction",
];

const END_SESSION: &[&str] = &[
    "Run `alzai reflect` before finishing substantial work.",
    "If you logged memory, optionally run `alzai sync` to synthesize facts.",
];

const IMPORTANT: &[&str] = &[
    "Log only to existing topics.",
    "Do not edit `.agents/memory/events/*.jsonl`.",
    "Treat `.agents/memory/facts/*.md` as synthesized reading material.",
];

const LOG_COMMAND: &str = "alzai log --topic <topic> --kind <kind> --title \"...\" --body \"...\"";

// --- Public API ---

/// Run the context command: print session-start memory instructions.
pub fn run(paths: &RepoPaths, json: bool) -> Result<()> {
    let statuses = cmd_status::collect_statuses(paths)?;

    if json {
        println!("{}", render_json(&statuses)?);
    } else {
        print!("{}", render_text(&statuses));
    }

    Ok(())
}

/// Render the human-readable session-start context.
pub fn render_text(statuses: &[TopicStatus]) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}\n\n", colors::bold("Alzai context")));
    out.push_str("This repo uses alzai for durable agent memory.\n\n");

    push_list(&mut out, "At session start:", START_SESSION);
    push_topics(&mut out, statuses);
    push_pending_sync(&mut out, statuses);
    push_list(&mut out, "When to log:", WHEN_TO_LOG);
    push_list(&mut out, "Do not log:", DO_NOT_LOG);
    push_list(&mut out, "Kinds:", KINDS);

    out.push_str("How to log:\n");
    out.push_str(&format!("  {}\n\n", LOG_COMMAND));

    push_list(&mut out, "At task end:", END_SESSION);
    push_list(&mut out, "Important:", IMPORTANT);

    out
}

/// Render the session-start context as machine-readable JSON.
pub fn render_json(statuses: &[TopicStatus]) -> Result<String> {
    let payload = ContextPayload {
        summary: "This repo uses alzai for durable agent memory.",
        start_session: START_SESSION,
        topics: statuses
            .iter()
            .map(TopicContext::from_status)
            .collect::<Vec<_>>(),
        pending_events: statuses.iter().map(|s| s.pending_events).sum(),
        when_to_log: WHEN_TO_LOG,
        do_not_log: DO_NOT_LOG,
        kinds: KINDS,
        log_command: LOG_COMMAND,
        end_session: END_SESSION,
        important: IMPORTANT,
    };

    Ok(serde_json::to_string(&payload)?)
}

// --- Private types ---

#[derive(Serialize)]
struct ContextPayload<'a> {
    summary: &'static str,
    start_session: &'static [&'static str],
    topics: Vec<TopicContext<'a>>,
    pending_events: u64,
    when_to_log: &'static [&'static str],
    do_not_log: &'static [&'static str],
    kinds: &'static [&'static str],
    log_command: &'static str,
    end_session: &'static [&'static str],
    important: &'static [&'static str],
}

#[derive(Serialize)]
struct TopicContext<'a> {
    name: &'a str,
    fact_path: String,
    total_events: u64,
    pending_events: u64,
    synced_through_seq: u64,
}

impl<'a> TopicContext<'a> {
    fn from_status(status: &'a TopicStatus) -> Self {
        Self {
            name: &status.name,
            fact_path: fact_path(&status.name),
            total_events: status.total_events,
            pending_events: status.pending_events,
            synced_through_seq: status.synced_through_seq,
        }
    }
}

// --- Private helpers ---

fn push_list(out: &mut String, title: &str, items: &[&str]) {
    out.push_str(title);
    out.push('\n');

    for item in items {
        out.push_str(&format!("  - {}\n", item));
    }
    out.push('\n');
}

fn push_topics(out: &mut String, statuses: &[TopicStatus]) {
    out.push_str("Available topics:\n");

    if statuses.is_empty() {
        out.push_str("  none\n\n");
        return;
    }

    let name_width = statuses
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(5)
        .max(5);

    for status in statuses {
        let pending = if status.pending_events > 0 {
            format!(", {} pending sync", status.pending_events)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "  {:<width$}  {} event(s){}  {}\n",
            status.name,
            status.total_events,
            pending,
            fact_path(&status.name),
            width = name_width,
        ));
    }
    out.push('\n');
}

fn push_pending_sync(out: &mut String, statuses: &[TopicStatus]) {
    let pending: u64 = statuses.iter().map(|s| s.pending_events).sum();
    if pending == 0 {
        return;
    }

    out.push_str(&format!(
        "{} {} pending event(s) are not synthesized yet. Run `alzai sync` if LLM synthesis is configured.\n\n",
        colors::warn("Note:"),
        pending,
    ));
}

fn fact_path(topic: &str) -> String {
    format!(".agents/memory/facts/{}.md", topic)
}
