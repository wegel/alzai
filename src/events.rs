//! Event log I/O: append, read, validate topics, generate IDs.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::repo::RepoPaths;
use crate::types::Event;

// --- Error types ---

/// Errors from event operations.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("unknown topic '{topic}'. Available: {available}")]
    TopicNotFound { topic: String, available: String },

    #[error("topic file missing but events exist for '{topic}' — restore with: touch {fact_path}")]
    OrphanedEvents { topic: String, fact_path: String },

    #[error("malformed event at {path}:{line}: {detail}")]
    MalformedEvent {
        path: PathBuf,
        line: usize,
        detail: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// --- Public API ---

/// Validate that a topic exists (its `facts/<topic>.md` file is present).
pub fn validate_topic(paths: &RepoPaths, topic: &str) -> Result<(), EventError> {
    let fact_path = paths.fact_file(topic);
    if fact_path.exists() {
        return Ok(());
    }

    // Check if orphaned events exist without a fact file.
    let event_path = paths.event_file(topic);
    if event_path.exists() {
        return Err(EventError::OrphanedEvents {
            topic: topic.to_string(),
            fact_path: fact_path.display().to_string(),
        });
    }

    let available = paths.list_topics().unwrap_or_default().join(", ");
    Err(EventError::TopicNotFound {
        topic: topic.to_string(),
        available: if available.is_empty() {
            "(none)".to_string()
        } else {
            available
        },
    })
}

/// Generate a new event ID: `<ISO-timestamp>-<8-hex>`.
pub fn generate_event_id(ts: &str) -> String {
    let hex: u32 = rand::random();
    format!("{}-{:08x}", ts, hex)
}

/// Read the last seq number from a topic's event file. Returns 0 if absent or empty.
pub fn last_seq(event_path: &Path) -> Result<u64, EventError> {
    if !event_path.exists() {
        return Ok(0);
    }

    let file = File::open(event_path)?;
    let reader = BufReader::new(file);
    let mut last = 0u64;

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(trimmed) {
            Ok(event) => last = event.seq,
            Err(e) => {
                return Err(EventError::MalformedEvent {
                    path: event_path.to_path_buf(),
                    line: i + 1,
                    detail: e.to_string(),
                });
            }
        }
    }

    Ok(last)
}

/// Append a single event to the topic's JSONL file, with fsync.
pub fn append_event(event_path: &Path, event: &Event) -> Result<(), EventError> {
    if let Some(parent) = event_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(event_path)?;

    let mut line = serde_json::to_string(event).map_err(|e| std::io::Error::other(e))?;
    line.push('\n');

    file.write_all(line.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Read all events from a topic's JSONL file. Returns empty vec if file is absent.
pub fn read_all_events(event_path: &Path) -> Result<Vec<Event>, EventError> {
    if !event_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(event_path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event: Event =
            serde_json::from_str(trimmed).map_err(|e| EventError::MalformedEvent {
                path: event_path.to_path_buf(),
                line: i + 1,
                detail: e.to_string(),
            })?;
        events.push(event);
    }

    Ok(events)
}

/// Read events with `seq > after_seq` from a topic's JSONL file.
pub fn read_events_after(event_path: &Path, after_seq: u64) -> Result<Vec<Event>, EventError> {
    Ok(read_all_events(event_path)?
        .into_iter()
        .filter(|e| e.seq > after_seq)
        .collect())
}
