use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Settings {
    pub library_folders: Vec<String>,
    pub theme: String,
    pub replay_gain_mode: String,
    pub audio_filter: String,
    pub repeat_mode: String,
    pub shuffle_enabled: bool,
    pub scrobble_enabled: bool,
    pub scrobble_source: String,
    pub listenbrainz_token: Option<String>,
    pub lastfm_token: Option<String>,
    pub locale: String,
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
            repeat_mode: "None".to_string(),
            shuffle_enabled: false,
            scrobble_enabled: false,
            scrobble_source: "off".to_string(),
            listenbrainz_token: None,
            lastfm_token: None,
            locale: "en".to_string(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, data)
    }

    pub fn scrobble_enabled(&self) -> bool {
        self.scrobble_enabled
    }

    pub fn set_scrobble_enabled(&mut self, enabled: bool) {
        self.scrobble_enabled = enabled;
    }

    pub fn scrobble_source(&self) -> &str {
        &self.scrobble_source
    }

    pub fn set_scrobble_source(&mut self, source: String) {
        self.scrobble_source = source;
    }

    pub fn listenbrainz_token(&self) -> Option<&str> {
        self.listenbrainz_token.as_deref()
    }

    pub fn set_listenbrainz_token(&mut self, token: Option<String>) {
        self.listenbrainz_token = token;
    }

    pub fn lastfm_token(&self) -> Option<&str> {
        self.lastfm_token.as_deref()
    }

    pub fn set_lastfm_token(&mut self, token: Option<String>) {
        self.lastfm_token = token;
    }
}
