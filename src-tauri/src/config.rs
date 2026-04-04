use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: i32,
    pub y: i32,
    pub is_maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 960.0,
            height: 640.0,
            x: 0,
            y: 0,
            is_maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub remote_url: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub notes_folder: PathBuf,
    #[serde(default)]
    pub window: WindowState,
    #[serde(default)]
    pub sync: Option<SyncConfig>,
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
                window: WindowState::default(),
                sync: None,
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

    pub fn set_sync_config(&mut self, remote_url: String, branch: String) -> Result<(), anyhow::Error> {
        self.sync = Some(SyncConfig { remote_url, branch });
        self.save()
    }
}
