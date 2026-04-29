//! Tests for memory reflection output.

use alzai::cmd_reflect;
use alzai::types::TopicStatus;
use serde_json::Value;

// --- Helpers ---

fn topic(name: &str, pending_events: u64) -> TopicStatus {
    TopicStatus {
        name: name.to_string(),
        total_events: pending_events,
        pending_events,
        last_event_id: None,
        synced_through_seq: 0,
    }
}

// --- Tests ---

#[test]
fn text_reflection_lists_topics_and_logging_triggers() {
    let statuses = vec![topic("architecture", 0), topic("pitfalls", 2)];
    let rendered = cmd_reflect::render_text(&statuses);

    assert!(rendered.contains("Memory reflection"));
    assert!(rendered.contains("Available topics:"));
    assert!(rendered.contains("  - architecture"));
    assert!(rendered.contains("  - pitfalls"));
    assert!(rendered.contains("Log something if this session produced:"));
    assert!(rendered.contains("a durable design decision or tradeoff"));
    assert!(rendered.contains("Do not log:"));
    assert!(rendered.contains("ordinary task progress"));
    assert!(rendered.contains("alzai log --topic <topic>"));
}

#[test]
fn text_reflection_warns_when_events_are_pending_sync() {
    let statuses = vec![topic("architecture", 3)];
    let rendered = cmd_reflect::render_text(&statuses);

    assert!(rendered.contains("3 pending event(s)"));
    assert!(rendered.contains("alzai sync"));
}

#[test]
fn text_reflection_handles_no_topics() {
    let rendered = cmd_reflect::render_text(&[]);

    assert!(rendered.contains("Available topics:"));
    assert!(rendered.contains("  none"));
}

#[test]
fn json_reflection_reports_topics_and_policy() {
    let statuses = vec![topic("architecture", 1), topic("pitfalls", 2)];
    let rendered = cmd_reflect::render_json(&statuses).expect("render json");
    let value: Value = serde_json::from_str(&rendered).expect("parse json");

    assert_eq!(value["pending_events"], 3);
    assert_eq!(value["topics"][0]["name"], "architecture");
    assert_eq!(value["topics"][1]["name"], "pitfalls");
    assert_eq!(
        value["log_when"][0],
        "a durable design decision or tradeoff"
    );
    assert_eq!(value["do_not_log"][0], "ordinary task progress");
    assert_eq!(
        value["command"],
        "alzai log --topic <topic> --kind <kind> --title \"...\" --body \"...\"",
    );
}
