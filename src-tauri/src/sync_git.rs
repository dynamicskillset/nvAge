use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::crypto;
use crate::note;
use crate::sync_provider::{SyncProvider, SyncStatus};

/// Find the git binary. Tauri apps don't inherit shell PATH, so we
/// search common locations before falling back to just "git".
pub fn find_git() -> Result<String, anyhow::Error> {
    let candidates = [
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
        "/home/linuxbrew/.linuxbrew/bin/git",
        "git",
    ];

    for path in &candidates {
        let output = Command::new(path).arg("--version").output();
        if let Ok(o) = output {
            if o.status.success() {
                return Ok(path.to_string());
            }
        }
    }

    anyhow::bail!("Git is not installed. Install Git to use sync.")
}

/// Git-based sync provider that shells out to the `git` CLI.
///
/// Push cycle:
/// 1. Clone or open the sync repo
/// 2. For each changed note, encrypt it to `<uuid>.md.age` in the repo
/// 3. Remove orphaned .md.age files (permanently deleted locally)
/// 4. Stage, commit, push
///
/// Pull cycle:
/// 1. Fetch and pull
/// 2. Detect remote deletions and soft-delete matching local notes
/// 3. Decrypt any `.md.age` files into the notes folder
/// 4. Return conflict paths if local notes have also changed
pub struct GitSyncProvider {
    /// Remote repo URL (e.g. `git@github.com:user/nvage-sync.git`)
    pub remote_url: String,
    /// Branch to sync on
    pub branch: String,
    /// Local path to the cloned sync repo (hidden working tree)
    pub repo_path: PathBuf,
}

impl GitSyncProvider {
    pub fn new(remote_url: String, branch: String, repo_path: PathBuf) -> Self {
        Self {
            remote_url,
            branch,
            repo_path,
        }
    }

    /// Ensure the repo exists: clone if missing, otherwise fetch.
    fn ensure_repo(&self) -> Result<(), anyhow::Error> {
        if self.repo_path.join(".git").exists() {
            self.git(&["fetch"])?;
            Ok(())
        } else {
            if let Some(parent) = self.repo_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let git_path = find_git()?;
            let output = Command::new(&git_path)
                .args([
                    "clone",
                    "--branch",
                    &self.branch,
                    "--single-branch",
                    &self.remote_url,
                    self.repo_path.to_str().unwrap(),
                ])
                .current_dir(self.repo_path.parent().unwrap_or(&self.repo_path))
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git clone failed: {}", stderr);
            }
            Ok(())
        }
    }

    /// Run a git command in the repo directory.
    fn git(&self, args: &[&str]) -> Result<std::process::Output, anyhow::Error> {
        let git_path = find_git()?;
        let output = Command::new(&git_path)
            .args(args)
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git {:?} failed: {}", args, stderr);
        }

        Ok(output)
    }

    /// Get the public key from the secret key file.
    fn public_key(key_path: &Path) -> Result<String, anyhow::Error> {
        let identity = crypto::load_secret_key(key_path)?;
        Ok(identity.to_public().to_string())
    }

    /// Find notes that have changed locally since last sync.
    fn changed_local_notes(
        &self,
        notes_folder: &Path,
    ) -> Result<Vec<note::Note>, anyhow::Error> {
        let notes = note::list_notes(notes_folder)?;
        let mut changed = Vec::new();

        for n in &notes {
            let age_file = self.repo_path.join(format!("{}.md.age", n.id));
            if !age_file.exists() {
                changed.push(n.clone());
            } else {
                let local_meta = std::fs::metadata(&n.path)?;
                let age_meta = std::fs::metadata(&age_file)?;
                if local_meta.modified()? > age_meta.modified()? {
                    changed.push(n.clone());
                }
            }
        }

        Ok(changed)
    }

    /// Collect all note IDs currently in the sync repo.
    fn remote_note_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();
        if let Ok(entries) = std::fs::read_dir(&self.repo_path) {
            for entry in entries.flatten() {
                let filename = entry.file_name();
                let filename = filename.to_str().unwrap_or("");
                if filename.ends_with(".md.age") && !filename.starts_with('.') {
                    ids.insert(filename.trim_end_matches(".md.age").to_string());
                }
            }
        }
        ids
    }

    /// Collect all local note IDs (including soft-deleted).
    fn all_local_note_ids(&self, notes_folder: &Path) -> HashSet<String> {
        let mut ids = HashSet::new();
        if let Ok(notes) = note::list_notes(notes_folder) {
            for n in &notes {
                ids.insert(n.id.to_string());
            }
        }
        if let Ok(deleted) = note::list_deleted_notes(notes_folder) {
            for n in &deleted {
                ids.insert(n.id.to_string());
            }
        }
        ids
    }
}

impl SyncProvider for GitSyncProvider {
    fn is_configured(&self) -> bool {
        !self.remote_url.is_empty()
    }

    fn status(&self) -> SyncStatus {
        if !self.is_configured() {
            return SyncStatus::NotConfigured;
        }
        SyncStatus::Idle
    }

