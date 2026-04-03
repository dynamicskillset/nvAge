use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
}

impl FsWatcher {
    pub fn new<F>(notes_folder: &Path, on_change: F) -> Result<Self, anyhow::Error>
    where
        F: Fn(&Path, &[PathBuf]) + Send + 'static,
    {
        let folder = notes_folder.to_path_buf();
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;
        watcher.watch(notes_folder, RecursiveMode::NonRecursive)?;

        std::thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                match result {
                    Ok(event) => {
                        if let Some(paths) = changed_md_files(&event) {
                            on_change(&folder, &paths);
                        }
                    }
                    Err(e) => log::error!("Watch error: {:?}", e),
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}

fn changed_md_files(event: &Event) -> Option<Vec<PathBuf>> {
    match &event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
            let paths: Vec<PathBuf> = event
                .paths
                .iter()
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e == "md")
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            if paths.is_empty() {
                None
            } else {
                Some(paths)
            }
        }
        _ => None,
    }
}
