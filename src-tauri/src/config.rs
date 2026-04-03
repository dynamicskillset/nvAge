use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub notes_folder: PathBuf,
}

impl AppConfig {
    pub fn default_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nvage");
        config_dir.join("config.json")
    }

    pub fn default_notes_folder() -> PathBuf {
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nvage-notes")
    }

    pub fn load() -> Result<Self, anyhow::Error> {
        let path = Self::default_path();
        if !path.exists() {
            let config = Self {
                notes_folder: Self::default_notes_folder(),
            };
            config.save()?;
            return Ok(config);
        }
        let content = fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn set_notes_folder(&mut self, folder: PathBuf) -> Result<(), anyhow::Error> {
        self.notes_folder = folder;
        self.save()
    }
}