    fn push(&self, notes_folder: &Path, key_path: &Path) -> Result<usize, anyhow::Error> {
        self.ensure_repo()?;

        let public_key = Self::public_key(key_path)?;
        let changed = self.changed_local_notes(notes_folder)?;

        // Detect orphaned .md.age files in the sync repo that no longer
        // have a matching local note — these were permanently deleted.
        let local_ids = self.all_local_note_ids(notes_folder);
        let mut removed_count = 0;
        if self.repo_path.join(".git").exists() {
            let remote_ids = self.remote_note_ids();
            for id in &remote_ids {
                if !local_ids.contains(id) {
                    let age_path = self.repo_path.join(format!("{}.md.age", id));
                    if age_path.exists() {
                        std::fs::remove_file(&age_path)?;
                        removed_count += 1;
                    }
                }
            }
        }

        if changed.is_empty() && removed_count == 0 {
            return Ok(0);
        }

        // Encrypt each changed note into the repo
        for n in &changed {
            let age_path = self.repo_path.join(format!("{}.md.age", n.id));
            let plaintext = n.serialize().into_bytes();
            let encrypted = crypto::encrypt(&public_key, &plaintext)
                .map_err(|e| anyhow::anyhow!("Failed to encrypt note {}: {}", n.id, e))?;
            std::fs::write(&age_path, encrypted)?;
        }

        // Stage, commit, push
        self.git(&["add", "*.md.age"])?;

        let status = self.git(&["status", "--porcelain"])?;
        if String::from_utf8_lossy(&status.stdout).trim().is_empty() {
            return Ok(0);
        }

        self.git(&["commit", "-m", "Update notes"])?;
        self.git(&["push", "origin", &self.branch])?;

        Ok(changed.len() + removed_count)
    }

    fn pull(
        &self,
        notes_folder: &Path,
        key_path: &Path,
    ) -> Result<(usize, Vec<PathBuf>), anyhow::Error> {
        self.ensure_repo()?;

        // Fetch and pull
        self.git(&["fetch"])?;
        self.git(&["pull", "origin", &self.branch])?;

        let secret_key = crypto::load_secret_key(key_path)?;

        // Collect remote note IDs after pull
        let remote_ids = self.remote_note_ids();

        // Soft-delete local notes that no longer exist on the remote.
        // These were permanently deleted on another device.
        let all_local_ids = self.all_local_note_ids(notes_folder);
        let mut soft_deleted = 0;
        for id in &all_local_ids {
            if !remote_ids.contains(id) {
                // Find the note on disk and soft-delete it
                if let Ok(notes) = note::list_deleted_notes(notes_folder) {
                    if let Some(n) = notes.iter().find(|n| n.id.to_string() == *id) {
                        if !n.deleted {
                            let mut note = n.clone();
                            let _ = note::soft_delete_note(&mut note);
                            soft_deleted += 1;
                        }
                    }
                }
                if let Ok(notes) = note::list_notes(notes_folder) {
                    if let Some(n) = notes.iter().find(|n| n.id.to_string() == *id) {
                        let mut note = n.clone();
                        let _ = note::soft_delete_note(&mut note);
                        soft_deleted += 1;
                    }
                }
            }
        }

        // Decrypt all .md.age files from the repo
        let mut pulled = 0;
        let mut conflicts = Vec::new();

        let entries = std::fs::read_dir(&self.repo_path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

            if !filename.ends_with(".md.age") || filename.starts_with('.') {
                continue;
            }

            let note_id = filename.trim_end_matches(".md.age");

            let local_exists = note::list_notes(notes_folder)?
                .iter()
                .any(|n| n.id.to_string() == note_id);

            let ciphertext = std::fs::read(&path)?;
            let plaintext = crypto::decrypt(&secret_key, &ciphertext)
                .map_err(|e| anyhow::anyhow!("Failed to decrypt note {}: {}", note_id, e))?;

            let parsed = note::deserialize_content(&String::from_utf8_lossy(&plaintext));

            let dest_path = match &parsed {
                Ok(n) => notes_folder.join(format!("{}.md", note::slugify(&n.title))),
                Err(_) => notes_folder.join(format!("{}.md", note_id)),
            };

            if local_exists {
                if let Some(stem) = dest_path.file_stem() {
                    let stem = stem.to_string_lossy();
                    let parent = dest_path.parent().unwrap();
                    let now = chrono::Utc::now().format("%Y-%m-%d");
                    let conflict_dest = parent.join(format!("{}.conflict-{}.md", stem, now));

                    if conflict_dest.exists() {
                        let mut counter = 1;
                        loop {
                            let alt =
                                parent.join(format!("{}.conflict-{}-{}.md", stem, now, counter));
                            if !alt.exists() {
                                std::fs::write(&alt, &plaintext)?;
                                conflicts.push(alt);
                                break;
                            }
                            counter += 1;
                        }
                    } else {
                        std::fs::write(&conflict_dest, &plaintext)?;
                        conflicts.push(conflict_dest);
                    }
                }
            } else {
                std::fs::write(&dest_path, &plaintext)?;
            }

            pulled += 1;
        }

        Ok((pulled + soft_deleted, conflicts))
    }
}
