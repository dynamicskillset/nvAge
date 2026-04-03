use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub path: PathBuf,
}

impl Note {
    /// Serialize the note to its full file content (frontmatter + body).
    pub fn serialize(&self) -> String {
        serialize_note(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: Uuid,
    pub title: String,
    pub preview: String,
    pub modified: DateTime<Utc>,
}

pub fn parse_frontmatter(content: &str) -> (Option<Uuid>, Option<DateTime<Utc>>, &str) {
    if !content.starts_with("---\n") {
        return (None, None, content);
    }

    let rest = &content[4..];
    if let Some(end) = rest.find("\n---\n") {
        let frontmatter = &rest[..end];
        let body = &rest[end + 5..];

        let mut id = None;
        let mut created = None;

        for line in frontmatter.lines() {
            if let Some((key, value)) = line.split_once(": ") {
                match key.trim() {
                    "id" => {
                        if let Ok(uuid) = Uuid::parse_str(value.trim()) {
                            id = Some(uuid);
                        }
                    }
                    "created" => {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(value.trim()) {
                            created = Some(dt.with_timezone(&Utc));
                        }
                    }
                    _ => {}
                }
            }
        }

        (id, created, body)
    } else {
        (None, None, content)
    }
}

pub fn build_frontmatter(id: Uuid, created: DateTime<Utc>) -> String {
    format!(
        "---\nid: {}\ncreated: {}\n---\n",
        id,
        created.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    )
}

pub fn serialize_note(note: &Note) -> String {
    let frontmatter = build_frontmatter(note.id, note.created);
    format!("{}{}", frontmatter, note.content)
}

pub fn deserialize_note(path: &Path) -> Result<Note, anyhow::Error> {
    let content = fs::read_to_string(path)?;
    let (id, created, body) = parse_frontmatter(&content);

    let id = id.unwrap_or_else(Uuid::new_v4);
    let created = created.unwrap_or_else(Utc::now);

    let title = extract_title(body);
    let modified = path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.into())
        .unwrap_or_else(Utc::now);

    Ok(Note {
        id,
        title,
        content: body.to_string(),
        created,
        modified,
        path: path.to_path_buf(),
    })
}

/// Deserialize a note from raw content string (used when decrypting from sync).
/// Returns the parsed Note without a valid path (caller must set it).
pub fn deserialize_content(content: &str) -> Result<Note, anyhow::Error> {
    let (id, created, body) = parse_frontmatter(content);

    let id = id.unwrap_or_else(Uuid::new_v4);
    let created = created.unwrap_or_else(Utc::now);
    let title = extract_title(body);
    let now = Utc::now();

    Ok(Note {
        id,
        title,
        content: body.to_string(),
        created,
        modified: now,
        path: PathBuf::new(), // caller must set this
    })
}

pub fn extract_title(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();

    if first_line.starts_with("# ") {
        first_line[2..].trim().to_string()
    } else if !first_line.is_empty() {
        first_line.to_string()
    } else {
        "Untitled".to_string()
    }
}

pub fn slugify(title: &str) -> String {
    let s = slug::slugify(title);
    if s.is_empty() {
        format!("note-{}", Uuid::new_v4())
    } else {
        s
    }
}

pub fn create_note(folder: &Path, title: &str, content: &str) -> Result<Note, anyhow::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let slug = slugify(title);
    let path = folder.join(format!("{}.md", slug));

    let note = Note {
        id,
        title: title.to_string(),
        content: content.to_string(),
        created: now,
        modified: now,
        path: path.clone(),
    };

    let serialized = serialize_note(&note);
    fs::write(&path, serialized)?;

    Ok(note)
}

pub fn update_note(note: &mut Note) -> Result<(), anyhow::Error> {
    note.modified = Utc::now();
    note.title = extract_title(&note.content);

    let new_slug = slugify(&note.title);
    let new_path = note
        .path
        .parent()
        .unwrap()
        .join(format!("{}.md", new_slug));

    let serialized = serialize_note(note);
    fs::write(&new_path, &serialized)?;

    if new_path != note.path {
        let _ = fs::remove_file(&note.path);
        note.path = new_path;
    }

    Ok(())
}

pub fn delete_note(note: &Note) -> Result<(), anyhow::Error> {
    if note.path.exists() {
        fs::remove_file(&note.path)?;
    }
    Ok(())
}

pub fn list_notes(folder: &Path) -> Result<Vec<Note>, anyhow::Error> {
    let mut notes = Vec::new();

    if !folder.exists() {
        return Ok(notes);
    }

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(note) = deserialize_note(&path) {
                notes.push(note);
            }
        }
    }

    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(notes)
}

pub fn to_summary(note: &Note) -> NoteSummary {
    let preview = note
        .content
        .lines()
        .filter(|l| !l.starts_with("---") && !l.trim().is_empty())
        .next()
        .unwrap_or("")
        .chars()
        .take(100)
        .collect();

    NoteSummary {
        id: note.id,
        title: note.title.clone(),
        preview,
        modified: note.modified,
    }
}
