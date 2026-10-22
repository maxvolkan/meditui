//! Pomodoro: focused work sprints with breaks in between.
//!
//! Work ends with a bell and a notification, then waits at 00:00 until you're
//! ready, so you can finish your thought before the break starts. Breaks are
//! either a blank, low-stimulation countdown or one of the meditations.

use serde::{Deserialize, Serialize};

use crate::app::MAX_MINUTES;
use crate::session::Countdown;

/// Work / break lengths in minutes, offered on the setup screen.
pub const PRESETS: [(u64, u64); 3] = [(25, 5), (40, 10), (50, 10)];

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum BreakMode {
    /// A calm countdown and nothing else: look away and rest your eyes.
    #[default]
    Blank,
    /// A short meditation that lasts the length of the break.
    Meditation,
}

impl BreakMode {
    pub fn label(self) -> &'static str {
        match self {
            BreakMode::Blank => "blank wall",
            BreakMode::Meditation => "meditation",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            BreakMode::Blank => BreakMode::Meditation,
            BreakMode::Meditation => BreakMode::Blank,
        }
    }
}

/// What you last chose, remembered for next time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroSettings {
    pub work: u64,
    pub rest: u64,
    pub break_mode: BreakMode,
    /// The meditation for meditation breaks, by name.
    pub meditation: Option<String>,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        Self {
            work: 25,
            rest: 5,
            break_mode: BreakMode::Blank,
            meditation: None,
        }
    }
}

impl PomodoroSettings {
    /// Moves to the next (or previous) preset. Lengths that aren't a preset,
    /// like ones from `medi pomo 30/7`, step to the first one.
    pub fn cycle_preset(&mut self, step: isize) {
        let current = PRESETS
            .iter()
            .position(|&lengths| lengths == (self.work, self.rest));
        let next = match current {
            Some(index) => (index as isize + step).rem_euclid(PRESETS.len() as isize) as usize,
            None => 0,
        };
        (self.work, self.rest) = PRESETS[next];
    }
}

/// Reads "25/5" (work/break minutes), as typed after `medi pomo`.
pub fn parse_lengths(text: &str) -> Result<(u64, u64), String> {
    let usage = || format!("expected work/break minutes like 25/5, got '{text}'");
    let (work, rest) = text.split_once('/').ok_or_else(usage)?;
    let work: u64 = work.trim().parse().map_err(|_| usage())?;
    let rest: u64 = rest.trim().parse().map_err(|_| usage())?;
    for minutes in [work, rest] {
        if !(1..=MAX_MINUTES).contains(&minutes) {
            return Err(format!(
                "each length must be between 1 and {MAX_MINUTES} minutes"
            ));
        }
    }
    Ok((work, rest))
}

#[derive(Debug)]
pub enum Phase {
    Work(Countdown),
    /// The sprint's over; waiting for you to say you're ready for a break.
    WorkDone,
    /// A blank break.
    Break(Countdown),
    /// A meditation break, running on the normal session screen.
    MeditationBreak,
    /// The break's over; waiting for you to start the next sprint.
    BreakDone,
}

/// Something worth an alert.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    SprintDone,
    BreakOver,
}

#[derive(Debug)]
pub struct Pomodoro {
    pub settings: PomodoroSettings,
    pub phase: Phase,
    /// Sprints finished so far in this run.
    pub sprints: u32,
    /// True while asking "stop the pomodoro?".
    pub stopping: bool,
}

impl Pomodoro {
    pub fn start(settings: &PomodoroSettings) -> Self {
        Self {
            settings: settings.clone(),
            phase: Phase::Work(Countdown::start(settings.work)),
            sprints: 0,
            stopping: false,
        }
    }

    /// The running clock, if this phase has one.
    pub fn clock(&self) -> Option<&Countdown> {
        match &self.phase {
            Phase::Work(clock) | Phase::Break(clock) => Some(clock),
            _ => None,
        }
    }

    pub fn clock_mut(&mut self) -> Option<&mut Countdown> {
        match &mut self.phase {
            Phase::Work(clock) | Phase::Break(clock) => Some(clock),
            _ => None,
        }
    }

    /// Moves on when a clock runs out. Returns what just happened, if anything.
    pub fn tick(&mut self) -> Option<Event> {
        match &self.phase {
            Phase::Work(clock) if clock.is_finished() => {
                self.phase = Phase::WorkDone;
                self.sprints += 1;
                Some(Event::SprintDone)
            }
            Phase::Break(clock) if clock.is_finished() => {
                self.phase = Phase::BreakDone;
                Some(Event::BreakOver)
            }
            _ => None,
        }
    }

    pub fn begin_work(&mut self) {
        self.phase = Phase::Work(Countdown::start(self.settings.work));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_parse_from_the_command_line() {
        assert_eq!(parse_lengths("25/5"), Ok((25, 5)));
        assert_eq!(parse_lengths(" 50 / 10 "), Ok((50, 10)));
        assert!(parse_lengths("25").unwrap_err().contains("like 25/5"));
        assert!(parse_lengths("a/b").unwrap_err().contains("like 25/5"));
        assert!(parse_lengths("0/5").unwrap_err().contains("between 1"));
        assert!(parse_lengths("25/999").unwrap_err().contains("between 1"));
    }

    #[test]
    fn presets_cycle_both_ways() {
        let mut settings = PomodoroSettings::default();
        settings.cycle_preset(1);
        assert_eq!((settings.work, settings.rest), (40, 10));
        settings.cycle_preset(1);
        settings.cycle_preset(1);
        assert_eq!((settings.work, settings.rest), (25, 5));
        settings.cycle_preset(-1);
        assert_eq!((settings.work, settings.rest), (50, 10));

        settings.work = 30;
        settings.cycle_preset(1);
        assert_eq!((settings.work, settings.rest), (25, 5));
    }

    #[test]
    fn a_finished_sprint_waits_for_you() {
        let mut pomodoro = Pomodoro::start(&PomodoroSettings::default());
        assert_eq!(pomodoro.tick(), None);

        pomodoro.phase = Phase::Work(Countdown::start(0));
        assert_eq!(pomodoro.tick(), Some(Event::SprintDone));
        assert!(matches!(pomodoro.phase, Phase::WorkDone));
        assert_eq!(pomodoro.sprints, 1);
        // Nothing more happens until you're ready.
        assert_eq!(pomodoro.tick(), None);

        pomodoro.phase = Phase::Break(Countdown::start(0));
        assert_eq!(pomodoro.tick(), Some(Event::BreakOver));
        assert!(matches!(pomodoro.phase, Phase::BreakDone));

        pomodoro.begin_work();
        let clock = pomodoro.clock().unwrap();
        assert_eq!(clock.total.as_secs(), 25 * 60);
    }
}
