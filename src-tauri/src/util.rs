// Utility helpers for locating external binaries used by nvAge
// ---------------------------------------------------------------
// These helpers first check for an explicit environment variable so that
// the binary location can be forced (useful for packaged RPM builds),
// then fall back to a short list of common installation paths, and finally
// to the plain command name which relies on the process' PATH.

use std::process::Command;

/// Locate the `git` executable.
/// Returns an absolute path or the command name if it can be executed.
pub fn locate_git() -> Result<String, anyhow::Error> {
    if let Ok(p) = std::env::var("NVAGE_GIT_PATH") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    let candidates = [
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
        "/home/linuxbrew/.linuxbrew/bin/git",
        "git",
    ];
    for path in &candidates {
        let out = Command::new(path).arg("--version").output();
        if let Ok(o) = out {
            if o.status.success() {
                return Ok(path.to_string());
            }
        }
    }
    anyhow::bail!("Git is not installed. Install Git to use sync.")
}

/// Locate the `age` executable.
/// Mirrors `locate_git` but for the `age` binary.
pub fn locate_age() -> Result<String, anyhow::Error> {
    if let Ok(p) = std::env::var("NVAGE_AGE_PATH") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    let candidates = [
        "/usr/bin/age",
        "/usr/local/bin/age",
        "/opt/homebrew/bin/age",
        "/home/linuxbrew/.linuxbrew/bin/age",
        "age",
    ];
    for path in &candidates {
        let out = Command::new(path).arg("--version").output();
        if let Ok(o) = out {
            if o.status.success() {
                return Ok(path.to_string());
            }
        }
    }
    anyhow::bail!("`age` is not installed. Install the `age` package to use encrypted sync.")
}
