//! Tests for event log operations.

use std::fs;

use alzai::events::{self, EventError};
use alzai::repo::RepoPaths;
use alzai::types::Event;
use tempfile::TempDir;

// --- Helpers ---

fn setup() -> (TempDir, RepoPaths) {
    let dir = TempDir::new().expect("create temp dir");
    let paths = RepoPaths::from_memory_root(&dir.path().join("memory"));
    fs::create_dir_all(&paths.facts_dir).expect("create facts dir");
    fs::create_dir_all(&paths.events_dir).expect("create events dir");
    fs::create_dir_all(&paths.state_dir).expect("create state dir");
    (dir, paths)
}

fn make_event(topic: &str, seq: u64, kind: &str) -> Event {
    Event {
        id: format!("2026-04-09T00:00:00Z-{:08x}", seq),
        topic: topic.to_string(),
        seq,
        kind: kind.to_string(),
        title: format!("Event {}", seq),
        body: format!("Body for event {}", seq),
        ts: "2026-04-09T00:00:00Z".to_string(),
    }
}

// --- Tests ---

#[test]
fn last_seq_returns_zero_for_missing_file() {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("nonexistent.jsonl");
    assert_eq!(events::last_seq(&path).expect("last_seq"), 0);
}

#[test]
fn append_and_read_events() {
    let (_dir, paths) = setup();
    let event_path = paths.event_file("test");

    let e1 = make_event("test", 1, "fact");
    let e2 = make_event("test", 2, "pitfall");
    events::append_event(&event_path, &e1).expect("append e1");
    events::append_event(&event_path, &e2).expect("append e2");

    let all = events::read_all_events(&event_path).expect("read all");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].seq, 1);
    assert_eq!(all[1].seq, 2);
    assert_eq!(events::last_seq(&event_path).expect("last_seq"), 2);
}

#[test]
fn read_events_after_filters_correctly() {
    let (_dir, paths) = setup();
    let event_path = paths.event_file("test");

    for seq in 1..=5 {
        let e = make_event("test", seq, "fact");
        events::append_event(&event_path, &e).expect("append");
    }

    let after_3 = events::read_events_after(&event_path, 3).expect("read after 3");
    assert_eq!(after_3.len(), 2);
    assert_eq!(after_3[0].seq, 4);
    assert_eq!(after_3[1].seq, 5);
}

#[test]
fn validate_topic_rejects_unknown() {
    let (_dir, paths) = setup();
    fs::write(paths.fact_file("real"), "").expect("create topic");

    let err = events::validate_topic(&paths, "fake").unwrap_err();
    assert!(matches!(err, EventError::TopicNotFound { .. }));
}

#[test]
fn validate_topic_accepts_existing() {
    let (_dir, paths) = setup();
    fs::write(paths.fact_file("real"), "").expect("create topic");
    events::validate_topic(&paths, "real").expect("should succeed");
}

#[test]
fn event_id_format() {
    let ts = "2026-04-09T13:02:11Z";
    let id = events::generate_event_id(ts);
    assert!(id.starts_with(ts), "ID should start with timestamp");
    assert_eq!(id.len(), ts.len() + 1 + 8, "ID should be ts-8hex");
}
