//! Repo root discovery and `.agents/memory/` path layout.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

// --- Public types ---

/// Resolved paths for the `.agents/memory/` directory tree.
pub struct RepoPaths {
    pub events_dir: PathBuf,
    pub facts_dir: PathBuf,
    pub state_dir: PathBuf,
    pub offsets_path: PathBuf,
}

// --- Public API ---

impl RepoPaths {
    /// Discover the repo root by walking upward from `cwd` looking for `.agents/memory/facts/`.
    pub fn discover() -> Result<Self> {
        let start = std::env::current_dir().context("get current directory")?;
        let mut dir = start.as_path();

        loop {
            let candidate = dir.join(".agents").join("memory").join("facts");
            if candidate.is_dir() {
                return Ok(Self::from_root(dir));
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => bail!(
                    "no .agents/memory/facts/ found in any parent of {}",
                    start.display()
                ),
            }
        }
    }

    /// List all known topics (stems of `facts/*.md` files), sorted.
    pub fn list_topics(&self) -> Result<Vec<String>> {
        let mut topics = Vec::new();
        let entries = fs::read_dir(&self.facts_dir)
            .with_context(|| format!("read {}", self.facts_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                topics.push(stem.to_string());
            }
        }

        topics.sort();
        Ok(topics)
    }

    /// Path to the JSONL event file for a topic.
    pub fn event_file(&self, topic: &str) -> PathBuf {
        self.events_dir.join(format!("{}.jsonl", topic))
    }

    /// Path to the markdown fact file for a topic.
    pub fn fact_file(&self, topic: &str) -> PathBuf {
        self.facts_dir.join(format!("{}.md", topic))
    }

    /// Ensure the events/ and state/ directories exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.events_dir)
            .with_context(|| format!("create {}", self.events_dir.display()))?;
        fs::create_dir_all(&self.state_dir)
            .with_context(|| format!("create {}", self.state_dir.display()))?;
        Ok(())
    }
}

// --- Constructors ---

impl RepoPaths {
    /// Build paths from an explicit `memory/` root. Useful for tests.
    pub fn from_memory_root(memory: &Path) -> Self {
        Self {
            events_dir: memory.join("events"),
            facts_dir: memory.join("facts"),
            state_dir: memory.join("state"),
            offsets_path: memory.join("state").join("topic_offsets.json"),
        }
    }

    fn from_root(root: &Path) -> Self {
        Self::from_memory_root(&root.join(".agents").join("memory"))
    }
}
