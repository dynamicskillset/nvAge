use std::path::{Path, PathBuf};

/// Status of the sync provider
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SyncStatus {
    /// No sync configured
    NotConfigured,
    /// Idle, no sync in progress
    Idle,
    /// Sync is currently running
    Syncing,
    /// Last sync failed with an error
    Error(String),
    /// Conflicts detected during last sync
    Conflict(Vec<PathBuf>),
}

/// Result of a sync operation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SyncResult {
    pub status: SyncStatus,
    pub notes_pushed: usize,
    pub notes_pulled: usize,
    pub conflicts: Vec<PathBuf>,
}

/// Abstraction for sync providers.
///
/// Implementations handle encrypting notes on push and decrypting on pull.
/// The v1 implementation shells out to the `git` CLI.
pub trait SyncProvider: Send + Sync {
    /// Check if this provider is properly configured
    fn is_configured(&self) -> bool;

    /// Get current sync status
    fn status(&self) -> SyncStatus;

    /// Push local changes to the remote.
    ///
    /// Encrypts changed notes into a staging area, stages, commits, and pushes.
    /// Returns the number of notes pushed.
    fn push(&self, notes_folder: &Path, key_path: &Path) -> Result<usize, anyhow::Error>;

    /// Pull remote changes and decrypt into the notes folder.
    ///
    /// Fetches, pulls, decrypts changed files into the notes folder.
    /// Returns the number of notes pulled and any conflict paths.
    fn pull(
        &self,
        notes_folder: &Path,
        key_path: &Path,
    ) -> Result<(usize, Vec<PathBuf>), anyhow::Error>;

    /// Run a full sync cycle: push then pull.
    #[allow(dead_code)]
    fn sync(&self, notes_folder: &Path, key_path: &Path) -> Result<SyncResult, anyhow::Error> {
        let notes_pushed = self.push(notes_folder, key_path)?;
        let (notes_pulled, conflicts) = self.pull(notes_folder, key_path)?;

        let status = if conflicts.is_empty() {
            SyncStatus::Idle
        } else {
            SyncStatus::Conflict(conflicts.clone())
        };

        Ok(SyncResult {
            status,
            notes_pushed,
            notes_pulled,
            conflicts,
        })
    }
}
