use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Settings {
    pub library_folders: Vec<String>,
    pub theme: String,
    pub replay_gain_mode: String,
    pub audio_filter: String,
}

impl Settings {
    pub fn load(path: &PathBuf) -> Self {
        if path.exists() {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(s) = serde_json::from_str::<Settings>(&data) {
                    return s;
                }
            }
        }
        Settings {
            library_folders: vec![],
            theme: "system".to_string(),
            replay_gain_mode: "off".to_string(),
            audio_filter: "".to_string(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, data)
    }
}
