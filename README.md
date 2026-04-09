# alzai

Repo-local knowledge system for coding agents. Agents log durable learnings (architectural constraints, pitfalls, decisions) as append-only events. An LLM synthesizes them into concise per-topic summaries that survive across sessions.

No daemon, no database, no magic — just JSONL event files and markdown summaries checked into your repo.

## How it works

1. A human creates **topics** — things the agent keeps forgetting or getting wrong.
2. Agents call `alzai log` to record learnings as they work. This is instant (no LLM call).
3. `alzai sync` feeds pending events to an LLM, which rewrites each topic's summary file.
4. Next session, the agent reads `facts/*.md` to get up to speed immediately.

Events are the source of truth. Summaries are derived and can always be rebuilt.

## Setup

### Install

```
cargo install --path .
```

### Initialize a repo

Create the directory structure and at least one topic:

```
mkdir -p .agents/memory/{events,facts,state}
touch .agents/memory/facts/architecture.md
```

A topic exists when its `facts/<slug>.md` file exists. Create as many as needed:

```
touch .agents/memory/facts/streaming.md
touch .agents/memory/facts/testing.md
```

### Configure the LLM

Set the command that `alzai sync` will use to call an LLM. The prompt is piped to stdin; the response is read from stdout.

```
export ALZAI_LLM_CMD="claude -p"
```

Or pass it per invocation:

```
alzai sync --llm-cmd "claude -p"
```

## Commands

### `alzai log`

Append a learning to a topic's event log.

```
alzai log --topic architecture --kind decision \
  --title "Use pull-based streaming" \
  --body "Push-based approach caused backpressure issues in production"
```

If `--body` is omitted, reads from stdin.

### `alzai sync`

Synthesize all topics with pending events.

```
alzai sync
```

For each dirty topic, the LLM rewrites `facts/<topic>.md` incorporating new events. Offsets are checkpointed per topic — if interrupted, the next sync picks up where it left off.

### `alzai status`

Show what needs syncing.

```
alzai status
```

```
  topic          total  pending   synced
  architecture       5        2        3
  streaming         12        0       12
```

### JSON output

All commands accept `--json` for machine-readable output:

```
alzai --json status
alzai --json log --topic arch --kind fact --title "..." --body "..."
```

## Repo layout

```
.agents/
  memory/
    events/
      architecture.jsonl   # append-only event log
      streaming.jsonl
    facts/
      architecture.md      # synthesized summary (human + agent readable)
      streaming.md
    state/
      topic_offsets.json   # per-topic sync checkpoint
```

- **events/** — append-only JSONL, one file per topic. Don't edit these.
- **facts/** — markdown summaries produced by `sync`. Read these to get context.
- **state/** — internal bookkeeping. Tracks which events have been synthesized.

## Event format

Each JSONL line:

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

- **kind** is free-form — use `fact`, `decision`, `pitfall`, `open_question`, `workaround`, or whatever fits.
- **topics** are created by the human operator. Agents log to existing topics.

## AGENTS.md

Place this at your repo root to tell agents how to use the knowledge system:

````markdown
# Agent Knowledge System

This repo uses `alzai` for durable project knowledge across sessions.

## At session start

1. Run `alzai sync` to process any pending events from previous sessions.
2. Run `alzai status` to see available topics.
3. Read relevant `.agents/memory/facts/*.md` files to get up to speed.

## During your session

When you discover something worth preserving across sessions — an architectural
constraint, a pitfall, a decision rationale, an invariant, an open question —
log it:

```
alzai log --topic <topic> --kind <kind> --title "one-line summary" --body "full description"
```

### What to log

- Architectural constraints and invariants
- Pitfalls that cost you time (so the next session doesn't repeat the mistake)
- Decisions and their rationale
- Workarounds for known issues
- Open questions that need future investigation

### What NOT to log

- Transient task details (what you're currently doing)
- Things obvious from reading the code
- Speculative ideas without evidence

### Kinds

Use whatever kind fits: `fact`, `decision`, `pitfall`, `open_question`,
`assumption`, `workaround`, `contradiction`, etc.

### Topics

Log to existing topics only. Run `alzai status` to see available topics.
If you think a new topic is needed, propose it to the human operator — don't
create it yourself.

## At session end

Optionally run `alzai sync` to synthesize any events you logged. If you forget,
the next session's sync will catch them.

## Important

- `facts/*.md` files are your reading material — they are the synthesized summaries.
- `events/*.jsonl` files are append-only history — never edit them.
- The raw event log is the source of truth. Summaries can always be rebuilt.
````
