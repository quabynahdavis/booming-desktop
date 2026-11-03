use std::path::Path;

use lofty::{read_from_path};
use lofty::file::{TaggedFileExt, AudioFile};
use lofty::picture::PictureType;
use lofty::tag::{ItemKey, Accessor};
use lofty::error::FileParseError;

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
    pub duration: Option<u32>,
    pub cover: Option<Vec<u8>>,
    pub replay_gain_track: Option<String>,
    pub replay_gain_album: Option<String>,
    pub replay_peak_track: Option<String>,
    pub replay_peak_album: Option<String>,
}

#[derive(Debug)]
pub enum ScanError {
    Io(std::io::Error),
    Lofty(FileParseError),
    Other(String),
}

impl From<std::io::Error> for ScanError {
    fn from(e: std::io::Error) -> Self {
        ScanError::Io(e)
    }
}

impl From<FileParseError> for ScanError {
    fn from(e: FileParseError) -> Self {
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
    let album_artist = tag.as_ref().and_then(|t| t.get_string(ItemKey::AlbumArtist).map(|s| s.to_string()));
    let track = tag.as_ref().and_then(|t| t.track());
    let disc = tag.as_ref().and_then(|t| t.disk());
    let year = tag.as_ref().and_then(|t| t.get_string(ItemKey::Year)).and_then(|s| s.parse::<u32>().ok());
    let genre = tag.as_ref().and_then(|t| t.genre().map(|s| s.to_string()));

    let cover = tag.as_ref().and_then(|t| t.get_picture_type(PictureType::CoverFront)).and_then(|pic| {
        let data = pic.data();
        if data.len() > 512 * 1024 { None } else { Some(data.to_vec()) }
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
        replay_gain_track: None,
        replay_gain_album: None,
        replay_peak_track: None,
        replay_peak_album: None,
    })
}
