mod config;
mod crypto;
mod index;
mod note;
mod sync_git;
mod sync_provider;
mod watcher;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use sync_provider::SyncProvider;

struct AppState {
    config: Mutex<config::AppConfig>,
    search_index: Mutex<index::SearchIndex>,
    sync_provider: Mutex<Option<sync_git::GitSyncProvider>>,
    sync_key_path: Mutex<Option<PathBuf>>,
}

#[derive(serde::Serialize)]
struct SearchResult {
    id: String,
    title: String,
    preview: String,
    modified: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NoteDto {
    id: String,
    title: String,
    content: String,
    created: String,
    modified: String,
}

#[derive(serde::Serialize)]
struct SyncStatusDto {
    status: String,
    message: String,
}

#[derive(serde::Serialize)]
struct KeyPairDto {
    public_key: String,
    secret_key: String,
}

// ── Note commands ──

#[tauri::command]
fn search_notes(query: String, state: State<Arc<AppState>>) -> Result<Vec<SearchResult>, String> {
    let index = state.search_index.lock().map_err(|e| e.to_string())?;
    let results = index.search(&query).map_err(|e| e.to_string())?;
    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            id: r.id,
            title: r.title,
            preview: r.preview,
            modified: r.modified,
        })
        .collect())
}

#[tauri::command]
fn get_note(id: String, state: State<Arc<AppState>>) -> Result<Option<NoteDto>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let folder = config.notes_folder.clone();
    drop(config);

    let index = state.search_index.lock().map_err(|e| e.to_string())?;
    let n = index
        .get_note(&id, &folder)
        .map_err(|e| e.to_string())?;
    Ok(n.map(|n| NoteDto {
        id: n.id.to_string(),
        title: n.title,
        content: n.content,
        created: n.created.to_rfc3339(),
        modified: n.modified.to_rfc3339(),
    }))
}

#[tauri::command]
fn create_note(title: String, content: String, state: State<Arc<AppState>>) -> Result<NoteDto, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let folder = config.notes_folder.clone();
    drop(config);

    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;

    let n = note::create_note(&folder, &title, &content).map_err(|e| e.to_string())?;

    let mut index = state.search_index.lock().map_err(|e| e.to_string())?;
    index.insert(&folder, &n).map_err(|e| e.to_string())?;

    Ok(NoteDto {
        id: n.id.to_string(),
        title: n.title,
        content: n.content,
        created: n.created.to_rfc3339(),
        modified: n.modified.to_rfc3339(),
    })
}

#[tauri::command]
fn update_note(id: String, content: String, state: State<Arc<AppState>>) -> Result<NoteDto, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let folder = config.notes_folder.clone();
    drop(config);

    let mut n = {
        let index = state.search_index.lock().map_err(|e| e.to_string())?;
        index
            .get_note(&id, &folder)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note not found: {}", id))?
    };

    n.content = content;
    note::update_note(&mut n).map_err(|e| e.to_string())?;

    {
        let mut index = state.search_index.lock().map_err(|e| e.to_string())?;
        index.insert(&folder, &n).map_err(|e| e.to_string())?;
    }

    Ok(NoteDto {
        id: n.id.to_string(),
        title: n.title,
        content: n.content,
        created: n.created.to_rfc3339(),
        modified: n.modified.to_rfc3339(),
    })
}

