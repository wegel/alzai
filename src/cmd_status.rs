//! Handler for `alzai status`: show per-topic event counts and sync state.

use anyhow::{Context, Result};

use crate::colors;
use crate::events;
use crate::offsets;
use crate::repo::RepoPaths;
use crate::types::TopicStatus;

// --- Public API ---

/// Run the status command: pure filesystem check, no LLM.
pub fn run(paths: &RepoPaths, json: bool) -> Result<()> {
    let statuses = collect_statuses(paths)?;

    if statuses.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("  {}", colors::dim("no topics found"));
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string(&statuses)?);
        return Ok(());
    }

    print_table(&statuses);
    Ok(())
}

/// Collect topic status without printing.
pub fn collect_statuses(paths: &RepoPaths) -> Result<Vec<TopicStatus>> {
    let topic_offsets = offsets::load_offsets(&paths.offsets_path).context("load offsets")?;
    let topics = paths.list_topics().context("list topics")?;

    let mut statuses = Vec::new();
    for topic in &topics {
        let event_path = paths.event_file(topic);
        let total = events::last_seq(&event_path).unwrap_or(0);
        let synced = offsets::topic_seq(&topic_offsets, topic);
        let pending = total.saturating_sub(synced);
        let last_id = topic_offsets
            .get(topic.as_str())
            .map(|o| o.last_event_id.clone());

        statuses.push(TopicStatus {
            name: topic.clone(),
            total_events: total,
            pending_events: pending,
            last_event_id: last_id,
            synced_through_seq: synced,
        });
    }

    Ok(statuses)
}

// --- Private helpers ---

fn print_table(statuses: &[TopicStatus]) {
    let name_width = statuses
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(10)
        .max(5);

    println!(
        "  {:<width$}  {:>5}  {:>7}  {:>7}",
        colors::bold("topic"),
        colors::bold("total"),
        colors::bold("pending"),
        colors::bold("synced"),
        width = name_width,
    );

    for s in statuses {
        let pending_str = if s.pending_events > 0 {
            colors::warn(&s.pending_events.to_string())
        } else {
            colors::ok("0")
        };
        println!(
            "  {:<width$}  {:>5}  {:>7}  {:>7}",
            s.name,
            s.total_events,
            pending_str,
            s.synced_through_seq,
            width = name_width,
        );
    }
}
