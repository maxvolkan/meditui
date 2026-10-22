//! Practice history: day streaks and the year-at-a-glance heatmap.
//!
//! History is stored as completed sessions per local date, keyed like
//! "2026-10-22" (see `Stats::days`).

use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate};

/// Shades for 1, 2, 3 and 4+ sessions in a day. Empty days are drawn as a
/// faint dot so the grid's shape shows even before you've started.
pub const SHADES: [char; 4] = ['░', '▒', '▓', '█'];
pub const EMPTY: char = '·';

/// Today, in the local time zone.
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn day_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn sessions_on(days: &BTreeMap<String, u32>, date: NaiveDate) -> u32 {
    days.get(&day_key(date)).copied().unwrap_or(0)
}

/// Days in a row with at least one session, counting back from today. If you
/// haven't sat yet today, yesterday's streak still counts until midnight.
pub fn current_streak(days: &BTreeMap<String, u32>, today: NaiveDate) -> u32 {
    let mut date = if sessions_on(days, today) > 0 {
        today
    } else {
        match today.pred_opt() {
            Some(yesterday) => yesterday,
            None => return 0,
        }
    };
    let mut streak = 0;
    while sessions_on(days, date) > 0 {
        streak += 1;
        match date.pred_opt() {
            Some(before) => date = before,
            None => break,
        }
    }
    streak
}

/// The longest run of consecutive days with a session, ever.
pub fn longest_streak(days: &BTreeMap<String, u32>) -> u32 {
    let mut dates: Vec<NaiveDate> = days
        .iter()
        .filter(|(_, count)| **count > 0)
        .filter_map(|(key, _)| NaiveDate::parse_from_str(key, "%Y-%m-%d").ok())
        .collect();
    dates.sort();
    let (mut best, mut run) = (0, 0);
    let mut previous: Option<NaiveDate> = None;
    for date in dates {
        run = match previous {
            Some(p) if p.succ_opt() == Some(date) => run + 1,
            _ => 1,
        };
        best = best.max(run);
        previous = Some(date);
    }
    best
}

/// One cell of the heatmap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    /// A day with this many sessions (capped at 4 for shading).
    Day { date: NaiveDate, sessions: u32 },
    /// Later this week: hasn't happened yet.
    Future,
}

impl Cell {
    pub fn glyph(self) -> char {
        match self {
            Cell::Day { sessions: 0, .. } => EMPTY,
            Cell::Day { sessions, .. } => SHADES[(sessions.min(4) - 1) as usize],
            Cell::Future => ' ',
        }
    }
}

/// A grid of 7 rows (Monday to Sunday) by `weeks` columns. The last column is
/// this week; each column to the left is a week earlier.
pub fn heatmap(days: &BTreeMap<String, u32>, today: NaiveDate, weeks: usize) -> Vec<Vec<Cell>> {
    let this_monday = today - Days::new(today.weekday().num_days_from_monday() as u64);
    let first_monday = this_monday - Days::new(7 * weeks.saturating_sub(1) as u64);
    (0..7)
        .map(|weekday| {
            (0..weeks)
                .map(|week| {
                    let date = first_monday + Days::new((week * 7 + weekday) as u64);
                    if date > today {
                        Cell::Future
                    } else {
                        Cell::Day {
                            date,
                            sessions: sessions_on(days, date),
                        }
                    }
                })
                .collect()
        })
        .collect()
}

/// Month names above the grid, placed over the first week of each month.
pub fn month_labels(grid: &[Vec<Cell>]) -> String {
    let Some(mondays) = grid.first() else {
        return String::new();
    };
    let mut row: Vec<char> = vec![' '; mondays.len()];
    let mut last_month = None;
    let mut free_from = 0; // don't let labels overlap
    for (week, cell) in mondays.iter().enumerate() {
        let Cell::Day { date, .. } = cell else {
            continue;
        };
        if last_month != Some(date.month()) {
            last_month = Some(date.month());
            // A month that began before the grid would crowd out the next
            // label, so only name it if the grid starts in its first week.
            if week == 0 && date.day() > 7 {
                continue;
            }
            let name = date.format("%b").to_string();
            if week >= free_from && week + name.len() <= row.len() {
                for (i, c) in name.chars().enumerate() {
                    row[week + i] = c;
                }
                free_from = week + name.len() + 1;
            }
        }
    }
    row.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn history(entries: &[(&str, u32)]) -> BTreeMap<String, u32> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn streak_counts_back_from_today() {
        let days = history(&[("2026-10-20", 1), ("2026-10-21", 2), ("2026-10-22", 1)]);
        assert_eq!(current_streak(&days, date(2026, 10, 22)), 3);
    }

    #[test]
    fn yesterdays_streak_holds_until_you_sit_today() {
        let days = history(&[("2026-10-20", 1), ("2026-10-21", 1)]);
        assert_eq!(current_streak(&days, date(2026, 10, 22)), 2);
        assert_eq!(current_streak(&days, date(2026, 10, 23)), 0);
    }

    #[test]
    fn longest_streak_finds_the_best_run() {
        let days = history(&[
            ("2026-01-01", 1),
            ("2026-01-02", 1),
            ("2026-01-03", 1),
            ("2026-02-10", 1),
            ("2026-02-11", 0),
        ]);
        assert_eq!(longest_streak(&days), 3);
        assert_eq!(longest_streak(&BTreeMap::new()), 0);
    }

    #[test]
    fn heatmap_ends_with_this_week() {
        // 2026-10-22 is a Thursday.
        let today = date(2026, 10, 22);
        let days = history(&[("2026-10-22", 2), ("2026-10-19", 9)]);
        let grid = heatmap(&days, today, 52);
        assert_eq!(grid.len(), 7);
        assert!(grid.iter().all(|row| row.len() == 52));

        let this_week: Vec<Cell> = grid.iter().map(|row| row[51]).collect();
        assert_eq!(
            this_week[0],
            Cell::Day {
                date: date(2026, 10, 19),
                sessions: 9
            }
        );
        assert_eq!(
            this_week[3],
            Cell::Day {
                date: today,
                sessions: 2
            }
        );
        assert_eq!(this_week[4], Cell::Future);
        assert_eq!(this_week[0].glyph(), '█'); // capped at 4+
        assert_eq!(this_week[3].glyph(), '▒');
        assert_eq!(this_week[1].glyph(), EMPTY);
        assert_eq!(this_week[6].glyph(), ' ');

        // The first column is 51 weeks before this Monday.
        let Cell::Day { date: first, .. } = grid[0][0] else {
            panic!("first cell should be a past day");
        };
        assert_eq!(first, date(2026, 10, 19) - Days::new(7 * 51));
    }

    #[test]
    fn month_labels_mark_where_months_start() {
        let grid = heatmap(&BTreeMap::new(), date(2026, 10, 22), 10);
        let labels = month_labels(&grid);
        assert_eq!(labels.chars().count(), 10);
        // The grid starts on Mon 17 Aug, part-way through August.
        assert_eq!(labels, "   Sep Oct");
    }
}
