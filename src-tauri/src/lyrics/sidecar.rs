//! Sidecar `.lrc` files stored next to the audio tracks.
//!
//! `<song_path_without_ext>.lrc`, e.g. `/music/Let It Be.m4a` →
//! `/music/Let It Be.lrc`. These are the same files most desktop players use,
//! which lets users fix lyrics with any text editor.

use std::path::{Path, PathBuf};

/// `(state.Version?) Filesystem target for a song's sidecar lyrics.`
pub fn sidecar_path_for(song_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(song_path);
    if song_path.trim().is_empty() || path.file_name().is_none() {
        return Err("invalid song path".to_string());
    }
    // with_extension replaces the final extension only
    Ok(path.with_extension("lrc"))
}

/// Write `lrc_text` to `<song_path_without_ext>.lrc`. Returns the written path.
pub fn save(song_path: &str, lrc_text: &str) -> Result<String, String> {
    let sidecar = sidecar_path_for(song_path)?;
    std::fs::write(&sidecar, lrc_text)
        .map_err(|e| format!("failed to write {}: {e}", sidecar.display()))?;
    Ok(sidecar.to_string_lossy().into_owned())
}

/// Read the sidecar lyrics for a song; `None` when there is no sidecar file.
pub fn load(song_path: &str) -> Result<Option<String>, String> {
    let sidecar = sidecar_path_for(song_path)?;
    match std::fs::read_to_string(&sidecar) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("failed to read {}: {e}", sidecar.display())),
    }
}

/// Delete the sidecar; `true` when it existed, `false` if there was nothing to do.
pub fn delete(song_path: &str) -> Result<bool, String> {
    let sidecar = sidecar_path_for(song_path)?;
    match std::fs::remove_file(&sidecar) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("failed to delete {}: {e}", sidecar.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("booming_sidecar_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn sidecar_path_replaces_last_extension() {
        assert_eq!(
            sidecar_path_for("/music/Let It Be.flac").unwrap(),
            PathBuf::from("/music/Let It Be.lrc")
        );
        // multi-dot files keep earlier extensions
        assert_eq!(
            sidecar_path_for("/music/a.b.m4a").unwrap(),
            PathBuf::from("/music/a.b.lrc")
        );
        // files with no extension get one appended
        assert_eq!(
            sidecar_path_for("/music/song").unwrap(),
            PathBuf::from("/music/song.lrc")
        );
        // invalid inputs are rejected instead of writing to weird places
        assert!(sidecar_path_for("").is_err());
        assert!(sidecar_path_for("   ").is_err());
    }

    #[test]
    fn save_load_roundtrip_is_character_perfect() {
        let dir = test_dir("roundtrip");
        let song = dir.join("song.flac");
        fs::File::create(&song).expect("stub audio file");

        let lrc_text = "[ti:Track]\r\n[00:01.00]<00:01.20>Some \u{e4}\u{266b} text\r\n";
        let written = save(song.to_str().unwrap(), lrc_text).expect("save");
        assert_eq!(written.ends_with("song.lrc"), true);

        let loaded = load(song.to_str().unwrap()).expect("load").expect("present");
        assert_eq!(loaded, lrc_text); // \r\n and non-ASCII survive untouched

        assert_eq!(delete(song.to_str().unwrap()).expect("delete"), true);
        assert_eq!(load(song.to_str().unwrap()).expect("load"), None);
    }

    #[test]
    fn save_overwrites_existing_sidecar() {
        let dir = test_dir("overwrite");
        let song = dir.join("track.mp3");
        let _ = fs::remove_file(&dir.join("track.lrc"));

        save(song.to_str().unwrap(), "v1").expect("first save");
        save(song.to_str().unwrap(), "v2").expect("second save");
        assert_eq!(load(song.to_str().unwrap()).expect("load").as_deref(), Some("v2"));
    }

    #[test]
    fn load_and_delete_tolerate_missing_files() {
        let dir = test_dir("missing");
        let song = dir.join("gone.wav");
        assert_eq!(load(song.to_str().unwrap()).expect("load"), None);
        assert_eq!(delete(song.to_str().unwrap()).expect("delete"), false);
        assert!(load("").is_err()); // invalid path → Err, not a false negative
    }
}
