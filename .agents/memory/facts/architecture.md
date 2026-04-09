

## Facts

- **Lib + bin crate split:** `src/lib.rs` re-exports all modules as `pub`; `src/main.rs` is a thin CLI wrapper. Required for integration tests in `tests/` to access internals.
- **Atomic write pattern:** `facts/*.md` and `state/topic_offsets.json` use temp + fsync + rename. Event log uses append + fsync. Fact file is written before offset — if interrupted, next sync re-synthesizes (wasted work, no inconsistency).
- **Repo discovery walks upward:** `RepoPaths::discover()` walks from cwd upward looking for `.agents/memory/facts/` directory, similar to how git finds `.git/`.

## Decisions

- **LLM invoked via `sh -c` with stdin/stdout:** `alzai sync` shells out to an LLM CLI configured via `ALZAI_LLM_CMD` env var or `--llm-cmd` flag. Prompt piped to stdin, response read from stdout. No embedded LLM, no HTTP API — keeps the tool simple and LLM-agnostic.
