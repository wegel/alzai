# agentctl — Repo-Local Agent Knowledge

## What This Is

A minimal system that lets coding agents accumulate durable project knowledge across sessions. Knowledge is stored as an append-only event log plus synthesized per-topic summaries. No daemon, no session tracking, no magic — just structured memory that survives agent restarts and crashes.

### Design Constraints

- **Append-only source of truth.** Every learning is first recorded as an immutable event. Everything else is derived.
- **Topic files are canonical but regenerable.** `facts/<topic>.md` files are the human- and agent-readable summaries. They're produced by LLM synthesis from the event log and can always be rebuilt from scratch.
- **Best-effort durability.** During `sync`, the topic file is written first, then the offset is advanced. If interrupted between the two, the offset is stale and the next `sync` re-synthesizes — wasted work, but no inconsistency.
- **No daemon.** No watcher, no bootstrap ritual, no long-lived process.
- **No task tracking.** The human gives the agent a task directly. This system stores *project knowledge*, not session intent.
- **Topics are human-curated.** The human operator creates topics based on observed patterns — things the agent keeps forgetting or struggling with across sessions. Agents log freely to existing topics, and may propose new ones, but must not create topics unilaterally.
- **`AGENTS.md` tells agents when to log.** The agent is responsible for calling `agentctl log`. There's no way to automatically capture unlogged insights.

---

## How It Works

1. Human starts an agent with a task prompt.
2. Agent reads `AGENTS.md`, which explains the knowledge system and when to log.
3. Agent runs `agentctl sync` to process any pending events from previous sessions.
4. Agent reads relevant `facts/*.md` files to get up to speed.
5. When the agent discovers something durable — an architectural constraint, a pitfall, a decision, an open question — it calls `agentctl log`.
6. `agentctl log` appends the event to the topic's event file and returns immediately. No LLM call, no blocking.
7. Optionally, the agent or human runs `agentctl sync` again before ending the session. This is nice-to-have, not relied upon — the next session's sync will catch anything left pending.

The invariant: if the checkpoint says event N is applied, then `facts/<topic>.md` reflects event N. If `sync` is interrupted, the offset may be stale — the next `sync` will re-synthesize the topic, producing identical results.

---

## Repo Layout

```
repo/
  AGENTS.md
  .agents/
    memory/
      events/
        streaming.jsonl        # append-only, one file per topic
        architecture.jsonl
      facts/
        streaming.md           # synthesized topic summaries
        architecture.md
        ...                    # topics created by human operator
      state/
        topic_offsets.json     # per-topic checkpoint
```

---

## Data Model

### Events

Each event is a single JSONL line with these fields:

| Field   | Description |
|---------|-------------|
| `id`    | `<ISO-timestamp>-<8-hex>`, globally unique |
| `topic` | Slug identifying the topic. Topics are created by the human operator; agents log to existing topics and may propose new ones but must not create them on their own. |
| `seq`   | Monotonically increasing per topic. Next seq = last line of that topic's event file + 1 (or 1 if empty). |
| `kind`  | Free-form string describing the nature of the event (e.g. `fact`, `decision`, `pitfall`, `open_question`, `assumption`, `contradiction`). Not constrained to a fixed enum — agents use whatever kind fits. New kinds emerge naturally as the project evolves. |
| `title` | One-line summary. |
| `body`  | Full description. |
| `ts`    | ISO 8601 timestamp. |

Example:

```json
{
  "id": "2026-04-09T13:02:11Z-7f4a2c1e",
  "topic": "streaming",
  "seq": 42,
  "kind": "pitfall",
  "title": "Framework buffering hid backpressure",
  "body": "hyper/axum consumed buffered data before TCP backpressure reached producer",
  "ts": "2026-04-09T13:02:11Z"
}
```

Topics are created by the human operator when recurring patterns emerge — things the agent keeps getting wrong, architectural constraints that need to survive across sessions, etc. Agents log to existing topics freely and may propose new topics to the operator, but don't create them unilaterally. Kinds are just strings — `"workaround"`, `"regression"`, `"dependency-note"`, whatever is appropriate. The synthesis prompt handles arbitrary kinds by grouping them into sections.

### Topic Offsets

```json
{
  "streaming": {
    "last_event_id": "2026-04-09T13:02:11Z-7f4a2c1e",
    "last_event_seq": 42
  }
}
```

Meaning: all events for this topic with `seq <= last_event_seq` are reflected in the corresponding `facts/*.md` file. Anything above is pending.

---

## `agentctl log`

Appends a durable learning to the event log. Fast, no LLM call — returns immediately.

### Inputs

- `--topic` — must match an existing topic. Errors on unknown topics (catches typos, enforces curation). A topic exists if its `facts/<topic>.md` file exists — `touch` is enough to create one.
- `--kind` — free-form string
- `--title` — one-line summary
- `--body` — full content (or stdin)

### Steps

1. **Validate topic.** Check that `facts/<topic>.md` exists. If not: check whether `events/<topic>.jsonl` exists — if it does, warn that the topic file appears to have been deleted and suggest restoring it with `touch`. Otherwise, error with a clear message listing available topics.
2. **Allocate metadata.** Generate event ID, assign next seq (last line of `events/<topic>.jsonl` + 1, or 1 if new), record timestamp.
3. **Append to event file.** Write the event to `events/<topic>.jsonl` and `fsync`.

