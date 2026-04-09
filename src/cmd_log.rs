//! Handler for `alzai log`: append a durable learning to the event log.

use anyhow::{Context, Result};
use chrono::Utc;

use crate::colors;
use crate::events;
use crate::fs_util;
use crate::repo::RepoPaths;
use crate::types::Event;

// --- Public API ---

/// Run the log command: validate topic, build event, append to JSONL.
pub fn run(
    paths: &RepoPaths,
    topic: &str,
    kind: &str,
    title: &str,
    body: Option<&str>,
    json: bool,
) -> Result<()> {
    paths.ensure_dirs()?;
    events::validate_topic(paths, topic).context("validate topic")?;

    let body = match body {
        Some(b) => b.to_string(),
        None => fs_util::read_stdin_body()?,
    };

    let ts = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let id = events::generate_event_id(&ts);
    let event_path = paths.event_file(topic);
    let seq = events::last_seq(&event_path)
        .context("read last seq")?
        + 1;

    let event = Event {
        id: id.clone(),
        topic: topic.to_string(),
        seq,
        kind: kind.to_string(),
        title: title.to_string(),
        body,
        ts,
    };

    events::append_event(&event_path, &event).context("append event")?;

    if json {
        let out = serde_json::json!({ "id": event.id, "topic": topic, "seq": seq });
        println!("{}", serde_json::to_string(&out)?);
    } else {
        println!(
            "  {} {} #{} {}",
            colors::pill(kind),
            colors::bold(topic),
            colors::dim(&seq.to_string()),
            colors::highlight(title),
        );
    }

    Ok(())
}
