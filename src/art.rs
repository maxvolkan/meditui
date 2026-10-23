//! ASCII art: the title banner, pictures, big clock digits and the breathing
//! circle. Everything here returns plain strings or lines; ui.rs places them.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

// "meditui" in figlet's standard font, one letter per array so the banner can
// be built with even spacing.
const BANNER_LETTERS: [[&str; 5]; 7] = [
    [
        "           ",
        " _ __ ___  ",
        r"| '_ ` _ \ ",
        "| | | | | |",
        "|_| |_| |_|",
    ],
    ["      ", "  ___ ", r" / _ \", "|  __/", r" \___|"],
    ["     _ ", "  __| |", " / _` |", "| (_| |", r" \__,_|"],
    [" _ ", "(_)", "| |", "| |", "|_|"],
    [" _   ", "| |_ ", "| __|", "| |_ ", r" \__|"],
    ["       ", " _   _ ", "| | | |", "| |_| |", r" \__,_|"],
    [" _ ", "(_)", "| |", "| |", "|_|"],
];

pub fn banner() -> Vec<String> {
    (0..5)
        .map(|row| {
            BANNER_LETTERS
                .iter()
                .map(|letter| letter[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub const LOTUS: [&str; 9] = [
    "             .             ",
    r"            / \            ",
    r"      .    /   \    .      ",
    r"     / \  |     |  / \     ",
    r"    |   \ |     | /   |    ",
    r" .--'.   \ \   / /   .'--. ",
    r"  '.  '-. \ \ / / .-'  .'  ",
    r"    '-.__'-\ V /-'__.-'    ",
    "  ~~~~~~~~~~'-'~~~~~~~~~~  ",
];

// Half blocks and box lines, which terminals draw themselves, so the sun
// keeps its shape in any font.
pub const SUNRISE: [&str; 3] = [
    "       ▄█████▄       ",
    "      █████████      ",
    "━━━━━━━━━━━━━━━━━━━━━",
];

// Big digits, drawn on a 3x5 grid and squeezed into 3 rows with half blocks.
const DIGITS: [[&str; 5]; 10] = [
    ["###", "# #", "# #", "# #", "###"],
    ["## ", " # ", " # ", " # ", "###"],
    ["###", "  #", "###", "#  ", "###"],
    ["###", "  #", "###", "  #", "###"],
    ["# #", "# #", "###", "  #", "  #"],
    ["###", "#  ", "###", "  #", "###"],
    ["###", "#  ", "###", "# #", "###"],
    ["###", "  #", "  #", "  #", "  #"],
    ["###", "# #", "###", "# #", "###"],
    ["###", "# #", "###", "  #", "###"],
];
const COLON: [&str; 5] = [" ", "#", " ", "#", " "];

/// "04:12" as three rows of big block characters.
pub fn big_clock(text: &str) -> Vec<String> {
    let glyphs: Vec<&[&str; 5]> = text
        .chars()
        .filter_map(|c| match c {
            '0'..='9' => Some(&DIGITS[c as usize - '0' as usize]),
            ':' => Some(&COLON),
            _ => None,
        })
        .collect();

    let grid: Vec<Vec<char>> = (0..5)
        .map(|row| {
            glyphs
                .iter()
                .map(|g| g[row])
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .collect()
        })
        .collect();

    // Pair up rows (0+1, 2+3, 4+blank) into half block characters.
    let width = grid[0].len();
    (0..3)
        .map(|pair| {
            (0..width)
                .map(|x| {
                    let top = grid[pair * 2][x] == '#';
                    let bottom = grid.get(pair * 2 + 1).is_some_and(|r| r[x] == '#');
                    match (top, bottom) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        (false, false) => ' ',
                    }
                })
                .collect()
        })
        .collect()
}

/// The circle's radius on a full breath, leaving room around the edge for
/// the glow.
fn full_radius(width: u16, height: u16) -> f64 {
    (height as f64 / 2.0 - 1.5)
        .min(width as f64 / 4.0 - 2.0)
        .max(1.0)
}

/// How many columns a full circle reaches out from the middle.
pub fn circle_reach(width: u16, height: u16) -> u16 {
    (2.0 * full_radius(width, height)).ceil() as u16
}

/// The breathing circle. At `size` 1.0 it fills `width` x `height`; at 0.0
/// it's a small dot. Cells are about twice as tall as they are wide, so x
/// distances are halved.
pub fn breath_circle(size: f64, width: u16, height: u16) -> Vec<Line<'static>> {
    let size = size.clamp(0.0, 1.0);
    let (w, h) = (width as i32, height as i32);
    let max_r = full_radius(width, height);
    let min_r = (max_r * 0.18).max(1.0);
    let r = min_r + (max_r - min_r) * size;
    let (cx, cy) = ((w - 1) as f64 / 2.0, (h - 1) as f64 / 2.0);

    let edge = Style::new().fg(Color::LightGreen).bold();
    let band = Style::new().fg(Color::Green);
    let core = Style::new().fg(Color::DarkGray);
    let glow = Style::new().fg(Color::DarkGray);
    let glow_width = 1.6 * size;

    (0..h)
        .map(|y| {
            // Group runs of the same kind of cell into one span each.
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut run = String::new();
            let mut run_style = Style::new();
            for x in 0..w {
                let dx = (x as f64 - cx) / 2.0;
                let dy = y as f64 - cy;
                let from_edge = (dx * dx + dy * dy).sqrt() - r; // negative inside
                let (ch, style) = if from_edge.abs() < 0.5 {
                    ('•', edge)
                } else if from_edge < 0.0 && from_edge > -1.8 {
                    ('·', band)
                } else if from_edge < 0.0 {
                    ('·', core)
                } else if from_edge < 0.5 + glow_width && (x + y) % 2 == 0 {
                    ('·', glow)
                } else {
                    (' ', Style::new())
                };
                if style != run_style && !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), run_style));
                }
                run_style = style;
                run.push(ch);
            }
            spans.push(Span::styled(run, run_style));
            Line::from(spans)
        })
        .collect()
}