That's it. No synthesis, no offset update. The event is durable and pending.

---

## `agentctl sync`

Processes all pending events across all dirty topics. This is where LLM synthesis happens.

### Steps

1. **Find dirty topics.** For each topic, compare the offset in `topic_offsets.json` against the last seq in `events/<topic>.jsonl`. Any topic with events beyond its checkpoint is dirty.
2. **For each dirty topic:**
   a. **Load topic state.** Read current `facts/<topic>.md`, current offset, and all unapplied events.
   b. **LLM synthesis.** Shell out to an LLM CLI (e.g. `codex exec`, `claude`) with the current topic file + unapplied events on stdin. The model rewrites the entire topic file — no patching, no diffing. The specific CLI tool is a runtime configuration concern, not prescribed by this spec.
   c. **Write results.** Write the new topic file to a temp file and rename into place, then write the updated offset to a temp file and rename into place. Topic file lands first, offset second. If interrupted between the two renames, the offset is stale and the next `sync` re-synthesizes — wasted work, no inconsistency.

### Failure Mode

If synthesis or the atomic commit fails for a topic:
- The raw events are still in the log.
- The topic file and offset are unchanged.
- This is "pending unapplied events", not corruption. The next `sync` will pick them up.

---

## `agentctl status`

Shows the current state of all topics. No LLM call, pure filesystem check.

Output includes, per topic: topic name, total event count, pending (unsynthesized) event count, and last sync timestamp. Useful for both agents and humans to decide whether `sync` is needed.

---

## LLM Synthesis

### Scope

Always per-topic. The LLM never touches unrelated topics in a single call.

### Input

- Current `facts/<topic>.md` (may be empty for new topics)
- All unapplied events for that topic

### Output Contract

A complete replacement for `facts/<topic>.md`. The output must:

- Preserve existing valid knowledge
- Incorporate new events
- Deduplicate
- Stay concise
- Explicitly note contradictions
- Avoid speculation and transient chatter
- Organize by event kind — but adapt sections to whatever kinds are present (don't force a fixed template). If a kind has only one or two entries, merge it into a broader section rather than creating a one-line section per kind.

Example output for a topic with `fact`, `pitfall`, `decision`, and `open_question` events:

```md
# streaming

## Facts
- Piece fetch must be downstream-demand-driven.

## Pitfalls
- Framework buffering can hide real downstream backpressure.

## Decisions
- Prefer pull-driven iteration over speculative prefetch.

## Open Questions
- Whether the HTTP body implementation can expose sufficient demand signals.
```

A different topic might have `workaround` and `regression` sections instead — the structure follows the data.

### Reference Prompt

This is the prompt piped to the LLM CLI during `sync`. The implementation should construct it by interpolating the topic name, current file contents, and pending events.

```
You maintain a concise knowledge file for the topic "{topic}".

Below is the current file (may be empty for a new topic), followed by new events to incorporate.

Rules:
- Output ONLY the complete replacement file content. No preamble, no commentary.
- Preserve all existing knowledge that is still valid.
- Incorporate every new event.
- Deduplicate — if a new event restates something already present, keep the better phrasing.
- If two events contradict each other, note the contradiction explicitly.
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
</new_events>
```

---

## Recovery

### From Partial Failure

The next `agentctl sync` will discover unapplied events (via the offset) and include them in synthesis. No manual intervention needed.

### Full Rebuild

The entire knowledge state can be reconstructed from `.agents/memory/events/*.jsonl` alone. A future `agentctl rebuild` command can:

- Ignore all current topic files and offsets
- Replay every event from the beginning
- Regenerate everything

The data model supports this by design. Not required for v1, but the invariant is maintained.

---

## `AGENTS.md`

Lives at repo root. Contains durable instructions for any coding agent:

- Run `agentctl sync` at the start of every session, before reading topic files
- Where knowledge lives (`.agents/memory/facts/`)
- When to call `agentctl log` (architectural discoveries, pitfalls, decisions, invariants, anything worth preserving across sessions)
- That topic files are canonical reading material
- That raw event logs are append-only history (don't edit them)
- That topics are created by the human operator — agents log to existing topics, and may propose new ones to the operator, but must not create them
- That event kinds are free-form — use whatever fits

This file contains workflow instructions only. No session-specific task text.

---

## Concurrency

v1 assumes a single active writer. One agent, one repo, no parallel sessions. The append-only log and per-topic offsets provide a reasonable foundation for multi-writer extension later, but that's out of scope.

---

## What This Doesn't Do

- Track sessions or tasks
- Construct prompts
- Run a background daemon
- Summarize the whole project on every change
- Autonomously discover learnings (the agent must explicitly call `agentctl log`)
- Build cross-topic knowledge graphs

---

## Implementation Surface

v1 ships exactly:

- `AGENTS.md`
- `agentctl log` (append-only, no LLM, rejects unknown topics)
- `agentctl sync` (LLM synthesis for all dirty topics)
- `agentctl status` (filesystem-only, shows pending state)
- Human-curated topics, free-form event kinds

One sentence: **`log` appends the raw event instantly, `sync` synthesizes dirty topics and advances their checkpoints.**
