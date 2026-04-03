mod config;
mod index;
mod note;
mod watcher;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

struct AppState {
    config: Mutex<config::AppConfig>,
    search_index: Mutex<index::SearchIndex>,
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

    std::fs::create_dir_all(&notes_folder).expect("Failed to create notes folder");

    let search_index =
        index::SearchIndex::new(&notes_folder).expect("Failed to create search index");

    let app_state = Arc::new(AppState {
        config: Mutex::new(config),
        search_index: Mutex::new(search_index),
    });

    // Set up filesystem watcher
    let watcher_state = Arc::clone(&app_state);
    let _watcher = watcher::FsWatcher::new(&notes_folder, move |_folder, changed| {
        std::thread::sleep(std::time::Duration::from_millis(300));
        update_files(Arc::clone(&watcher_state), changed);
    })
    .expect("Failed to create filesystem watcher");

    // Keep watcher alive for the lifetime of the app
    std::mem::forget(_watcher);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            search_notes,
            get_note,
            create_note,
            update_note,
            delete_note_cmd,
            set_notes_folder,
            get_notes_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