/// A still point for gazing practice, with a faint aura that slowly
/// brightens and dims.
pub fn gaze_point(seconds: f64, width: u16, height: u16) -> Vec<Line<'static>> {
    let (w, h) = (width as i32, height as i32);
    let (cx, cy) = (w / 2, h / 2);
    // One slow swell every 8 seconds, only in the outer aura.
    let swell = 0.5 - 0.5 * (std::f64::consts::TAU * seconds / 8.0).cos();
    let aura = Style::new().fg(if swell > 0.5 {
        Color::Gray
    } else {
        Color::DarkGray
    });
    let halo = Style::new().fg(Color::DarkGray);
    let point = Style::new().fg(Color::Yellow).bold();

    (0..h)
        .map(|y| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for x in 0..w {
                let dx = (x - cx) as f64 / 2.0;
                let dy = (y - cy) as f64;
                let d = (dx * dx + dy * dy).sqrt();
                let (ch, style) = if x == cx && y == cy {
                    ('•', point)
                } else if (d - 1.5).abs() < 0.5 && (x + y) % 2 == 0 {
                    ('·', aura)
                } else if (d - 3.0).abs() < 0.4 && (x + y) % 2 == 0 {
                    ('·', halo)
                } else {
                    (' ', Style::new())
                };
                spans.push(Span::styled(ch.to_string(), style));
            }
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every character the art uses is one column wide, so counting chars
    // gives the on-screen width.
    fn width(s: &str) -> usize {
        s.chars().count()
    }

    #[test]
    fn banner_rows_line_up() {
        let rows = banner();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|r| width(r) == width(&rows[0])));
    }

    #[test]
    fn pictures_have_even_rows() {
        assert!(LOTUS.iter().all(|r| width(r) == width(LOTUS[0])));
        assert!(SUNRISE.iter().all(|r| width(r) == width(SUNRISE[0])));
    }

    #[test]
    fn big_clock_is_three_rows_of_equal_width() {
        let rows = big_clock("04:12");
        assert_eq!(rows.len(), 3);
        // 4 digits * 3 + colon 1 + 4 gaps = 17
        assert!(rows.iter().all(|r| width(r) == 17));
    }

    #[test]
    fn circle_fits_its_area() {
        for size in [0.0, 0.5, 1.0] {
            let lines = breath_circle(size, 40, 12);
            assert_eq!(lines.len(), 12);
            assert!(lines.iter().all(|l| l.width() == 40));
        }
    }

    #[test]
    fn gaze_point_stays_in_the_middle() {
        for seconds in [0.0, 2.0, 4.0, 7.5] {
            let lines = gaze_point(seconds, 41, 11);
            assert_eq!(lines.len(), 11);
            assert!(lines.iter().all(|l| l.width() == 41));
            let middle: String = lines[5].spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(middle.chars().nth(20), Some('•'));
        }
    }

    #[test]
    fn circle_is_bigger_on_the_in_breath() {
        let filled = |size| {
            breath_circle(size, 60, 17)
                .iter()
                .flat_map(|line| line.spans.iter())
                .map(|span| span.content.chars().filter(|c| *c != ' ').count())
                .sum::<usize>()
        };
        assert!(filled(0.0) < filled(0.5));
        assert!(filled(0.5) < filled(1.0));
    }
}
