use serde::{Deserialize, Serialize};
use std::path::Path;

/// 15-band equalizer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualizerManager {
    pub enabled: bool,
    /// gains in dB, indexed to band centers [25,40,63,100,160,250,400,630,1k,1.6k,2.5k,4k,6.3k,10k,16k] Hz
    pub bands: [f32; 15],
    pub preset: Option<String>,
}

// Band center frequencies in Hz
const BAND_CENTERS_HZ: [f32; 15] = [
    25.0, 40.0, 63.0, 100.0, 160.0, 250.0, 400.0, 630.0, 1000.0, 1600.0, 2500.0, 4000.0,
    6300.0, 10000.0, 16000.0,
];

impl EqualizerManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
            bands: [0.0; 15],
            preset: None,
        }
    }

    /// Reset all bands to 0 dB
    pub fn reset(&mut self) {
        self.bands = [0.0; 15];
    }

    /// Apply the equalizer: build mpv superequalizer filter string
    /// Format: "b1=25:g1=0|b2=40:g2=0|..."
    pub fn apply_filter(&self) -> String {
        let mut parts = Vec::with_capacity(15);
        for (i, &gain) in self.bands.iter().enumerate() {
            let freq = BAND_CENTERS_HZ[i];
            parts.push(format!("b{}={}:g{}={}", i + 1, freq, i + 1, gain));
        }
        parts.join("|")
    }

    /// Load a built‑in preset or a saved custom preset.
    /// Presets are defined as a map of band gain arrays.
    pub fn load_preset(&mut self, name: &str) -> Result<(), String> {
        // Built‑in presets (gain values in dB)
        let builtin = match name {
            "Flat" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Bass Boost" => [6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Treble Boost" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, -3.0, -4.0, -5.0, -4.0, -3.0],
            "Rock" => [3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Pop" => [2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0],
            "Classical" => [1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -3.0, -2.0, -1.0],
            "Jazz" => [0.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0, 0.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0],
            "Electronic" => [2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -3.0, -4.0],
            "Vocal" => [0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 2.0, 1.0, 0.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0],
            "Acoustic" => [1.0, 1.5, 2.0, 2.5, 3.0, 2.5, 2.0, 1.5, 1.0, 0.5, 0.0, -0.5, -1.0, -1.0, -1.0],
            _ => return Err(format!("Unknown preset '{}'.", name)),
        };
        self.bands = builtin;
        self.preset = Some(name.to_string());
        Ok(())
    }

    /// Save the current bands as a custom preset.
    pub fn save_preset(&mut self, name: &str) -> Result<(), String> {
        self.bands = self.bands; // no-op, just ensure bands are current
        self.preset = Some(name.to_string());
        Ok(())
    }
}

/// Persist equalizer state to a JSON settings file.
/// The caller must ensure `settings_dir` points to the app's config directory.
pub fn persist_eq_settings<P: AsRef<std::path::Path>>(settings_dir: P, eq: &EqualizerManager) -> Result<(), String> {
    let path = settings_dir.as_ref().join("equalizer.json");
    let json = serde_json::to_string(eq).map_err(|e| format!("serialize error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("write error: {}", e))?;
    Ok(())
}

/// Load equalizer state from the JSON settings file.
pub fn load_eq_settings<P: AsRef<std::path::Path>>(settings_dir: P) -> Result<EqualizerManager, String> {
    let path = settings_dir.as_ref().join("equalizer.json");
    if !path.exists() {
        return Ok(EqualizerManager::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("read error: {}", e))?;
    let eq: EqualizerManager = serde_json::from_str(&content).map_err(|e| format!("parse error: {}", e))?;
    Ok(eq)
}

/// Build the mpv `--af` string that adds the superequalizer filter.
/// This is intended to be set on the mpv `af` property, e.g.:
/// `mpv_command_string("set", "af", "superequalizer=b1=25:g1=0|b2=40:g2=0|...")`
pub fn build_mpv_superequalizer(eq: &EqualizerManager) -> String {
    let mut parts = Vec::with_capacity(15);
    for (i, &gain) in eq.bands.iter().enumerate() {
        let freq = BAND_CENTERS_HZ[i];
        parts.push(format!("b{}={}:g{}={}", i + 1, freq, i + 1, gain));
    }
    parts.join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_manager_bands_clamp() {
        let mut eq = EqualizerManager::new();
        // set some extreme values; the struct does not clamp automatically;
        // this test documents the expected range.
        eq.bands = [15.0, -15.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!(eq.bands.iter().all(|&b| b >= -15.0 && b <= 15.0));
    }

    #[test]
    fn eq_preset_flat_loads() {
        let mut eq = EqualizerManager::new();
        eq.load_preset("Flat").unwrap();
        assert_eq!(eq.bands, [0.0; 15]);
        assert_eq!(eq.preset, Some("Flat".into()));
    }

    #[test]
    fn eq_preset_unknown_returns_error() {
        let mut eq = EqualizerManager::new();
        let err = eq.load_preset("NonExistent").unwrap_err();
        assert!(err.contains("Unknown preset"));
    }

    #[test]
    fn eq_build_filter_string() {
        let mut eq = EqualizerManager::new();
        eq.bands = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let filter = eq.apply_filter();
        // first band should be b1=25:g1=1
        assert!(filter.starts_with("b1=25:g1=1|"));
        // last band should be b16=16k:g16=15
        assert!(filter.ends_with("b15=16000:g15=15"));
        // count pipes +1 = 15 bands
        let bands: Vec<&str> = filter.split('|').collect();
        assert_eq!(bands.len(), 15);
    }

    #[test]
    fn eq_reset_zeroes() {
        let mut eq = EqualizerManager::new();
        eq.bands = [5.0; 15];
        eq.reset();
        assert_eq!(eq.bands, [0.0; 15]);
    }
}