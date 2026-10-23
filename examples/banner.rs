//! The animated banner at the top of the README: the logo next to the
//! breathing circle, drawn with the app's own art. Recorded with
//! `vhs assets/banner.tape`. Any key quits.

#[allow(dead_code)]
#[path = "../src/art.rs"]
mod art;

use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    crossterm::event::{self, Event},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::Paragraph,
};

/// One breath in and out, in seconds. The recording is exactly this long,
/// so the GIF loops without a jump.
const BREATH: f64 = 8.0;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let start = Instant::now();
    let result = loop {
        let seconds = start.elapsed().as_secs_f64();
        if let Err(err) = terminal.draw(|frame| draw(frame, seconds)) {
            break Err(err);
        }
        match event::poll(Duration::from_millis(40)) {
            Ok(true) if matches!(event::read(), Ok(Event::Key(_))) => break Ok(()),
            Ok(_) => {}
            Err(err) => break Err(err),
        }
    };
    ratatui::restore();
    result
}

fn draw(frame: &mut Frame, seconds: f64) {
    // The app's easing: in for half the breath, out for the other half.
    let size = 0.5 - 0.5 * (TAU * seconds / BREATH).cos();

    let logo = art::banner();
    let logo_width = logo[0].chars().count() as u16;
    let area = frame.area();
    let height = area.height.min(17);
    // Wide enough that the circle's height, not its width, sets its size.
    let circle_width = height * 2 + 4;
    let [circle_area, _, logo_area] = Layout::horizontal([
        Constraint::Length(circle_width),
        Constraint::Length(4),
        Constraint::Length(logo_width),
    ])
    .areas(area.centered(
        Constraint::Length(circle_width + 4 + logo_width),
        Constraint::Length(height),
    ));

    frame.render_widget(
        Paragraph::new(art::breath_circle(
            size,
            circle_area.width,
            circle_area.height,
        )),
        circle_area,
    );

    let mut lines: Vec<Line> = logo
        .into_iter()
        .map(|row| Line::from(row).fg(Color::Green).bold())
        .collect();
    lines.push(Line::from(""));
    lines.push(
        Line::from("a quiet place in your terminal")
            .centered()
            .style(Style::new().fg(Color::DarkGray).italic()),
    );
    let text_height = lines.len() as u16;
    frame.render_widget(
        Paragraph::new(lines),
        Rect {
            y: logo_area.y + logo_area.height.saturating_sub(text_height) / 2,
            height: text_height,
            ..logo_area
        },
    );
}
