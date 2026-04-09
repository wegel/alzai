//! Core domain types shared across all modules.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// --- Event log ---

/// A single event in the append-only JSONL log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub topic: String,
    pub seq: u64,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub ts: String,
}

// --- Sync state ---

/// Per-topic sync checkpoint, tracking the last synthesized event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicOffset {
    pub last_event_id: String,
    pub last_event_seq: u64,
}

/// Offsets for all topics, keyed by topic slug.
pub type TopicOffsets = HashMap<String, TopicOffset>;

// --- Status reporting ---

/// Summary of a single topic's state (for `alzai status`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStatus {
    pub name: String,
    pub total_events: u64,
    pub pending_events: u64,
    pub last_event_id: Option<String>,
    pub synced_through_seq: u64,
}
