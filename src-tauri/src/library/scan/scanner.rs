use std::path::PathBuf;
use walkdir::WalkDir;
use rayon::prelude::*;
use crate::library::scan::{extract_tags, TrackMeta};
use crate::library::db::Db;

#[derive(Debug)]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
    pub current_path: Option<String>,
}

pub struct Scanner {
    db: Db,
}

impl Scanner {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub fn scan_folder(&self, root: PathBuf) -> Vec<ScanProgress> {
        let exts = ["mp3","flac","ogg","opus","m4a","aac","wav","wma","ape","wv"];
        let files: Vec<PathBuf> = WalkDir::new(&root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                exts.contains(&e.path().extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str())
            })
            .map(|e| e.into_path())
            .collect();

        let total = files.len();
        let mut progress = Vec::new();
        let mut scanned = 0;

        files.par_iter().for_each(|path| {
            if let Ok(meta) = extract_tags(path) {
                let _ = self.db.insert_or_update_song(&meta);
            }
            // Note: progress events not thread-safe here; simplified
        });

        scanned = total;
        progress.push(ScanProgress { scanned, total, current_path: None });
        progress
    }
}