#[tauri::command]
fn delete_note_cmd(id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let folder = config.notes_folder.clone();
    drop(config);

    let n = {
        let index = state.search_index.lock().map_err(|e| e.to_string())?;
        index
            .get_note(&id, &folder)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note not found: {}", id))?
    };

    note::delete_note(&n).map_err(|e| e.to_string())?;

    {
        let mut index = state.search_index.lock().map_err(|e| e.to_string())?;
        index.delete(&id).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn set_notes_folder(folder: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let path = PathBuf::from(&folder);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;

    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.set_notes_folder(path.clone()).map_err(|e| e.to_string())?;
    }

    {
        let mut index = state.search_index.lock().map_err(|e| e.to_string())?;
        *index = index::SearchIndex::new(&path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn get_notes_folder(state: State<Arc<AppState>>) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.notes_folder.to_string_lossy().to_string())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Sync commands ──

#[tauri::command]
fn generate_sync_key(state: State<Arc<AppState>>) -> Result<KeyPairDto, String> {
    let (public_key, secret_key) = crypto::generate_key().map_err(|e| e.to_string())?;

    let key_dir = dirs::config_dir()
        .ok_or_else(|| "Cannot determine config directory".to_string())?
        .join("nvage");
    let key_path = key_dir.join("key.txt");

    crypto::save_secret_key(&key_path, &secret_key).map_err(|e| e.to_string())?;

    {
        let mut key_guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;
        *key_guard = Some(key_path);
    }

    Ok(KeyPairDto { public_key, secret_key })
}

#[tauri::command]
fn import_sync_key(key_str: String, state: State<Arc<AppState>>) -> Result<KeyPairDto, String> {
    let identity = crypto::parse_secret_key(&key_str).map_err(|e| e.to_string())?;
    let public_key = identity.to_public().to_string();

    let key_dir = dirs::config_dir()
        .ok_or_else(|| "Cannot determine config directory".to_string())?
        .join("nvage");
    let key_path = key_dir.join("key.txt");

    crypto::save_secret_key(&key_path, &key_str).map_err(|e| e.to_string())?;

    {
        let mut key_guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;
        *key_guard = Some(key_path);
    }

    Ok(KeyPairDto { public_key, secret_key: key_str })
}

#[tauri::command]
fn configure_sync(remote_url: String, branch: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let key_path = {
        let guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;
        guard.clone().ok_or_else(|| "No sync key configured. Generate or import a key first.".to_string())?
    };

    if !key_path.exists() {
        return Err("Sync key file not found. Generate or import a key first.".to_string());
    }

    let repo_path = dirs::cache_dir()
        .ok_or_else(|| "Cannot determine cache directory".to_string())?
        .join("nvage")
        .join("sync-repo");

    let provider = sync_git::GitSyncProvider::new(remote_url, branch, repo_path);

    {
        let mut provider_guard = state.sync_provider.lock().map_err(|e| e.to_string())?;
        *provider_guard = Some(provider);
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct ValidationDto {
    git_installed: bool,
    key_exists: bool,
    remote_reachable: bool,
    errors: Vec<String>,
}

#[tauri::command]
fn validate_sync_setup(remote_url: String, state: State<Arc<AppState>>) -> Result<ValidationDto, String> {
    let mut errors = Vec::new();

    let git_installed = sync_git::find_git().is_ok();
    if !git_installed {
        errors.push("Git is not installed. Install Git to use sync.".to_string());
    }

    let key_path = {
        let guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };
    let key_exists = key_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    if !key_exists {
        errors.push("No encryption key found. Generate or import a key first.".to_string());
    }

    let mut remote_reachable = false;
    if git_installed && !remote_url.is_empty() {
        let git_path = sync_git::find_git().unwrap_or_else(|_| "git".to_string());
        let output = std::process::Command::new(git_path)
            .args(["ls-remote", "--exit-code", "--heads", &remote_url])
            .output();
        remote_reachable = output.map(|o| o.status.success()).unwrap_or(false);
        if !remote_reachable {
            errors.push("Cannot reach remote repo. Check the URL and your access.".to_string());
        }
    }

    Ok(ValidationDto {
        git_installed,
        key_exists,
        remote_reachable,
        errors,
    })
}

#[tauri::command]
fn sync_notes(direction: String, state: State<Arc<AppState>>) -> Result<SyncStatusDto, String> {
    let key_path = {
        let guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;
        guard.clone().ok_or_else(|| "No sync key configured.".to_string())?
    };

    let folder = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.notes_folder.clone()
    };

    let mut provider_guard = state.sync_provider.lock().map_err(|e| e.to_string())?;
    let provider = provider_guard.as_mut().ok_or_else(|| "Sync not configured.".to_string())?;

    let result: Result<SyncStatusDto, String> = match direction.as_str() {
        "push" => {
            let count = provider.push(&folder, &key_path).map_err(|e| e.to_string())?;
            Ok(SyncStatusDto {
                status: "idle".to_string(),
                message: format!("Pushed {} notes", count),
            })
        }
        "pull" => {
            let (count, conflicts) = provider.pull(&folder, &key_path).map_err(|e| e.to_string())?;
            if !conflicts.is_empty() {
                Ok(SyncStatusDto {
                    status: "conflict".to_string(),
                    message: format!("Pulled {} notes, {} conflicts detected", count, conflicts.len()),
                })
            } else {
                Ok(SyncStatusDto {
                    status: "idle".to_string(),
                    message: format!("Pulled {} notes", count),
                })
            }
        }
        "full" => {
            let pushed = provider.push(&folder, &key_path).map_err(|e| e.to_string())?;
            let (pulled, conflicts) = provider.pull(&folder, &key_path).map_err(|e| e.to_string())?;
            let status = if conflicts.is_empty() { "idle" } else { "conflict" };
            Ok(SyncStatusDto {
                status: status.to_string(),
                message: format!("Pushed {} notes, pulled {} notes", pushed, pulled),
            })
        }
        _ => Err("Invalid sync direction. Use 'push', 'pull', or 'full'.".to_string()),
    };

    let result = result?;

    if direction == "pull" || direction == "full" {
        let mut index = state.search_index.lock().map_err(|e| e.to_string())?;
        let _ = index.rebuild(&folder);
    }

    Ok(result)
}

#[tauri::command]
fn get_sync_status(state: State<Arc<AppState>>) -> Result<SyncStatusDto, String> {
    let provider_guard = state.sync_provider.lock().map_err(|e| e.to_string())?;
    let key_guard = state.sync_key_path.lock().map_err(|e| e.to_string())?;

    if key_guard.is_none() {
        return Ok(SyncStatusDto {
            status: "not_configured".to_string(),
            message: "No sync key configured".to_string(),
        });
    }

    match provider_guard.as_ref() {
        Some(provider) => {
            let status = provider.status();
            match status {
                sync_provider::SyncStatus::NotConfigured => Ok(SyncStatusDto {
                    status: "not_configured".to_string(),
                    message: "Sync provider not configured".to_string(),
                }),
                sync_provider::SyncStatus::Idle => Ok(SyncStatusDto {
                    status: "idle".to_string(),
                    message: "Ready to sync".to_string(),
                }),
                sync_provider::SyncStatus::Syncing => Ok(SyncStatusDto {
                    status: "syncing".to_string(),
                    message: "Sync in progress".to_string(),
                }),
                sync_provider::SyncStatus::Error(msg) => Ok(SyncStatusDto {
                    status: "error".to_string(),
                    message: msg,
                }),
                sync_provider::SyncStatus::Conflict(paths) => Ok(SyncStatusDto {
                    status: "conflict".to_string(),
                    message: format!("{} conflicts detected", paths.len()),
                }),
            }
        }
        None => Ok(SyncStatusDto {
            status: "not_configured".to_string(),
            message: "Sync not configured".to_string(),
        }),
    }
}

// ── Helpers ──

fn update_files(state: Arc<AppState>, changed: &[std::path::PathBuf]) {
    let folder = {
        let config = match state.config.lock() {
            Ok(c) => c.notes_folder.clone(),
            Err(_) => return,
        };
        config
    };

    let mut index = match state.search_index.lock() {
        Ok(i) => i,
        Err(_) => return,
    };
    let _ = index.update_files(&folder, changed);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config::AppConfig::load().expect("Failed to load config");
    let notes_folder = config.notes_folder.clone();
    let window_state = config.window.clone();

    std::fs::create_dir_all(&notes_folder).expect("Failed to create notes folder");

    let search_index =
        index::SearchIndex::new(&notes_folder).expect("Failed to create search index");

    let key_path = dirs::config_dir()
        .map(|d| d.join("nvage").join("key.txt"))
        .filter(|p| p.exists());

    let app_state = Arc::new(AppState {
        config: Mutex::new(config),
        search_index: Mutex::new(search_index),
        sync_provider: Mutex::new(None),
        sync_key_path: Mutex::new(key_path),
    });

    let watcher_state = Arc::clone(&app_state);
    let _watcher = watcher::FsWatcher::new(&notes_folder, move |_folder, changed| {
        std::thread::sleep(std::time::Duration::from_millis(300));
        update_files(Arc::clone(&watcher_state), changed);
    })
    .expect("Failed to create filesystem watcher");

    std::mem::forget(_watcher);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            let window = app.get_webview_window("main").unwrap();
            if window_state.is_maximized {
                let _ = window.maximize();
            } else {
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: window_state.width as u32,
                    height: window_state.height as u32,
                }));
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: window_state.x,
                    y: window_state.y,
                }));
            }

            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api: _, .. } = event {
                    if let Some(win) = app_handle.get_webview_window("main") {
                        if let (Ok(size), Ok(position), Ok(is_maximized)) =
                            (win.inner_size(), win.outer_position(), win.is_maximized())
                        {
                            if let Ok(mut cfg) = config::AppConfig::load() {
                                cfg.window = config::WindowState {
                                    width: size.width as f64,
                                    height: size.height as f64,
                                    x: position.x,
                                    y: position.y,
                                    is_maximized,
                                };
                                let _ = cfg.save();
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_notes,
            get_note,
            create_note,
            update_note,
            delete_note_cmd,
            set_notes_folder,
            get_notes_folder,
            get_app_version,
            generate_sync_key,
            import_sync_key,
            configure_sync,
            sync_notes,
            get_sync_status,
            validate_sync_setup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
