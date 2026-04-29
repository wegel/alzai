//! Tests for session-start memory context output.

use alzai::cmd_context;
use alzai::types::TopicStatus;
use serde_json::Value;

// --- Helpers ---

fn topic(name: &str, total_events: u64, pending_events: u64) -> TopicStatus {
    TopicStatus {
        name: name.to_string(),
        total_events,
        pending_events,
        last_event_id: None,
        synced_through_seq: total_events.saturating_sub(pending_events),
    }
}

// --- Tests ---

#[test]
fn text_context_teaches_agent_memory_workflow() {
    let statuses = vec![topic("architecture", 5, 1), topic("pitfalls", 2, 0)];
    let rendered = cmd_context::render_text(&statuses);

    assert!(rendered.contains("Alzai context"));
    assert!(rendered.contains("This repo uses alzai for durable agent memory."));
    assert!(rendered.contains("At session start:"));
    assert!(rendered.contains("Available topics:"));
    assert!(rendered.contains("architecture"));
    assert!(rendered.contains(".agents/memory/facts/architecture.md"));
    assert!(rendered.contains("1 pending event(s)"));
    assert!(rendered.contains("When to log:"));
    assert!(rendered.contains("decisions and tradeoffs"));
    assert!(rendered.contains("Do not log:"));
    assert!(rendered.contains("transient task progress"));
    assert!(rendered.contains("How to log:"));
    assert!(rendered.contains("alzai log --topic <topic>"));
    assert!(rendered.contains("At task end:"));
    assert!(rendered.contains("alzai reflect"));
}

#[test]
fn text_context_handles_no_topics() {
    let rendered = cmd_context::render_text(&[]);

    assert!(rendered.contains("Available topics:"));
    assert!(rendered.contains("  none"));
    assert!(rendered.contains("Log only to existing topics."));
}

#[test]
fn json_context_reports_topics_and_instructions() {
    let statuses = vec![topic("architecture", 5, 1), topic("pitfalls", 2, 0)];
    let rendered = cmd_context::render_json(&statuses).expect("render json");
    let value: Value = serde_json::from_str(&rendered).expect("parse json");

    assert_eq!(
        value["summary"],
        "This repo uses alzai for durable agent memory."
    );
    assert_eq!(value["pending_events"], 1);
    assert_eq!(value["topics"][0]["name"], "architecture");
    assert_eq!(
        value["topics"][0]["fact_path"],
        ".agents/memory/facts/architecture.md"
    );
    assert_eq!(value["when_to_log"][1], "decisions and tradeoffs");
    assert_eq!(value["do_not_log"][0], "transient task progress");
    assert_eq!(value["kinds"][0], "fact");
    assert_eq!(
        value["log_command"],
        "alzai log --topic <topic> --kind <kind> --title \"...\" --body \"...\"",
    );
    assert_eq!(
        value["end_session"][0],
        "Run `alzai reflect` before finishing substantial work."
    );
}
