//! Saving settings, your own meditations and stats between runs.
//!
//! Everything lives in one JSON file: ~/.local/share/meditui/data.json on
//! Linux (or $XDG_DATA_HOME/meditui/data.json if that's set).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pomodoro::PomodoroSettings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Show the tour on the next launch. Turned off after the tour is
    /// finished or skipped, and back on from the settings screen.
    pub show_tutorial: bool,
    /// Show the short prep checklist before each session.
    pub preflight: bool,
    /// Master switch for every sound.
    pub sound: bool,
    /// 10 to 100, in steps of 10.
    pub volume: u8,
    /// Soft rising/falling tones when the breath turns.
    pub breath_tones: bool,
    /// Noise that plays quietly under a session.
    pub background: Background,
    /// Hide the circle and cues during sessions, leaving only a dim clock
    /// (for eyes-closed practice). Toggled with `h`.
    pub minimal_view: bool,
    /// What the small clock under a session shows.
    pub clock: ClockStyle,
    /// Dim the practices that usually come later, and ask before starting
    /// one early.
    pub follow_path: bool,
    /// Practices you've unlocked by hand (`u` on the "comes later" popup).
    pub unlocked: BTreeSet<String>,
    /// Library folders you've closed, by `Folder::key`.
    pub closed_folders: BTreeSet<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_tutorial: true,
            preflight: true,
            sound: true,
            volume: 70,
            breath_tones: true,
            background: Background::Off,
            minimal_view: false,
            clock: ClockStyle::Remaining,
            follow_path: true,
            unlocked: BTreeSet::new(),
            closed_folders: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum Background {
    #[default]
    Off,
    BrownNoise,
    Ocean,
}

impl Background {
    const ALL: [Background; 3] = [Background::Off, Background::BrownNoise, Background::Ocean];

    pub fn label(self) -> &'static str {
        match self {
            Background::Off => "off",
            Background::BrownNoise => "brown noise",
            Background::Ocean => "ocean waves",
        }
    }

    /// The next option, wrapping around. `step` is 1 or -1.
    pub fn cycle(self, step: isize) -> Self {
        let index = Self::ALL.iter().position(|b| *b == self).unwrap_or(0) as isize;
        let len = Self::ALL.len() as isize;
        Self::ALL[(index + step).rem_euclid(len) as usize]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum ClockStyle {
    /// Time left, counting down.
    #[default]
    Remaining,
    /// Time sat so far, counting up.
    Elapsed,
    /// No numbers, just the progress line.
    Hidden,
}

impl ClockStyle {
    const ALL: [ClockStyle; 3] = [
        ClockStyle::Remaining,
        ClockStyle::Elapsed,
        ClockStyle::Hidden,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ClockStyle::Remaining => "time left",
            ClockStyle::Elapsed => "time sat",
            ClockStyle::Hidden => "hidden",
        }
    }

    /// The next option, wrapping around. `step` is 1 or -1.
    pub fn cycle(self, step: isize) -> Self {
        let index = Self::ALL.iter().position(|c| *c == self).unwrap_or(0) as isize;
        let len = Self::ALL.len() as isize;
        Self::ALL[(index + step).rem_euclid(len) as usize]
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// Sessions sat all the way to the end.
    pub sessions: u64,
    /// Every second spent meditating, including sessions ended early.
    pub seconds: u64,
    /// Completed sessions per local date, like "2026-10-22": 2.
    pub days: BTreeMap<String, u32>,
    /// Pomodoro work sprints finished.
    pub sprints: u64,
    /// The days each meditation was sat to the end, by name. This is what
    /// unlocks the practices that come later.
    pub practice: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomMeditation {
    pub name: String,
    pub minutes: u64,
}

// `#[serde(default)]` means a missing field (say, from an older version of
// the file) gets its default instead of failing the whole load.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub settings: Settings,
    pub custom: Vec<CustomMeditation>,
    pub stats: Stats,
    pub pomodoro: PomodoroSettings,
}

pub fn data_path() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("meditui").join("data.json"))
}

/// Never fails: a missing file is a first run, and an unreadable file is
/// set aside (renamed to data.json.broken) so a fresh one can be started.
pub fn load() -> SaveData {
    let Some(path) = data_path() else {
        return SaveData::default();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return SaveData::default();
    };
    match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(_) => {
            let _ = fs::rename(&path, path.with_extension("json.broken"));
            SaveData::default()
        }
    }
}

pub fn save(path: &Path, data: &SaveData) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(data).map_err(io::Error::other)?;
    // Write to a temporary file first, then swap it in, so quitting halfway
    // through a write can never leave a half-written data.json behind.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let mut data = SaveData::default();
        data.settings.sound = false;
        data.settings.background = Background::Ocean;
        data.custom.push(CustomMeditation {
            name: "Morning sit".to_string(),
            minutes: 12,
        });
        data.stats.sessions = 3;
        data.settings.clock = ClockStyle::Hidden;
        data.settings.unlocked.insert("Bhastrika".to_string());
        data.settings
            .closed_folders
            .insert("deep_unwind".to_string());
        data.stats
            .practice
            .entry("Nadi Shuddhi".to_string())
            .or_default()
            .insert("2026-10-22".to_string());

        let text = serde_json::to_string(&data).unwrap();
        let back: SaveData = serde_json::from_str(&text).unwrap();
        assert_eq!(back, data);
    }

    #[test]
    fn background_cycles_both_ways() {
        assert_eq!(Background::Off.cycle(1), Background::BrownNoise);
        assert_eq!(Background::Ocean.cycle(1), Background::Off);
        assert_eq!(Background::Off.cycle(-1), Background::Ocean);
        assert_eq!(ClockStyle::Remaining.cycle(1), ClockStyle::Elapsed);
        assert_eq!(ClockStyle::Remaining.cycle(-1), ClockStyle::Hidden);
    }

    #[test]
    fn first_run_shows_the_tutorial() {
        assert!(SaveData::default().settings.show_tutorial);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let data: SaveData = serde_json::from_str(r#"{"settings": {"sound": false}}"#).unwrap();
        assert!(!data.settings.sound);
        assert!(data.settings.show_tutorial);
        assert_eq!(data.settings.volume, 70);
        assert!(data.custom.is_empty());
        assert_eq!(data.stats.sessions, 0);
        // Added later: older files don't have these.
        assert_eq!(data.settings.clock, ClockStyle::Remaining);
        assert!(data.settings.follow_path);
        assert!(data.settings.unlocked.is_empty() && data.settings.closed_folders.is_empty());
        assert!(data.stats.practice.is_empty());
    }
}
