//! Topic offset checkpoint: load and save `state/topic_offsets.json`.

use std::path::Path;

use anyhow::{Context, Result};

use crate::fs_util;
use crate::types::{TopicOffset, TopicOffsets};

// --- Public API ---

/// Load topic offsets from disk. Returns an empty map if the file doesn't exist.
pub fn load_offsets(path: &Path) -> Result<TopicOffsets> {
    if !path.exists() {
        return Ok(TopicOffsets::new());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;

    if content.trim().is_empty() {
        return Ok(TopicOffsets::new());
    }

    serde_json::from_str(&content)
        .with_context(|| format!("parse {}", path.display()))
}

/// Save topic offsets atomically (temp file + rename).
pub fn save_offsets(path: &Path, offsets: &TopicOffsets) -> Result<()> {
    let json = serde_json::to_string_pretty(offsets)
        .context("serialize topic offsets")?;
    fs_util::atomic_write(path, json.as_bytes())
        .context("write topic offsets")
}

/// Get the synced seq for a topic, returning 0 if absent.
pub fn topic_seq(offsets: &TopicOffsets, topic: &str) -> u64 {
    offsets.get(topic).map_or(0, |o| o.last_event_seq)
}

/// Update the offset for a single topic.
pub fn update_topic(offsets: &mut TopicOffsets, topic: &str, offset: TopicOffset) {
    offsets.insert(topic.to_string(), offset);
}
