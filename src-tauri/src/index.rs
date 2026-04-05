use crate::note;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created TEXT NOT NULL,
    modified TEXT NOT NULL,
    path TEXT NOT NULL
);
";

pub struct SearchIndex {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub modified: String,
}

impl SearchIndex {
    pub fn new(notes_folder: &Path) -> Result<Self, anyhow::Error> {
        let nvage_dir = notes_folder.join(".nvage");
        std::fs::create_dir_all(&nvage_dir)?;
        let db_path = nvage_dir.join("search.db");

        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;

        let mut index = Self { conn };
        index.rebuild(notes_folder)?;
        Ok(index)
    }

    pub fn rebuild(&mut self, notes_folder: &Path) -> Result<(), anyhow::Error> {
        self.conn.execute("DELETE FROM notes", [])?;

        let notes = note::list_notes(notes_folder)?;
        for n in &notes {
            self.insert(notes_folder, n)?;
        }

        Ok(())
    }

    /// Incrementally update the index for specific changed files.
    /// Much cheaper than a full rebuild — only touches the affected notes.
    pub fn update_files(
        &mut self,
        notes_folder: &Path,
        changed: &[PathBuf],
    ) -> Result<(), anyhow::Error> {
        for path in changed {
            // Check if the file still exists
            if path.exists() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Ok(n) = note::deserialize_note(path) {
                    self.insert(notes_folder, &n)?;
                }
            } else {
                // File was deleted — remove from index by path
                let relative = path
                    .strip_prefix(notes_folder)
                    .unwrap_or(path)
                    .to_string_lossy();
                self.conn
                    .execute("DELETE FROM notes WHERE path = ?1", params![relative])?;
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, notes_folder: &Path, note: &note::Note) -> Result<(), anyhow::Error> {
        let relative_path = note
            .path
            .strip_prefix(notes_folder)
            .unwrap_or(&note.path)
            .to_string_lossy()
            .to_string();

        self.conn.execute(
            "INSERT OR REPLACE INTO notes (id, title, content, created, modified, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                note.id.to_string(),
                note.title,
                note.content,
                note.created.to_rfc3339(),
                note.modified.to_rfc3339(),
                relative_path,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), anyhow::Error> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, anyhow::Error> {
        // For empty or short queries, return all notes ordered by recency
        if query.trim().is_empty() || query.trim().len() < 3 {
            let mut stmt = self
                .conn
                .prepare("SELECT id, title, content, modified FROM notes ORDER BY modified DESC")?;
            let rows = stmt.query_map([], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    preview: make_preview(&row.get::<_, String>(2)?),
                    modified: row.get(3)?,
                })
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            return Ok(results);
        }

        // Substring matching with LIKE, ordered by recency
        let like_query = format!("%{}%", query.replace("'", "''"));
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, modified FROM notes \
             WHERE title LIKE ?1 OR content LIKE ?1 \
             ORDER BY modified DESC \
             LIMIT 100",
        )?;

        let rows = stmt.query_map(params![like_query], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                title: row.get(1)?,
                preview: make_preview(&row.get::<_, String>(2)?),
                modified: row.get(3)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_note(
        &self,
        id: &str,
        notes_folder: &Path,
    ) -> Result<Option<note::Note>, anyhow::Error> {
        let note_path = notes_folder.join(format!("{}.md", id));
        if note_path.exists() {
            return Ok(Some(note::deserialize_note(&note_path)?));
        }

        let mut stmt = self.conn.prepare("SELECT path FROM notes WHERE id = ?1")?;
        let path_str: Option<String> = stmt.query_row(params![id], |row| row.get(0)).optional()?;

        if let Some(path_str) = path_str {
            let full_path = notes_folder.join(&path_str);
            if full_path.exists() {
                return Ok(Some(note::deserialize_note(&full_path)?));
            }
        }

        for entry in std::fs::read_dir(notes_folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Ok(n) = note::deserialize_note(&path) {
                    if n.id.to_string() == id {
                        return Ok(Some(n));
                    }
                }
            }
        }

        Ok(None)
    }
}

fn make_preview(content: &str) -> String {
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.starts_with("---") && !l.trim().is_empty())
        .collect();

    // If there's only one line (likely just a title), return empty so frontend shows "Empty note"
    if lines.len() <= 1 {
        return String::new();
    }

    // Return the second line (first line after title) as preview
    lines[1].chars().take(100).collect()
}
