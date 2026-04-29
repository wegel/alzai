//! Filesystem utilities: atomic writes and stdin helpers.

use std::fs::{self, File};
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};

// --- Public API ---

/// Write content to a file atomically: temp file in same dir, fsync, rename.
pub fn atomic_write(target: &Path, content: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .context("target path has no parent directory")?;
    fs::create_dir_all(parent)?;

    let tmp = target.with_extension("tmp");
    let mut file = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    file.write_all(content)?;
    file.sync_all()?;

    fs::rename(&tmp, target)
        .with_context(|| format!("rename {} -> {}", tmp.display(), target.display()))?;
    Ok(())
}

/// Read body text from stdin. Errors if stdin is a TTY (interactive).
pub fn read_stdin_body() -> Result<String> {
    if io::stdin().is_terminal() {
        bail!("--body not provided and stdin is a terminal; pipe input or pass --body");
    }
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("read body from stdin")?;
    Ok(buf)
}
