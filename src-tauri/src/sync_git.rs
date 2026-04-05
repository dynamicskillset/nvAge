use crate::crypto;
use crate::note;
use crate::sync_provider::{SyncProvider, SyncStatus};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the git binary. Tauri apps don't inherit shell PATH, so we
/// search common locations before falling back to just "git".
pub fn find_git() -> Result<String, anyhow::Error> {
    crate::util::locate_git()
}

/// Git-based sync provider that shells out to the `git` CLI.
///
/// Push cycle:
/// 1. Clone or open the sync repo
/// 2. For each changed note, encrypt it to `<uuid>.md.age` in the repo
/// 3. Remove `.md.age` files for notes that no longer exist locally
/// 4. Stage with `git add -A` (catches deletions), commit, push
///
/// Pull cycle:
/// 1. Fetch and pull
/// 2. Decrypt any `.md.age` files into the notes folder
/// 3. Return conflict paths if local notes have also changed
pub struct GitSyncProvider {
    pub remote_url: String,
    pub branch: String,
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

    fn public_key(key_path: &Path) -> Result<String, anyhow::Error> {
        let identity = crypto::load_secret_key(key_path)?;
        Ok(identity.to_public().to_string())
    }

    fn changed_local_notes(&self, notes_folder: &Path) -> Result<Vec<note::Note>, anyhow::Error> {
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

        // Remove .md.age files for notes that no longer exist locally
        let local_ids: HashSet<_> = note::list_notes(notes_folder)?
            .iter()
            .map(|n| n.id.to_string())
            .collect();
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

        for n in &changed {
            let age_path = self.repo_path.join(format!("{}.md.age", n.id));
            let plaintext = n.serialize().into_bytes();
            let encrypted = crypto::encrypt(&public_key, &plaintext)
                .map_err(|e| anyhow::anyhow!("Failed to encrypt note {}: {}", n.id, e))?;
            std::fs::write(&age_path, encrypted)?;
        }

        // -A stages deletions too
        self.git(&["add", "-A", "--", "*.md.age"])?;

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

        // Record HEAD before pulling so we can detect actual changes
        let head_before = self
            .git(&["rev-parse", "HEAD"])
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        self.git(&["fetch"])?;
        self.git(&["pull", "origin", &self.branch])?;

        // Determine which .md.age files actually changed in this pull
        let changed_files: HashSet<String> = if let Some(before) = &head_before {
            if let Ok(after) = self.git(&["rev-parse", "HEAD"]) {
                let after = String::from_utf8_lossy(&after.stdout).trim().to_string();
                if before != &after {
                    let diff = self.git(&["diff", "--name-only", before, &after]);
                    if let Ok(output) = diff {
                        String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .filter(|f| f.ends_with(".md.age"))
                            .filter_map(|f| {
                                std::path::Path::new(f).file_stem().map(|s| {
                                    s.to_string_lossy().trim_end_matches(".md").to_string()
                                })
                            })
                            .collect()
                    } else {
                        HashSet::new()
                    }
                } else {
                    HashSet::new()
                }
            } else {
                HashSet::new()
            }
        } else {
            // No HEAD yet (fresh clone) — treat all files as new
            self.remote_note_ids()
        };

        let secret_key = crypto::load_secret_key(key_path)?;

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

            // Skip files that didn't actually change in this pull
            if !changed_files.contains(note_id) {
                continue;
            }

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

        Ok((pulled, conflicts))
    }
}
