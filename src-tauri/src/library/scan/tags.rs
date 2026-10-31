use std::path::Path;

use lofty::{read_from_path, TaggedFileExt, PictureType};

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub duration: Option<u32>, // seconds
    pub cover: Option<Vec<u8>>,
    pub replay_gain_track: Option<String>,
    pub replay_gain_album: Option<String>,
    pub replay_peak_track: Option<String>,
    pub replay_peak_album: Option<String>,
}

#[derive(Debug)]
pub enum ScanError {
    Io(std::io::Error),
    Lofty(lofty::Error),
    Other(String),
}

impl From<std::io::Error> for ScanError {
    fn from(e: std::io::Error) -> Self {
        ScanError::Io(e)
    }
}

impl From<lofty::Error> for ScanError {
    fn from(e: lofty::Error) -> Self {
        ScanError::Lofty(e)
    }
}

pub fn extract_tags(path: &Path) -> Result<TrackMeta, ScanError> {
    let tagged = read_from_path(path).map_err(ScanError::Lofty)?;
    let tag = tagged.primary_tag().cloned();

    let props = tagged.properties();
    let duration_secs = props.duration().as_secs() as u32;

    let title = tag.as_ref().and_then(|t| t.title().map(|s| s.to_string()));
    let artist = tag.as_ref().and_then(|t| t.artist().map(|s| s.to_string()));
    let album = tag.as_ref().and_then(|t| t.album().map(|s| s.to_string()));
    let album_artist = tag.as_ref().and_then(|t| t.album_artist().map(|s| s.to_string()));
    let track = tag.as_ref().and_then(|t| t.track());
    let disc = tag.as_ref().and_then(|t| t.disc());
    let year = tag.as_ref().and_then(|t| t.year());
    let genre = tag.as_ref().and_then(|t| t.genre().map(|s| s.to_string()));

    // ReplayGain tags
    let replay_gain_track = tag.as_ref().and_then(|t| {
        t.get("\0").or_else(|| {
            // Fallback: search items
            t.items().find(|i| i.key().eq_ignore_ascii_case("REPLAYGAIN_TRACK_GAIN")).map(|i| i.value().to_string())
        })
    }).or_else(|| {
        tag.as_ref().and_then(|t| t.get_item("REPLAYGAIN_TRACK_GAIN").map(|v| v.to_string()))
    });
    // Simpler approach using get_item
    let replay_gain_track = tag.as_ref().and_then(|t| t.get_item("REPLAYGAIN_TRACK_GAIN").map(|v| v.to_string()));
    let replay_gain_album = tag.as_ref().and_then(|t| t.get_item("REPLAYGAIN_ALBUM_GAIN").map(|v| v.to_string()));
    let replay_peak_track = tag.as_ref().and_then(|t| t.get_item("REPLAYGAIN_TRACK_PEAK").map(|v| v.to_string()));
    let replay_peak_album = tag.as_ref().and_then(|t| t.get_item("REPLAYGAIN_ALBUM_PEAK").map(|v| v.to_string()));

    // Cover art: prefer CoverFront
    let cover = tagged.attachments().find(|p| p.picture_type() == PictureType::CoverFront)
        .or_else(|| tagged.attachments().next())
        .and_then(|pic| {
            let data = pic.data();
            if data.len() > 512 * 1024 {
                None
            } else {
                Some(data.to_vec())
            }
        });

    Ok(TrackMeta {
        path: path.to_string_lossy().into_owned(),
        title,
        artist,
        album,
        album_artist,
        track,
        disc,
        year,
        genre,
        duration: Some(duration_secs),
        cover,
        replay_gain_track,
        replay_gain_album,
        replay_peak_track,
        replay_peak_album,
    })
}
