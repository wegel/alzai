//! alzai CLI entry point.

use clap::{Parser, Subcommand};

use alzai::cmd_context;
use alzai::cmd_log;
use alzai::cmd_reflect;
use alzai::cmd_status;
use alzai::cmd_sync;
use alzai::colors;
use alzai::repo;

// --- CLI definition ---

#[derive(Parser)]
#[command(name = "alzai", about = "Repo-local agent knowledge system")]
struct Cli {
    /// Output machine-readable JSON instead of human-friendly text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show session-start instructions for using repo memory.
    Context,
    /// Append a durable learning to the event log.
    Log {
        /// Topic slug (must match an existing facts/<topic>.md file).
        #[arg(long)]
        topic: String,
        /// Free-form kind (e.g. fact, decision, pitfall, open_question).
        #[arg(long)]
        kind: String,
        /// One-line summary.
        #[arg(long)]
        title: String,
        /// Full description (reads from stdin if omitted).
        #[arg(long)]
        body: Option<String>,
    },
    /// Synthesize all dirty topics via LLM.
    Sync {
        /// LLM CLI command (overrides ALZAI_LLM_CMD env var).
        #[arg(long)]
        llm_cmd: Option<String>,
    },
    /// Prompt whether this session produced durable knowledge worth logging.
    Reflect,
    /// Show per-topic event counts and sync state.
    Status,
}

// --- Entry point ---

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    colors::set_json_mode(cli.json);

    let paths = repo::RepoPaths::discover()?;

    match cli.command {
        Commands::Context => cmd_context::run(&paths, cli.json),
        Commands::Log {
            topic,
            kind,
            title,
            body,
        } => cmd_log::run(&paths, &topic, &kind, &title, body.as_deref(), cli.json),
        Commands::Sync { llm_cmd } => cmd_sync::run(&paths, llm_cmd.as_deref(), cli.json),
        Commands::Reflect => cmd_reflect::run(&paths, cli.json),
        Commands::Status => cmd_status::run(&paths, cli.json),
    }
}
