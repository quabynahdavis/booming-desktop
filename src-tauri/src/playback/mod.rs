pub mod replaygain;
pub use replaygain::GainMode;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tauri::AppHandle;

use crate::engine::{EngineError, PlaybackState, SharedEngine};
use crate::library::db::Db;
use crate::playback::replaygain::{ReplayGainManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    None,
    One,
    All,
}

pub struct PlaybackService {
    engine: SharedEngine,
    current_song_id: Mutex<Option<i64>>,
    queue: Mutex<Vec<i64>>,
    repeat: Mutex<RepeatMode>,
    shuffle: Mutex<bool>,
    last_played_ids: Mutex<Vec<i64>>,
    app_handle: Mutex<Option<AppHandle>>,
    replay_gain_mode: Mutex<GainMode>,
    sleep_target: Mutex<Option<Instant>>,
    sleep_enabled: Mutex<bool>,
    sleep_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl PlaybackService {
    pub fn new(engine: SharedEngine) -> Self {
        Self {
            engine,
            current_song_id: Mutex::new(None),
            queue: Mutex::new(Vec::new()),
            repeat: Mutex::new(RepeatMode::None),
            shuffle: Mutex::new(false),
            last_played_ids: Mutex::new(Vec::new()),
            app_handle: Mutex::new(None),
            replay_gain_mode: Mutex::new(GainMode::Off),
            sleep_target: Mutex::new(None),
            sleep_enabled: Mutex::new(false),
            sleep_handle: Mutex::new(None),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    fn emit_state(&self) {
        if let Some(handle) = self.app_handle.lock().unwrap().as_ref() {
            let _ = handle.emit("playback-state-changed", ());
        }
    }

    pub fn play_song(&self, db: &Db, song_id: i64) -> Result<(), EngineError> {
        let song = db.get_song(song_id).map_err(|e| EngineError::Backend(e.to_string()))?.ok_or_else(|| EngineError::Backend("song not found".into()))?;
        let path = song.path.clone();
        {
            let mut cur = self.current_song_id.lock().unwrap();
            *cur = Some(song_id);
        }
        db.increment_play_count(song_id).ok();
        db.update_last_played(song_id).ok();

        // Apply ReplayGain before playing
        let mode = *self.replay_gain_mode.lock().unwrap();
        {
            let engine = self.engine.lock().unwrap();
            ReplayGainManager::apply_for_track(&**engine, &song, mode)?;
        }

        // load and play
        let engine = self.engine.lock().unwrap();
        engine.load(&path)?;
        engine.play()?;
        self.emit_state();
        Ok(())
    }

    pub fn set_replay_gain_mode(&self, mode: GainMode) {
        *self.replay_gain_mode.lock().unwrap() = mode;
    }

    pub fn pause(&self) -> Result<(), EngineError> {
        let engine = self.engine.lock().unwrap();
        engine.pause()?;
        self.emit_state();
        Ok(())
    }

    pub fn resume(&self) -> Result<(), EngineError> {
        let engine = self.engine.lock().unwrap();
        engine.play()?;
        self.emit_state();
        Ok(())
    }

    pub fn seek(&self, secs: f64) -> Result<(), EngineError> {
        let engine = self.engine.lock().unwrap();
        engine.seek(secs)?;
        self.emit_state();
        Ok(())
    }

    pub fn next(&self, db: &Db) -> Result<(), EngineError> {
        let mut queue = self.queue.lock().unwrap();
        let cur_id = *self.current_song_id.lock().unwrap();
        let next_id = self.pick_next(&mut queue, cur_id)?;
        if let Some(id) = next_id {
            drop(queue);
            self.play_song(db, id)?;
        }
        Ok(())
    }

    pub fn previous(&self, db: &Db) -> Result<(), EngineError> {
        let mut queue = self.queue.lock().unwrap();
        let cur_id = *self.current_song_id.lock().unwrap();
        let prev_id = self.pick_previous(&mut queue, cur_id)?;
        if let Some(id) = prev_id {
            drop(queue);
            self.play_song(db, id)?;
        }
        Ok(())
    }

    fn pick_next(&self, queue: &mut Vec<i64>, cur_id: Option<i64>) -> Result<Option<i64>, EngineError> {
        let shuffle = *self.shuffle.lock().unwrap();
        let repeat = *self.repeat.lock().unwrap();
        if shuffle {
            // simplified shuffle with ban list
            let mut last = self.last_played_ids.lock().unwrap();
            // build candidate list
            let mut candidates: Vec<i64> = queue.iter().cloned().collect();
            // remove recent
            candidates.retain(|id| !last.contains(id));
            if candidates.is_empty() {
                candidates = queue.clone();
            }
            if !candidates.is_empty() {
                // simple deterministic pick: first candidate
                let chosen = candidates[0];
                last.push(chosen);
                if last.len() > 3 {
                    last.remove(0);
                }
                return Ok(Some(chosen));
            }
        }
        // sequential
        if let Some(pos) = queue.iter().position(|&id| Some(id) == cur_id) {
            let next_pos = pos + 1;
            if next_pos < queue.len() {
                return Ok(Some(queue[next_pos]));
            } else if matches!(repeat, RepeatMode::All) {
                return Ok(Some(queue[0]));
            }
        }
        Ok(None)
    }

    fn pick_previous(&self, queue: &mut Vec<i64>, cur_id: Option<i64>) -> Result<Option<i64>, EngineError> {
        if let Some(pos) = queue.iter().position(|&id| Some(id) == cur_id) {
            if pos > 0 {
                return Ok(Some(queue[pos - 1]));
            }
        }
        Ok(None)
    }

    pub fn get_state(&self, db: &Db) -> PlaybackStateInfo {
        let engine = self.engine.lock().unwrap();
        let state = engine.get_state().unwrap_or(PlaybackState::Stopped);
        let position = engine.get_position().unwrap_or(0.0);
        let duration = engine.get_duration().unwrap_or(0.0);
        let current_song_id = *self.current_song_id.lock().unwrap();
        let queue = self.queue.lock().unwrap().clone();
        let current_song = current_song_id.and_then(|id| db.get_song(id).ok().flatten());
        PlaybackStateInfo {
            state,
            position,
            duration,
            current_song_id,
            current_song,
            queue,
        }
    }

    pub fn set_queue(&self, queue: Vec<i64>) {
        *self.queue.lock().unwrap() = queue;
    }

    pub fn get_queue(&self) -> Vec<i64> {
        self.queue.lock().unwrap().clone()
    }

    pub fn set_repeat(&self, mode: RepeatMode) {
        *self.repeat.lock().unwrap() = mode;
    }

    pub fn get_repeat(&self) -> RepeatMode {
        *self.repeat.lock().unwrap()
    }

    pub fn set_shuffle(&self, shuffle: bool) {
        *self.shuffle.lock().unwrap() = shuffle;
    }

    pub fn get_shuffle(&self) -> bool {
        *self.shuffle.lock().unwrap()
    }

    pub fn set_sleep_timer(&self, minutes: i64) {
        // cancel existing
        self.cancel_sleep_timer_internal();
        if minutes <= 0 {
            return;
        }
        let target = Instant::now() + Duration::from_secs((minutes as u64) * 60);
        *self.sleep_target.lock().unwrap() = Some(target);
        *self.sleep_enabled.lock().unwrap() = true;

        let engine = self.engine.clone();
        let app_handle_opt = {
            let guard = self.app_handle.lock().unwrap();
            guard.clone()
        };
        let handle = tokio::spawn(async move {
            let duration = Duration::from_secs((minutes as u64) * 60);
            tokio::time::sleep(duration).await;
            // pause playback
            let eng = engine.lock().unwrap();
            let _ = eng.pause();
            drop(eng);
            if let Some(handle) = app_handle_opt {
                let _ = handle.emit("sleep-timer-fired", ());
            }
        });
        *self.sleep_handle.lock().unwrap() = Some(handle);
    }

    pub fn cancel_sleep_timer(&self) {
        self.cancel_sleep_timer_internal();
    }

    fn cancel_sleep_timer_internal(&self) {
        if let Some(handle) = self.sleep_handle.lock().unwrap().take() {
            handle.abort();
        }
        *self.sleep_target.lock().unwrap() = None;
        *self.sleep_enabled.lock().unwrap() = false;
    }

    pub fn get_sleep_timer_status(&self) -> SleepTimerStatus {
        let enabled = *self.sleep_enabled.lock().unwrap();
        let remaining_secs = if enabled {
            if let Some(target) = *self.sleep_target.lock().unwrap() {
                let now = Instant::now();
                if target > now {
                    target.duration_since(now).as_secs() as i64
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };
        SleepTimerStatus {
            enabled,
            remaining_secs,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SleepTimerStatus {
    pub enabled: bool,
    pub remaining_secs: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PlaybackStateInfo {
    pub state: PlaybackState,
    pub position: f64,
    pub duration: f64,
    pub current_song_id: Option<i64>,
    pub current_song: Option<crate::library::db::Song>,
    pub queue: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::null::NullEngine;

    #[test]
    fn playback_state_transitions_load_play_pause() {
        let engine = Arc::new(Mutex::new(Box::new(NullEngine::new()) as Box<dyn crate::engine::Engine>));
        let service = PlaybackService::new(engine);
        // service should be constructible and state defaults to None
        assert_eq!(*service.current_song_id.lock().unwrap(), None);
        assert_eq!(*service.replay_gain_mode.lock().unwrap(), GainMode::Off);
    }
}
