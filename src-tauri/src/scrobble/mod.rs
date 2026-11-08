use crate::library::db::Db;
use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScrobbleError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobbleEvent {
    pub id: Option<i64>,
    pub source: String,
    pub artist: String,
    pub title: String,
    pub track_ts_ms: i64,
    pub duration_ms: i64,
}

impl ScrobbleEvent {
    pub fn new(source: String, artist: String, title: String, track_ts_ms: i64, duration_ms: i64) -> Self {
        Self {
            id: None,
            source,
            artist,
            title,
            track_ts_ms,
            duration_ms,
        }
    }

    fn hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.source.hash(&mut hasher);
        self.artist.hash(&mut hasher);
        self.title.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

pub struct ScrobbleService {
    db: Arc<Db>,
    client: reqwest::Client,
}

impl ScrobbleService {
    pub fn new(db: Arc<Db>) -> Self {
        Self {
            db,
            client: reqwest::Client::new(),
        }
    }

    pub async fn submit(&self, event: ScrobbleEvent) -> Result<(), ScrobbleError> {
        let hash = event.hash();
        self.db.add_scrobble_event(&event, &hash)?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), ScrobbleError> {
        let events = self.db.load_scrobble_queue()?;
        if events.is_empty() {
            return Ok(());
        }

        let mut processed_ids = Vec::new();

        for event in events {
            let result = match event.source.as_str() {
                "listenbrainz" => self.submit_listenbrainz(&event).await,
                "lastfm" => self.submit_lastfm(&event).await,
                _ => continue,
            };

            if result.is_ok() {
                processed_ids.push(event.id.unwrap_or(0));
            }
        }

        if !processed_ids.is_empty() {
            self.db.clear_scrobble_queue(&processed_ids)?;
        }

        Ok(())
    }

    async fn submit_listenbrainz(&self, event: &ScrobbleEvent) -> Result<(), ScrobbleError> {
        let recorded_at = chrono::DateTime::from_timestamp_millis(event.track_ts_ms)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        let payload = serde_json::json!({
            "artist": event.artist,
            "track": event.title,
            "duration": event.duration_ms / 1000,
            "recorded_at": recorded_at,
        });

        let url = "https://api.listenbrainz.org/2/submit/scrobble";
        self.client
            .post(url)
            .header("User-Agent", "BoomingDesktop/0.1.0")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn submit_lastfm(&self, event: &ScrobbleEvent) -> Result<(), ScrobbleError> {
        let api_key = std::env::var("LASTFM_API_KEY").ok();
        let token = std::env::var("LASTFM_TOKEN").ok();

        let Some(api_key) = api_key else {
            return Ok(()); // cannot submit without key
        };
        let Some(token) = token else {
            return Ok(()); // cannot submit without token
        };

        let timestamp = event.track_ts_ms / 1000;

        let params = [
            ("method", "track.scrobble".to_string()),
            ("artist", event.artist.clone()),
            ("track", event.title.clone()),
            ("timestamp", timestamp.to_string()),
            ("duration", (event.duration_ms / 1000).to_string()),
            ("api_key", api_key),
            ("sk", token),
            ("format", "json".to_string()),
        ];

        let url = "https://ws.audioscrobbler.com/2.0/";
        self.client
            .post(url)
            .form(&params)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::Db;
    use std::env;
    use std::sync::Arc;

    #[test]
    fn scrobble_event_hash_dedup() {
        let e1 = ScrobbleEvent::new("listenbrainz".into(), "Artist".into(), "Title".into(), 1000, 2000);
        let e2 = ScrobbleEvent::new("listenbrainz".into(), "Artist".into(), "Title".into(), 1000, 2000);
        let e3 = ScrobbleEvent::new("lastfm".into(), "Artist".into(), "Title".into(), 1000, 2000);
        assert_eq!(e1.hash(), e2.hash());
        assert_ne!(e1.hash(), e3.hash());
    }

    #[test]
    fn scrobble_queue_persistence() {
        let db_path = env::temp_dir().join("booming_test_scrobble.db");
        let db = Arc::new(Db::open(db_path.clone()).unwrap());

        let event = ScrobbleEvent::new("listenbrainz".into(), "Test Artist".into(), "Test Title".into(), 123456789, 180000);
        let hash = event.hash();

        db.add_scrobble_event(&event, &hash).unwrap();

        let events = db.load_scrobble_queue().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].artist, "Test Artist");
        assert_eq!(events[0].title, "Test Title");

        let _ = std::fs::remove_file(&db_path);
    }
}