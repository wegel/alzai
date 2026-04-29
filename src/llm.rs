//! LLM CLI invocation and synthesis prompt construction.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::types::Event;

// --- Synthesis prompt template ---

const SYNC_PROMPT: &str = r#"You maintain a concise knowledge file for the topic "{topic}".

Below is the current file (may be empty for a new topic), followed by new events to incorporate.

Rules:
- Output ONLY the complete replacement file content. No preamble, no commentary.
- Preserve all existing knowledge that is still valid.
- Incorporate every new event.
- Deduplicate — if a new event restates something already present, keep the better phrasing.
- If two events contradict each other, note the contradiction explicitly.
- Remove entries that have been resolved, completed, or made obsolete by newer events. A gap that has been filled, a question that has been answered, or a pitfall that has been fixed should be removed entirely — not kept as historical record.
- Remove anything that is speculative, transient, or task-specific.
- Organize into sections by event kind (## Facts, ## Pitfalls, ## Decisions, etc.).
- Adapt sections to whatever kinds are present — don't force a fixed template.
- If a kind has only one or two entries, merge it into a related section rather than giving it its own heading.
- Keep each entry to 1-2 lines. The entire file should stay compact.

<current_file>
{current_file_contents}
</current_file>

<new_events>
{events_as_json}
</new_events>"#;

// --- Public API ---

/// Resolve the LLM CLI command from flag or environment variable.
pub fn resolve_llm_cmd(flag: Option<&str>) -> Result<String> {
    if let Some(cmd) = flag {
        return Ok(cmd.to_string());
    }
    if let Ok(cmd) = std::env::var("ALZAI_LLM_CMD")
        && !cmd.is_empty()
    {
        return Ok(cmd);
    }
    bail!(
        "no LLM command configured. Set ALZAI_LLM_CMD or pass --llm-cmd.\n\
         Example: ALZAI_LLM_CMD=\"claude -p\" alzai sync"
    )
}

/// Build the synthesis prompt for a topic.
pub fn build_sync_prompt(topic: &str, current_contents: &str, events: &[Event]) -> String {
    let events_json = serde_json::to_string_pretty(events).unwrap_or_default();
    SYNC_PROMPT
        .replace("{topic}", topic)
        .replace("{current_file_contents}", current_contents.trim())
        .replace("{events_as_json}", &events_json)
}

/// Run the LLM CLI with the given prompt on stdin, return stdout as the response.
pub fn run_llm(cmd: &str, prompt: &str) -> Result<String> {
    let mut child = Command::new("sh")
        .args(["-c", cmd])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawn LLM command: sh -c '{}'", cmd))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .context("write prompt to LLM stdin")?;
    }

    let output = child.wait_with_output().context("wait for LLM process")?;

    if !output.status.success() {
        bail!(
            "LLM command exited with status {}",
            output.status.code().unwrap_or(-1)
        );
    }

    let response = String::from_utf8(output.stdout).context("LLM output is not valid UTF-8")?;
    Ok(response)
}
