//! Timers: a pausable countdown, and a running meditation built on one.

use std::time::{Duration, Instant};

use crate::library::Meditation;

/// A countdown that can be paused and resumed.
#[derive(Debug)]
pub struct Countdown {
    pub total: Duration,
    /// Time already counted before the most recent pause.
    banked: Duration,
    /// When the current stretch started. `None` while paused.
    running_since: Option<Instant>,
}

impl Countdown {
    pub fn start(minutes: u64) -> Self {
        Self {
            total: Duration::from_secs(minutes * 60),
            banked: Duration::ZERO,
            running_since: Some(Instant::now()),
        }
    }

    pub fn elapsed(&self) -> Duration {
        let current = self
            .running_since
            .map_or(Duration::ZERO, |since| since.elapsed());
        (self.banked + current).min(self.total)
    }

    pub fn remaining(&self) -> Duration {
        self.total.saturating_sub(self.elapsed())
    }

    pub fn progress(&self) -> f64 {
        if self.total.is_zero() {
            return 1.0;
        }
        (self.elapsed().as_secs_f64() / self.total.as_secs_f64()).clamp(0.0, 1.0)
    }

    pub fn is_finished(&self) -> bool {
        self.remaining().is_zero()
    }

    pub fn is_paused(&self) -> bool {
        self.running_since.is_none()
    }

    pub fn toggle_pause(&mut self) {
        match self.running_since.take() {
            Some(since) => self.banked += since.elapsed(),
            None => self.running_since = Some(Instant::now()),
        }
    }
}

/// A running meditation.
#[derive(Debug)]
pub struct Session {
    pub meditation: Meditation,
    pub clock: Countdown,
    /// The breath phase or cue index we last made a sound for.
    pub last_cue: Option<usize>,
    /// How many interval bells have rung so far.
    pub interval_bells: u64,
}

impl Session {
    pub fn start(meditation: Meditation, minutes: u64) -> Self {
        Self {
            meditation,
            clock: Countdown::start(minutes),
            last_cue: None,
            interval_bells: 0,
        }
    }

    pub fn total(&self) -> Duration {
        self.clock.total
    }

    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    pub fn remaining(&self) -> Duration {
        self.clock.remaining()
    }

    pub fn progress(&self) -> f64 {
        self.clock.progress()
    }

    pub fn is_finished(&self) -> bool {
        self.clock.is_finished()
    }

    pub fn is_paused(&self) -> bool {
        self.clock.is_paused()
    }

    pub fn toggle_pause(&mut self) {
        self.clock.toggle_pause();
    }
}

/// 65 seconds -> "01:05". Rounds up, so a fresh 10 minute timer shows 10:00.
pub fn format_clock(duration: Duration) -> String {
    let total_seconds = duration.as_secs_f64().ceil() as u64;
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// Time sat so far, like `format_clock` but rounding down, so a fresh
/// session shows 00:00.
pub fn format_elapsed(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// 0 -> "0m", 2700 -> "45m", 11100 -> "3h 05m"
pub fn format_total(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Meditation;

    #[test]
    fn clock_formats_and_rounds_up() {
        assert_eq!(format_clock(Duration::from_secs(65)), "01:05");
        assert_eq!(format_clock(Duration::from_secs(600)), "10:00");
        assert_eq!(format_clock(Duration::ZERO), "00:00");
        assert_eq!(format_clock(Duration::from_millis(9_200)), "00:10");
        assert_eq!(format_elapsed(Duration::from_millis(9_800)), "00:09");
        assert_eq!(format_elapsed(Duration::from_secs(65)), "01:05");
    }

    #[test]
    fn totals_read_nicely() {
        assert_eq!(format_total(0), "0m");
        assert_eq!(format_total(2700), "45m");
        assert_eq!(format_total(11_100), "3h 05m");
    }

    #[test]
    fn pausing_stops_the_clock() {
        let mut session = Session::start(Meditation::custom("t".into(), 1), 1);
        session.toggle_pause();
        assert!(session.is_paused());
        let frozen = session.elapsed();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(session.elapsed(), frozen);

        session.toggle_pause();
        assert!(!session.is_paused());
        std::thread::sleep(Duration::from_millis(30));
        assert!(session.elapsed() > frozen);
    }

    #[test]
    fn fresh_session_has_full_time_left() {
        let session = Session::start(Meditation::custom("t".into(), 10), 10);
        assert_eq!(format_clock(session.remaining()), "10:00");
        assert!(!session.is_finished());
    }
}
