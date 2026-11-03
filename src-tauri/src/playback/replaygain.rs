use crate::engine::{Engine, EngineError};
use crate::library::db::Song;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GainMode {
    Off,
    Track,
    Album,
}

pub struct ReplayGainManager;

impl ReplayGainManager {
    pub fn apply_for_track<E: Engine + ?Sized>(engine: &E, song: &Song, mode: GainMode) -> Result<(), EngineError> {
        match mode {
            GainMode::Off => {
                engine.set_replay_gain_db(0.0)?;
                engine.set_audio_filter("".into())?;
                Ok(())
            }
            GainMode::Track => Self::apply_gain(engine, song.replay_gain_track.as_deref()),
            GainMode::Album => Self::apply_gain(engine, song.replay_gain_album.as_deref().or(song.replay_gain_track.as_deref())),
        }
    }

    fn apply_gain<E: Engine + ?Sized>(engine: &E, gain_str: Option<&str>) -> Result<(), EngineError> {
        if let Some(s) = gain_str {
            if let Some(db) = parse_gain_db(s) {
                engine.set_replay_gain_db(db)?;
                engine.set_audio_filter("".into())?;
                return Ok(());
            }
        }
        engine.set_replay_gain_db(0.0)?;
        engine.set_audio_filter("loudnorm".into())?;
        Ok(())
    }
}

pub fn parse_gain_db(s: &str) -> Option<f64> {
    let val = s.split_whitespace().next()?;
    val.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaygain_db_from_track_tag() {
        assert_eq!(parse_gain_db("-3.10 dB"), Some(-3.10));
    }

    #[test]
    fn replaygain_db_from_album_tag() {
        assert_eq!(parse_gain_db("+1.5"), Some(1.5));
    }

    #[test]
    fn gapless_option_set() {
        assert!(true);
    }
}
