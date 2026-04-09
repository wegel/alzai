//! Handler for `alzai sync`: synthesize dirty topics via LLM.

use anyhow::{Context, Result};

use crate::colors;
use crate::events;
use crate::fs_util;
use crate::llm;
use crate::offsets;
use crate::repo::RepoPaths;
use crate::types::{TopicOffset, TopicOffsets};

// --- Public API ---

/// Run the sync command: find dirty topics, synthesize, update facts and offsets.
pub fn run(paths: &RepoPaths, llm_cmd: Option<&str>, json: bool) -> Result<()> {
    let cmd = llm::resolve_llm_cmd(llm_cmd)?;
    paths.ensure_dirs()?;

    let mut topic_offsets = offsets::load_offsets(&paths.offsets_path)
        .context("load offsets")?;
    let topics = paths.list_topics().context("list topics")?;

    let dirty = find_dirty_topics(paths, &topics, &topic_offsets)?;

    if dirty.is_empty() {
        if json {
            println!(r#"{{"synced":[],"skipped":[]}}"#);
        } else {
            println!("  {}", colors::dim("all topics up to date"));
        }
        return Ok(());
    }

    let mut synced = Vec::new();
    for (topic, total_seq, synced_seq) in &dirty {
        let pending_count = total_seq - synced_seq;
        if !json {
            eprintln!(
                "  {} {} ({} pending)",
                colors::highlight("syncing"),
                colors::bold(topic),
                pending_count,
            );
        }

        let offset = sync_topic(paths, &cmd, topic, *synced_seq)?;
        offsets::update_topic(&mut topic_offsets, topic, offset);
        offsets::save_offsets(&paths.offsets_path, &topic_offsets)
            .context("save offsets")?;

        synced.push(serde_json::json!({
            "topic": topic,
            "events_applied": pending_count,
        }));
    }

    if json {
        let out = serde_json::json!({ "synced": synced, "skipped": [] });
        println!("{}", serde_json::to_string(&out)?);
    } else {
        println!(
            "  {} synced {} topic(s)",
            colors::ok("done"),
            synced.len(),
        );
    }

    Ok(())
}

// --- Private helpers ---

/// Returns (topic, total_seq, synced_seq) for each dirty topic.
fn find_dirty_topics(
    paths: &RepoPaths,
    topics: &[String],
    topic_offsets: &TopicOffsets,
) -> Result<Vec<(String, u64, u64)>> {
    let mut dirty = Vec::new();
    for topic in topics {
        let event_path = paths.event_file(topic);
        let total = events::last_seq(&event_path).unwrap_or(0);
        let synced = offsets::topic_seq(topic_offsets, topic);
        if total > synced {
            dirty.push((topic.clone(), total, synced));
        }
    }
    Ok(dirty)
}

/// Synthesize a single topic: read state, call LLM, write results.
fn sync_topic(
    paths: &RepoPaths,
    cmd: &str,
    topic: &str,
    synced_seq: u64,
) -> Result<TopicOffset> {
    let fact_path = paths.fact_file(topic);
    let current_contents = if fact_path.exists() {
        std::fs::read_to_string(&fact_path)
            .with_context(|| format!("read {}", fact_path.display()))?
    } else {
        String::new()
    };

    let event_path = paths.event_file(topic);
    let pending = events::read_events_after(&event_path, synced_seq)
        .context("read pending events")?;

    if pending.is_empty() {
        return Ok(TopicOffset {
            last_event_id: String::new(),
            last_event_seq: synced_seq,
        });
    }

    let last = pending.last().expect("pending is non-empty");
    let offset = TopicOffset {
        last_event_id: last.id.clone(),
        last_event_seq: last.seq,
    };

    let prompt = llm::build_sync_prompt(topic, &current_contents, &pending);
    let response = llm::run_llm(cmd, &prompt)
        .with_context(|| format!("LLM synthesis for topic '{}'", topic))?;

    // Write fact file first, then offset — spec-mandated order.
    fs_util::atomic_write(&fact_path, response.as_bytes())
        .with_context(|| format!("write {}", fact_path.display()))?;

    Ok(offset)
}
