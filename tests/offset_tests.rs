//! Tests for topic offset operations.

use std::fs;

use alzai::offsets;
use alzai::types::{TopicOffset, TopicOffsets};
use tempfile::TempDir;

// --- Tests ---

#[test]
fn load_returns_empty_for_missing_file() {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("offsets.json");
    let offsets = offsets::load_offsets(&path).expect("load");
    assert!(offsets.is_empty());
}

#[test]
fn save_and_load_roundtrip() {
    let dir = TempDir::new().expect("create temp dir");
    fs::create_dir_all(dir.path().join("state")).expect("create state dir");
    let path = dir.path().join("state").join("offsets.json");

    let mut offsets = TopicOffsets::new();
    offsets.insert(
        "streaming".to_string(),
        TopicOffset {
            last_event_id: "2026-04-09T00:00:00Z-12345678".to_string(),
            last_event_seq: 42,
        },
    );

    offsets::save_offsets(&path, &offsets).expect("save");
    let loaded = offsets::load_offsets(&path).expect("load");

    assert_eq!(loaded.len(), 1);
    let entry = &loaded["streaming"];
    assert_eq!(entry.last_event_seq, 42);
    assert_eq!(entry.last_event_id, "2026-04-09T00:00:00Z-12345678");
}

#[test]
fn topic_seq_returns_zero_for_absent() {
    let offsets = TopicOffsets::new();
    assert_eq!(offsets::topic_seq(&offsets, "missing"), 0);
}

#[test]
fn topic_seq_returns_stored_value() {
    let mut offsets = TopicOffsets::new();
    offsets.insert(
        "arch".to_string(),
        TopicOffset {
            last_event_id: "id".to_string(),
            last_event_seq: 7,
        },
    );
    assert_eq!(offsets::topic_seq(&offsets, "arch"), 7);
}
