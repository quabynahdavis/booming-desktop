use std::collections::HashMap;
use std::sync::Mutex;

use crate::library::db::Db;

/// In-memory cache for small cover art images.
///
/// Decodes each blob from the DB into a base64 data URL (for the frontend)
/// and keeps the decoded bytes cached under the song id so scrolling the
/// song list does not re-decode the same cover over and over.
pub struct CoverCache {
    cache: Mutex<HashMap<i64, Vec<u8>>>,
}

impl CoverCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Fetch the cover for a song, decoding it to a data URL and caching the
    /// decoded bytes. Small covers only (blobs over ~512KB are not cached).
    pub fn get_data_url(&self, db: &Db, song_id: i64, max_bytes: usize) -> Option<String> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(bytes) = cache.get(&song_id) {
                if bytes.len() <= max_bytes {
                    return Some(to_data_url(bytes));
                }
            }
        }

        let cover = db.get_cover(song_id).unwrap_or(None)?;
        if cover.len() > max_bytes {
            // Too large to be worth caching as a thumbnail data URL.
            return None;
        }
        let url = to_data_url(&cover);
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(song_id, cover);
        }
        Some(url)
    }
}

fn to_data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let b64 = engine.encode(bytes);
    format!("data:image/png;base64,{b64}")
}
