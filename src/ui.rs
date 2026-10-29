//! Drawing every screen. Nothing in here changes app state except the list
//! scroll position, which ratatui keeps in `list_state`.

use std::path::Path;
use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Clear, HighlightSpacing, LineGauge, List, ListItem, Padding, Paragraph,
        Wrap,
    },
};

use crate::app::{
    App, Folder, FormStep, MAX_NAME_LEN, POMO_BREAK, POMO_LENGTHS, POMO_MEDITATION, Row as ListRow,
    SETTING_BACKGROUND, SETTING_CLOCK, SETTING_PATH, SETTING_PREFLIGHT, SETTING_REPLAY,
    SETTING_SOUND, SETTING_TONES, SETTING_TOUR, SETTING_VOLUME, SETTING_WALKTHROUGH, Screen,
    Summary,
};
use crate::art;
use crate::library::{self, BreathState, Guide, MIN_TONE_SECONDS, Meditation, Side, Stage, Tag};
use crate::pomodoro::{BreakMode, Phase};
use crate::progress::{self, Cell};
use crate::session::{self, Session};
use crate::storage::{self, ClockStyle};
use crate::tutorial::{self, Row};

const ACCENT: Color = Color::Green;
const MUTED: Color = Color::DarkGray;
const WARN: Color = Color::Yellow;

const MIN_WIDTH: u16 = 44;
const MIN_HEIGHT: u16 = 16;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        too_small(frame, area);
        return;
    }

    match app.screen {
        Screen::Tutorial { page } => tutorial(frame, area, app, page),
        Screen::Session { .. } => session(frame, area, app),
        Screen::Walkthrough { .. } => walkthrough(frame, area, app),
        Screen::Done(_) => done(frame, area, app),
        Screen::Settings { selected } => settings(frame, area, app, selected),
        Screen::Preflight { .. } => preflight(frame, area, app),
        Screen::Stats => stats(frame, area, app),
        Screen::PomodoroSetup { selected } => pomodoro_setup(frame, area, app, selected),
        Screen::Pomodoro => pomodoro(frame, area, app),
        Screen::Library
        | Screen::ConfirmStart { .. }
        | Screen::Locked
        | Screen::ConfirmDelete
        | Screen::AddForm { .. } => {
            library(frame, area, app);
            match app.screen {
                Screen::ConfirmStart { minutes } => confirm_start(frame, area, app, minutes),
                Screen::Locked => locked(frame, area, app),
                Screen::ConfirmDelete => confirm_delete(frame, area, app),
                Screen::AddForm { step } => add_form(frame, area, app, step),
                _ => {}
            }
        }
    }
}

// ----- Shared bits -----

/// The rounded green frame every screen sits in, with key hints at the bottom.
fn frame_block<'a>(title: &str, hints: &str) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(
            Line::from(format!(" {title} "))
                .centered()
                .fg(ACCENT)
                .bold(),
        )
        .title_bottom(Line::from(format!(" {hints} ")).centered().fg(MUTED))
}

fn popup_block<'a>(title: &str, hints: &str) -> Block<'a> {
    frame_block(title, hints).padding(Padding::horizontal(2))
}

/// A `width` x `height` box in the middle of `area` (shrunk to fit).
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    area.centered(
        Constraint::Length(width.min(area.width)),
        Constraint::Length(height.min(area.height)),
    )
}

fn art_lines<'a, S: AsRef<str>>(rows: &[S], style: Style) -> Vec<Line<'a>> {
    rows.iter()
        .map(|row| Line::styled(row.as_ref().to_string(), style).centered())
        .collect()
}

fn too_small(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new("Make the terminal a little bigger\nto make room for meditui.")
        .centered()
        .wrap(Wrap { trim: true });
    frame.render_widget(text, centered(area, area.width, 2));
}

fn stats_line(app: &App) -> String {
    let stats = &app.data.stats;
    if stats.sessions == 0 && stats.seconds == 0 {
        return "No sessions yet.".to_string();
    }
    let plural = if stats.sessions == 1 { "" } else { "s" };
    format!(
        "{} session{plural} · {} sat",
        stats.sessions,
        session::format_total(stats.seconds)
    )
}

// ----- Library -----

fn library(frame: &mut Frame, area: Rect, app: &mut App) {
    let hints = if area.width >= 138 {
        "Enter start · i walkthrough · ←→ folders · Tab next folder · o office · a add · d delete · p pomodoro · t stats · s settings · q quit"
    } else if area.width >= 92 {
        "Enter start · ←→ folders · o office · a add · p pomodoro · t stats · s settings · q quit"
    } else if area.width >= 74 {
        "Enter start · ←→ folders · p pomodoro · t stats · s settings · q quit"
    } else {
        "Enter start · s settings · q quit"
    };
    let outer = frame_block("meditui", hints);
    let inner = outer.inner(area).inner(ratatui::layout::Margin::new(1, 0));
    frame.render_widget(outer, area);

    let [head, body, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(inner);

    let tagline = Line::from("a quiet place in your terminal")
        .centered()
        .fg(MUTED)
        .italic();
    frame.render_widget(Paragraph::new(tagline), head);

    // List on the left, details on the right (when there's room).
    let (list_area, detail_area) = if body.width >= 72 {
        let [left, right] = Layout::horizontal([Constraint::Percentage(50), Constraint::Fill(1)])
            .spacing(1)
            .areas(body);
        (left, Some(right))
    } else {
        (body, None)
    };

    let mut title = vec![Span::styled(" Library ", Style::new().fg(ACCENT))];
    if app.office_only {
        title.push(Span::styled("· office-safe ", Style::new().fg(WARN)));
    }
    let mut list_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(MUTED))
        .title(Line::from(title));
    if app.office_only {
        list_block =
            list_block.title_bottom(Line::from(" o to show everything ").centered().fg(MUTED));
    } else if !app.has_custom() {
        list_block = list_block.title_bottom(
            Line::from(" none of your own yet · a to add ")
                .centered()
                .fg(MUTED),
        );
    }

    // Folder headers, then each meditation: indent, marker, name padded to
    // fill, tag letters, then the length.
    let row_width = list_block.inner(list_area).width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .visible
        .iter()
        .map(|&row| {
            let index = match row {
                ListRow::Folder(folder) => return folder_header(app, folder),
                ListRow::Item(index) => index,
            };
            let m = &app.library[index];
            let locked = app.lock_progress(m).is_some();
            let marker = if m.custom {
                "★ "
            } else if m.recipe {
                "◆ "
            } else if locked {
                "○ "
            } else {
                "  "
            };
            let minutes = format!("{:>3} min", m.minutes);
            let name_width = row_width.saturating_sub(2 + 2 + 3 + minutes.len() + 1);
            let name = truncate(&m.name, name_width);
            let pad = " ".repeat(name_width.saturating_sub(name.chars().count()));
            let name_style = if locked {
                Style::new().fg(MUTED)
            } else {
                Style::new()
            };
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(marker, Style::new().fg(if locked { MUTED } else { ACCENT })),
                Span::styled(format!("{name}{pad} "), name_style),
            ];
            // Up to two tag letters, with home-only in yellow.
            for slot in 0..2 {
                spans.push(match m.tags.get(slot) {
                    Some(Tag::HomeOnly) => Span::styled("H", Style::new().fg(WARN)),
                    Some(tag) => Span::styled(tag.letter().to_string(), Style::new().fg(MUTED)),
                    None => Span::raw(" "),
                });
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled(minutes, Style::new().fg(MUTED)));
            ListItem::new(Line::from(spans))
        })
        .collect();

    if items.is_empty() {
        let inner_list = list_block.inner(list_area);
        frame.render_widget(list_block, list_area);
        frame.render_widget(
            Paragraph::new("Nothing matches this filter.")
                .centered()
                .fg(MUTED),
            centered(inner_list, inner_list.width, 1),
        );
        if let Some(detail_area) = detail_area {
            frame.render_widget(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(MUTED)),
                detail_area,
            );
        }
        library_footer(frame, foot, app);
        return;
    }

    let list = List::new(items)
        .block(list_block)
        .highlight_symbol("› ")
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(Style::new().fg(ACCENT).bold());
    frame.render_stateful_widget(list, list_area, &mut app.list_state);

    if let Some(detail_area) = detail_area {
        details(frame, detail_area, app);
    }
    library_footer(frame, foot, app);
}

/// "▾ Down-regulation  11", or ▸ when the folder is closed.
fn folder_header(app: &App, folder: Folder) -> ListItem<'static> {
    let arrow = if app.is_open(folder) { "▾" } else { "▸" };
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{arrow} {}", folder.label()),
            Style::new().fg(ACCENT).bold(),
        ),
        Span::styled(
            format!("  {}", app.folder_count(folder)),
            Style::new().fg(MUTED),
        ),
    ]))
}

/// A status message if there is one, otherwise your stats.
fn library_footer(frame: &mut Frame, area: Rect, app: &App) {
    let footer = match &app.status {
        Some(status) => Line::from(status.clone()).fg(WARN),
        None => Line::from(stats_line(app)).fg(MUTED),
    };
    frame.render_widget(Paragraph::new(footer.centered()), area);
}

/// A path for display: the home folder becomes `~`, and if it's still too
/// wide the middle is cut out, e.g. `~/.local/.../data.json`.
fn short_path(path: &Path, home: Option<&Path>, max_width: usize) -> String {
    let full = match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    };
    if full.chars().count() <= max_width {
        return full;
    }
    let parts: Vec<&str> = full.split('/').collect();
    if let [first, second, .., last] = parts.as_slice() {
        for short in [
            format!("{first}/{second}/.../{last}"),
            format!(".../{last}"),
        ] {
            if short.chars().count() <= max_width {
                return short;
            }
        }
    }
    truncate(&full, max_width)
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut short: String = text.chars().take(width.saturating_sub(1)).collect();
    short.push('…');
    short
}

fn details(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ListRow::Folder(folder)) = app.selected_row() {
        folder_details(frame, area, app, folder);
        return;
    }
    let Some(m) = app.selected() else {
        return;
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(MUTED))
        .title(Line::from(format!(" {} ", m.name)).fg(ACCENT).bold())
        .padding(Padding::new(2, 2, 1, 0));

    let mut lines = Vec::new();
    if let Some(category) = m.category {
        lines.push(Line::from(category.label().to_uppercase()).fg(MUTED));
    }
    lines.push(Line::from(vec![
        Span::styled(m.guide.kind(), Style::new().fg(ACCENT)),
        Span::styled(format!(" · {} min", m.minutes), Style::new().fg(MUTED)),
    ]));
    if let Some(pattern) = m.guide.pattern_summary() {
        lines.push(Line::from(pattern).fg(MUTED));
    }
    if !m.tags.is_empty() {
        let badges: Vec<Span> = m
            .tags
            .iter()
            .map(|tag| Span::styled(format!("[{}] ", tag.label()), Style::new().fg(MUTED)))
            .collect();
        lines.push(Line::from(badges));
    }
    if m.recipe {
        lines.push(Line::from("◆ from your recipes folder").fg(MUTED));
    }
    if let Some(minutes) = m.interval_bell {
        lines.push(Line::from(format!("A chime every {minutes} min")).fg(MUTED));
    }
    let locked = app.lock_progress(m);
    if let Some((rule, days)) = locked {
        lines.push(
            Line::from(format!(
                "○ Comes after {} · {days} of {} days",
                rule.after, rule.days
            ))
            .fg(WARN),
        );
    }
    if let Some(when) = m.when {
        lines.push(Line::from(""));
        lines.push(Line::from(when).fg(ACCENT).italic());
    }
    lines.push(Line::from(""));
    lines.push(Line::from(m.description.clone()));
    lines.push(Line::from(""));
    let mut begin = if locked.is_some() {
        "Enter to begin (it'll ask first)"
    } else {
        "Enter to begin"
    }
    .to_string();
    if m.has_walkthrough() {
        begin.push_str(" · i walkthrough");
    }
    lines.push(Line::from(begin).fg(MUTED).italic());

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

/// The details panel when a folder header is selected.
fn folder_details(frame: &mut Frame, area: Rect, app: &App, folder: Folder) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(MUTED))
        .title(
            Line::from(format!(" {} ", folder.label()))
                .fg(ACCENT)
                .bold(),
        )
        .padding(Padding::new(2, 2, 1, 0));
    let inner = block.inner(area);
    let width = inner.width as usize;
    let count = app.folder_count(folder);
    let plural = if count == 1 { "" } else { "s" };
    let action = if app.is_open(folder) {
        "Enter or ← to close"
    } else {
        "Enter or → to open"
    };
    let mut lines = vec![
        Line::from(truncate(
            &format!("{count} meditation{plural} · {action}"),
            width,
        ))
        .fg(MUTED),
        Line::from(""),
    ];
    lines.extend(
        wrap_words(folder.blurb(), width)
            .into_iter()
            .map(Line::from),
    );
    lines.push(Line::from(""));

    // Which one to pick when, like a menu: two lines each if they fit, one
    // if not, and as many names as fit after that.
    let items: Vec<&Meditation> = app.in_folder(folder).collect();
    let room = (inner.height as usize).saturating_sub(lines.len());
    let when = |m: &Meditation| m.when.unwrap_or_default();
    if items.len() * 2 <= room {
        for m in &items {
            lines.push(Line::from(truncate(&m.name, width)).fg(ACCENT));
            lines.push(Line::from(truncate(&format!("  {}", when(m)), width)));
        }
    } else if items.len() <= room {
        for m in &items {
            let rest = width.saturating_sub(m.name.chars().count() + 3);
            lines.push(Line::from(vec![
                Span::styled(m.name.clone(), Style::new().fg(ACCENT)),
                Span::styled(" · ", Style::new().fg(MUTED)),
                Span::raw(truncate(when(m), rest)),
            ]));
        }
    } else {
        let shown = room.saturating_sub(1);
        for m in items.iter().take(shown) {
            lines.push(Line::from(truncate(&m.name, width)).fg(ACCENT));
        }
        lines.push(Line::from(format!("+{} more", items.len() - shown)).fg(MUTED));
    }
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// "This one usually comes later": start anyway, unlock for good, or back.
fn locked(frame: &mut Frame, area: Rect, app: &App) {
    let Some(m) = app.selected() else {
        return;
    };
    let Some((rule, days)) = app.lock_progress(m) else {
        return;
    };
    let popup = centered(area, 54, 10);
    let lines = vec![
        Line::from(format!(
            "This usually comes after {} days of {}.",
            rule.days, rule.after
        )),
        Line::from(""),
        Line::from(format!("You're at {days} so far.")).fg(MUTED),
        Line::from(""),
        Line::from(
            "You can start anyway, or unlock it for good if you already know the \
             earlier practice.",
        )
        .fg(MUTED),
    ];
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(popup_block(&m.name, "y start anyway · u unlock · n back"))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

/// One step at a time: the setup steps, one practice round with the circle,
/// then a last word before the session.
fn walkthrough(frame: &mut Frame, area: Rect, app: &App) {
    let (
        Screen::Walkthrough {
            step,
            round_since,
            replay,
            ..
        },
        Some(m),
    ) = (&app.screen, app.selected())
    else {
        return;
    };
    let stage = m.walkthrough_stage(*step);
    let hints = match stage {
        Stage::Setup(_) => "Enter next · s skip · Esc back",
        Stage::Round => "Enter skip ahead · Esc back",
        Stage::Done if *replay => "Enter choose a length · Esc back",
        Stage::Done => "Enter begin · Esc back",
    };
    let outer = frame_block(&format!("First time · {}", m.name), hints);
    let inner = outer.inner(area).inner(ratatui::layout::Margin::new(2, 0));
    frame.render_widget(outer, area);

    let [visual, text_area, counter_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(inner);

    let round = match (&m.guide, round_since) {
        (Guide::Breath(phases), Some(since)) => {
            let breath = library::breath_at(&library::warm_up(phases), since.elapsed());
            draw_breath(frame, visual, &breath);
            Some(breath_cue(&breath))
        }
        _ => {
            resting_visual(frame, visual, m, app.uptime());
            None
        }
    };
    let text = match stage {
        Stage::Setup(text) => text.to_string(),
        Stage::Round => round.unwrap_or_default(),
        Stage::Done => walkthrough_done(m).to_string(),
    };
    frame.render_widget(
        Paragraph::new(Line::from(text).fg(ACCENT).bold())
            .centered()
            .wrap(Wrap { trim: true }),
        centered(text_area, 64, 3),
    );

    let counter = match stage {
        Stage::Setup(_) => format!("Step {} of {}", step + 1, m.walkthrough.len()),
        Stage::Round => "Slow at first, then the real pace".to_string(),
        Stage::Done => String::new(),
    };
    frame.render_widget(
        Paragraph::new(Line::from(counter).centered().fg(MUTED)),
        counter_area,
    );
}

fn walkthrough_done(m: &Meditation) -> &'static str {
    match (matches!(m.guide, Guide::Breath(_)), m.eyes_closed) {
        (true, true) => "Now close your eyes. The tones keep time.",
        (true, false) => "This is the real pace now. Keep following the circle.",
        (false, true) => "Close your eyes when you're ready.",
        (false, false) => "Start whenever you're ready.",
    }
}

fn confirm_start(frame: &mut Frame, area: Rect, app: &App, minutes: u64) {
    let Some(m) = app.selected() else {
        return;
    };
    let popup = centered(area, 46, 9);
    let lines = vec![
        Line::from(m.guide.kind()).centered().fg(MUTED),
        Line::from(""),
        Line::from(vec![
            Span::styled("<   ", Style::new().fg(MUTED)),
            Span::styled(format!("{minutes} min"), Style::new().bold()),
            Span::styled("   >", Style::new().fg(MUTED)),
        ])
        .centered(),
        Line::from(""),
        Line::from("← → 1 minute   ↑ ↓ 5 minutes")
            .centered()
            .fg(MUTED),
    ];
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(popup_block(&m.name, "Enter begin · Esc back")),
        popup,
    );
}

fn confirm_delete(frame: &mut Frame, area: Rect, app: &App) {
    let Some(m) = app.selected() else {
        return;
    };
    let popup = centered(area, 46, 7);
    let lines = vec![
        Line::from(format!("Delete \"{}\"?", m.name)).centered(),
        Line::from(""),
        Line::from("This can't be undone.").centered().fg(MUTED),
    ];
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(popup_block("Delete", "y delete · n keep"))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn add_form(frame: &mut Frame, area: Rect, app: &App, step: FormStep) {
    let popup = centered(area, 56, 11);
    let on_name = step == FormStep::Name;

    // The field you're typing in gets a › and a cursor; the other is dim.
    let field = |label: &str, value: String, active: bool| {
        if active {
            Line::from(vec![
                Span::styled(format!("› {label}"), Style::new().fg(ACCENT).bold()),
                Span::raw(value),
                Span::styled("▏", Style::new().fg(ACCENT)),
            ])
        } else {
            Line::from(format!("  {label}{value}")).fg(MUTED)
        }
    };

    let mut lines = vec![
        field("Name     ", app.name_input.clone(), on_name),
        Line::from(""),
        field("Minutes  ", app.minutes_input.clone(), !on_name),
        Line::from(""),
    ];
    match &app.form_error {
        Some(error) => lines.push(Line::from(error.clone()).fg(WARN)),
        None if on_name => {
            lines.push(Line::from(format!("Up to {MAX_NAME_LEN} characters.")).fg(MUTED))
        }
        None => lines.push(Line::from("A quiet sit with the breathing circle.").fg(MUTED)),
    }

    let hints = if on_name {
        "Enter next · Esc cancel"
    } else {
        "Enter add · Esc back"
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(popup_block("New meditation", hints).padding(Padding::new(2, 2, 1, 0))),
        popup,
    );
}

// ----- Prep checklist -----

fn preflight(frame: &mut Frame, area: Rect, app: &App) {
    let (
        Screen::Preflight {
            checked, selected, ..
        },
        Some(meditation),
    ) = (&app.screen, app.selected())
    else {
        return;
    };
    let outer = frame_block(
        &meditation.name,
        "Space tick · Enter begin · s skip · Esc back",
    );
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let width = inner.width.saturating_sub(4).min(64);
    let mut lines = vec![
        Line::from("Before you begin").centered().fg(ACCENT).bold(),
        Line::from(""),
    ];
    for (i, (title, detail)) in library::prep_steps(meditation).iter().enumerate() {
        let here = i == *selected;
        let tick = if checked.get(i).copied().unwrap_or(false) {
            "[x]"
        } else {
            "[ ]"
        };
        let pointer = if here { "› " } else { "  " };
        let title_style = if here {
            Style::new().fg(ACCENT).bold()
        } else {
            Style::new().bold()
        };
        lines.push(Line::from(vec![
            Span::styled(pointer, Style::new().fg(ACCENT)),
            Span::styled(format!("{tick} "), Style::new().fg(ACCENT)),
            Span::styled(*title, title_style),
        ]));
        for row in wrap_words(detail, width.saturating_sub(6) as usize) {
            lines.push(Line::from(format!("      {row}")).fg(MUTED));
        }
        lines.push(Line::from(""));
    }
    let all_done = !checked.is_empty() && checked.iter().all(|c| *c);
    lines.push(
        Line::from(if all_done {
            "All set. Enter to begin."
        } else {
            "Tick what you've done, or just begin."
        })
        .centered()
        .fg(if all_done { ACCENT } else { MUTED }),
    );

    let content = centered(inner, width, lines.len() as u16);
    // Title and footer are centered across the whole column; the steps
    // read left to right inside it.
    frame.render_widget(Paragraph::new(lines), content);
}

/// Splits `text` into lines of at most `width` characters, breaking at spaces.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        if !row.is_empty() && row.chars().count() + 1 + word.chars().count() > width {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

// ----- Tour -----

fn tutorial(frame: &mut Frame, area: Rect, app: &App, page: usize) {
    let total = tutorial::PAGES.len();
    let hints = if page == tutorial::LAST_PAGE {
        "Enter begin · ← back · Esc skip"
    } else {
        "→ next · ← back · Esc skip the tour"
    };
    let outer = frame_block(&format!("Tour {}/{}", page + 1, total), hints);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let the_page = &tutorial::PAGES[page];
    let mut lines: Vec<Line> = Vec::new();

    // Pictures: banner + lotus on the welcome page, a live circle on the last.
    if page == 0 {
        if inner.width >= 52 && inner.height >= 26 {
            lines.extend(art_lines(&art::banner(), Style::new().fg(ACCENT)));
            lines.push(Line::from(""));
        }
        if inner.height >= 20 {
            lines.extend(art_lines(&art::LOTUS, Style::new().fg(ACCENT)));
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(the_page.title).centered().fg(ACCENT).bold());
    lines.push(Line::from(""));

    // Key rows line up in two columns.
    let key_width = the_page
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Key(key, _) => Some(key.chars().count()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let body: Vec<Line> = the_page
        .rows
        .iter()
        .map(|row| match row {
            Row::Text(text) => Line::from(*text),
            Row::Key(key, meaning) => Line::from(vec![
                Span::styled(format!("{key:<key_width$}"), Style::new().fg(ACCENT).bold()),
                Span::raw(format!("   {meaning}")),
            ]),
            Row::Blank => Line::from(""),
        })
        .collect();
    let body_width = body.iter().map(Line::width).max().unwrap_or(0);
    // Indent the body as a block so it's centered but stays left-aligned.
    let indent = " ".repeat((inner.width as usize).saturating_sub(body_width) / 2);
    lines.extend(body.into_iter().map(|line| {
        let mut spans = vec![Span::raw(indent.clone())];
        spans.extend(line.spans);
        Line::from(spans)
    }));

    let mut circle_height = 0;
    if page == tutorial::LAST_PAGE {
        circle_height = (inner.height as usize)
            .saturating_sub(lines.len() + 4)
            .min(11) as u16;
    }

    // Progress dots: · · • · · · ·
    let dots: String = (0..total)
        .map(|i| if i == page { "•" } else { "·" })
        .collect::<Vec<_>>()
        .join(" ");

    let content_height = lines.len() as u16 + circle_height + 2;
    let content = centered(inner, inner.width, content_height);
    let [circle_area, text_area, dots_area] = Layout::vertical([
        Constraint::Length(circle_height),
        Constraint::Length(lines.len() as u16),
        Constraint::Length(2),
    ])
    .areas(content);

    if circle_height >= 5 {
        // A preview of the breathing circle, on the same rhythm as the try.
        let preview = library::presets()
            .into_iter()
            .find(|m| m.name == library::TUTORIAL_PRESET)
            .map(|m| m.guide);
        if let Some(Guide::Breath(phases)) = preview {
            let state = library::breath_at(&phases, app.uptime());
            let width = circle_area.width.min(circle_height * 4);
            let area = centered(circle_area, width, circle_height);
            frame.render_widget(
                Paragraph::new(art::breath_circle(state.size, area.width, area.height)),
                area,
            );
        }
    }
    frame.render_widget(Paragraph::new(lines), text_area);
    frame.render_widget(
        Paragraph::new(vec![Line::from(""), Line::from(dots).centered().fg(MUTED)]),
        dots_area,
    );
}

// ----- Session -----

fn session(frame: &mut Frame, area: Rect, app: &App) {
    let Screen::Session { session, ending } = &app.screen else {
        return;
    };
    let ending = *ending;
    let paused = session.is_paused();

    let minimal = session.dim;
    let hints = match (ending, paused, minimal) {
        (true, _, _) => "Esc end session · Space keep going",
        (false, true, _) => "Space resume · Esc end",
        (false, false, true) => "h show the circle · Space pause · Esc end",
        (false, false, false) => "h dim · Space pause · Esc end",
    };
    let outer = frame_block(&session.meditation.name, hints);
    let inner = outer.inner(area).inner(ratatui::layout::Margin::new(2, 0));
    frame.render_widget(outer, area);

    let clock = app.data.settings.clock;
    if minimal {
        minimal_session(frame, inner, session, ending, paused, clock);
        return;
    }

    let [visual, cue_area, progress_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(inner);

    let (cue, cue_style) = session_visual(frame, visual, session);
    let (cue, cue_style) = if ending {
        (
            "End this session now?".to_string(),
            Style::new().fg(WARN).bold(),
        )
    } else if paused {
        ("Paused".to_string(), Style::new().fg(WARN).bold())
    } else {
        (cue, cue_style)
    };
    frame.render_widget(
        Paragraph::new(Line::styled(cue, cue_style))
            .centered()
            .wrap(Wrap { trim: true }),
        centered(cue_area, 60, 3),
    );

    progress_line(frame, progress_area, session, paused, clock);
}

/// The time to show under a session, if any, like "12:34".
fn session_clock(session: &Session, style: ClockStyle) -> Option<String> {
    match style {
        ClockStyle::Remaining => Some(session::format_clock(session.remaining())),
        ClockStyle::Elapsed => Some(session::format_elapsed(session.elapsed())),
        ClockStyle::Hidden => None,
    }
}

/// A thin progress line with the time after it, dimmed so it stays out of
/// the way.
fn progress_line(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    paused: bool,
    style: ClockStyle,
) {
    let time = session_clock(session, style);
    let time_width = time.as_ref().map_or(0, |t| t.len() as u16 + 2);
    let gauge_width = (area.width * 2 / 3).max(10);
    let row = centered(area, gauge_width + time_width, 1);
    let [gauge_area, time_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(time_width)]).areas(row);
    frame.render_widget(
        LineGauge::default()
            .ratio(session.progress())
            .label("")
            .filled_style(Style::new().fg(if paused { MUTED } else { ACCENT }))
            .unfilled_style(Style::new().fg(MUTED)),
        gauge_area,
    );
    if let Some(time) = time {
        let color = if paused { WARN } else { MUTED };
        frame.render_widget(
            Paragraph::new(Line::from(time).right_aligned().fg(color)),
            time_area,
        );
    }
}

/// The dim, eyes-closed view: nothing moves, just what to do right now and
/// the progress line, in case you peek.
fn minimal_session(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    ending: bool,
    paused: bool,
    clock: ClockStyle,
) {
    let prompt = if ending {
        Some("End this session now?")
    } else if paused {
        Some("Paused")
    } else {
        None
    };
    let [cue_area, _, line_area, _, prompt_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(centered(area, area.width, 6));
    let (cue, _) = session_cue(session);
    frame.render_widget(
        Paragraph::new(Line::from(cue).fg(MUTED))
            .centered()
            .wrap(Wrap { trim: true }),
        centered(cue_area, 60, 2),
    );
    progress_line(frame, line_area, session, paused, clock);
    if let Some(prompt) = prompt {
        frame.render_widget(
            Paragraph::new(Line::from(prompt).centered().fg(WARN).bold()),
            prompt_area,
        );
    }
}

/// Draws the visual anchor and returns the cue text to show under it.
/// Breathing exercises pulse to their own pattern; gazing practice gets a
/// still point; everything else pulses to a calm resting rhythm, so there's
/// always something to breathe along with.
fn session_visual(frame: &mut Frame, area: Rect, session: &Session) -> (String, Style) {
    let elapsed = session.elapsed();
    match &session.meditation.guide {
        Guide::Breath(phases) => draw_breath(frame, area, &library::breath_at(phases, elapsed)),
        _ => resting_visual(frame, area, &session.meditation, elapsed),
    }
    session_cue(session)
}

/// What to do right now: the breath phase or the current cue.
fn session_cue(session: &Session) -> (String, Style) {
    let elapsed = session.elapsed();
    if let Guide::Breath(phases) = &session.meditation.guide {
        return (
            breath_cue(&library::breath_at(phases, elapsed)),
            Style::new().fg(ACCENT).bold(),
        );
    }
    match session.meditation.guide.cues() {
        Some(cues) => {
            let index = library::prompt_index(cues.len(), elapsed, session.total());
            let cue = cues.get(index).copied().unwrap_or_default();
            (cue.to_string(), Style::new().italic())
        }
        None => (
            "Just sit. Breathe along with the circle if it helps.".to_string(),
            Style::new().fg(MUTED).italic(),
        ),
    }
}

/// "Breathe in · 3", with no countdown for fast breathing.
fn breath_cue(breath: &BreathState) -> String {
    if breath.phase_seconds >= MIN_TONE_SECONDS {
        format!("{} · {}", breath.label, breath.seconds_left.ceil() as u64)
    } else {
        breath.label.to_string()
    }
}

/// Where the circle or gaze point sits in `area`.
fn visual_anchor(area: Rect) -> Rect {
    let height = area.height.min(19);
    centered(area, area.width.min(height * 4 + 8), height)
}

/// The breathing circle, with an arrow on the side the breath goes through
/// for alternate nostril breathing.
fn draw_breath(frame: &mut Frame, area: Rect, breath: &BreathState) {
    let anchor = visual_anchor(area);
    frame.render_widget(
        Paragraph::new(art::breath_circle(breath.size, anchor.width, anchor.height)),
        anchor,
    );
    let Some(side) = library::side(breath.label) else {
        return;
    };
    // Just outside the glow of a full breath.
    let middle = anchor.x + anchor.width.saturating_sub(1) / 2;
    let gap = art::circle_reach(anchor.width, anchor.height) + 4;
    let (x, arrow) = match side {
        Side::Left => (middle.saturating_sub(gap).max(anchor.x), "◀"),
        Side::Right => ((middle + gap).min(anchor.right().saturating_sub(1)), "▶"),
    };
    frame.render_widget(
        Paragraph::new(Line::from(arrow).fg(ACCENT).bold()),
        Rect::new(x, anchor.y + anchor.height / 2, 1, 1),
    );
}

/// The gaze point for gazing practice, otherwise the circle at a calm
/// resting rhythm.
fn resting_visual(frame: &mut Frame, area: Rect, meditation: &Meditation, elapsed: Duration) {
    if let Guide::Gaze(_) = meditation.guide {
        let anchor = visual_anchor(area);
        frame.render_widget(
            Paragraph::new(art::gaze_point(
                elapsed.as_secs_f64(),
                anchor.width,
                anchor.height,
            )),
            anchor,
        );
    } else {
        draw_breath(
            frame,
            area,
            &library::breath_at(&library::RESTING_BREATH, elapsed),
        );
    }
}

// ----- Done -----

fn done(frame: &mut Frame, area: Rect, app: &App) {
    let Screen::Done(Summary {
        name,
        sat,
        completed,
    }) = &app.screen
    else {
        return;
    };
    let title = if *completed {
        "Session complete"
    } else {
        "Session ended"
    };
    let outer = frame_block(title, "Enter back to your library");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = Vec::new();
    if inner.height >= 12 {
        lines.extend(art_lines(&art::SUNRISE, Style::new().fg(WARN)));
        lines.push(Line::from(""));
    }
    let headline = if *completed {
        "Well sat."
    } else {
        "Ended early. That still counts."
    };
    lines.push(Line::from(headline).centered().fg(ACCENT).bold());
    lines.push(Line::from(""));
    lines.push(
        Line::from(format!("{name} · {}", session::format_clock(*sat)))
            .centered()
            .fg(MUTED),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(stats_line(app)).centered().fg(MUTED));
    let streak = progress::current_streak(&app.data.stats.days, progress::today());
    if streak > 1 {
        lines.push(
            Line::from(format!("{streak} days in a row"))
                .centered()
                .fg(ACCENT),
        );
    }

    let content = centered(inner, inner.width, lines.len() as u16);
    frame.render_widget(Paragraph::new(lines), content);
}

// ----- Pomodoro -----

fn pomodoro_setup(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let outer = frame_block("Pomodoro", "↑↓ move · ← → change · Enter start · Esc back");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let settings = &app.data.pomodoro;
    let meditation_break = settings.break_mode == BreakMode::Meditation;
    let break_meditation = app
        .break_meditation()
        .map(|index| app.library[index].name.clone())
        .unwrap_or_default();
    let mut rows = vec![
        (
            POMO_LENGTHS,
            "Work / break",
            format!("‹ {} / {} min ›", settings.work, settings.rest),
        ),
        (
            POMO_BREAK,
            "Break",
            format!("‹ {} ›", settings.break_mode.label()),
        ),
    ];
    if meditation_break {
        rows.push((
            POMO_MEDITATION,
            "Break meditation",
            format!("‹ {break_meditation} ›"),
        ));
    }

    let mut lines = vec![
        Line::from("Work in sprints, rest in between.")
            .centered()
            .fg(ACCENT)
            .bold(),
        Line::from(""),
    ];
    for (row, label, value) in rows {
        let here = row == selected;
        lines.push(Line::from(vec![
            Span::styled(if here { "› " } else { "  " }, Style::new().fg(ACCENT)),
            Span::styled(
                format!("{label:<20}"),
                if here {
                    Style::new().fg(ACCENT).bold()
                } else {
                    Style::new()
                },
            ),
            Span::styled(value, Style::new().fg(ACCENT).bold()),
        ]));
    }
    lines.push(Line::from(""));
    for text in [
        "Nothing moves on screen while you work, so your eyes stay on your code.",
        "When a sprint ends you get a bell and a notification, and the clock",
        "waits at 00:00 until you're ready, so you can finish your thought.",
        if meditation_break {
            "Breaks start with the prep checklist, then the meditation."
        } else {
            "Breaks are a calm countdown: look away and rest your eyes."
        },
    ] {
        lines.push(Line::from(text).fg(MUTED));
    }

    let width = lines.iter().map(|l| l.width() as u16).max().unwrap_or(0);
    frame.render_widget(Paragraph::new(lines), centered(inner, width, 11));
}

fn pomodoro(frame: &mut Frame, area: Rect, app: &App) {
    let Some(pomodoro) = &app.pomodoro else {
        return;
    };
    let next_sprint = pomodoro.sprints + 1;
    let paused = pomodoro.clock().is_some_and(|clock| clock.is_paused());

    let (title, hints) = match (&pomodoro.phase, pomodoro.stopping) {
        (_, true) => ("Pomodoro".to_string(), "Esc stop · Space keep going"),
        (Phase::Work(_), _) => (format!("Sprint {next_sprint}"), "Space pause · Esc stop"),
        (Phase::WorkDone, _) => (
            format!("Sprint {} done", pomodoro.sprints),
            "Enter break · Esc stop",
        ),
        (Phase::Break(_), _) => ("Break".to_string(), "Space pause · Esc stop"),
        (Phase::BreakDone | Phase::MeditationBreak, _) => {
            ("Break's over".to_string(), "Enter next sprint · Esc stop")
        }
    };
    let outer = frame_block(&title, hints);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // (heading, clock, clock style, lines under it)
    let rest = pomodoro.settings.rest;
    let (heading, clock, clock_style, notes): (&str, String, Style, Vec<String>) =
        match &pomodoro.phase {
            Phase::Work(clock) => (
                "WORK",
                session::format_clock(clock.remaining()),
                Style::new().fg(MUTED),
                vec![format!(
                    "Then a {rest} min break · {}",
                    pomodoro.settings.break_mode.label()
                )],
            ),
            Phase::WorkDone => (
                "SPRINT DONE",
                "00:00".to_string(),
                Style::new().fg(ACCENT),
                vec![
                    "Finish your thought.".to_string(),
                    "Press Enter when you're ready for your break.".to_string(),
                ],
            ),
            Phase::Break(clock) => (
                "BREAK",
                session::format_clock(clock.remaining()),
                Style::new().fg(ACCENT),
                vec![
                    "Look away from the screen.".to_string(),
                    "Let your eyes rest on something far away.".to_string(),
                ],
            ),
            Phase::BreakDone | Phase::MeditationBreak => (
                "BREAK'S OVER",
                "00:00".to_string(),
                Style::new().fg(ACCENT),
                vec![format!("Press Enter to start sprint {next_sprint}.")],
            ),
        };

    let mut lines = vec![
        Line::from(heading).centered().fg(MUTED).bold(),
        Line::from(""),
    ];
    lines.extend(art_lines(&art::big_clock(&clock), clock_style));
    lines.push(Line::from(""));
    if pomodoro.stopping {
        lines.push(Line::from("Stop the pomodoro?").centered().fg(WARN).bold());
    } else if paused {
        lines.push(Line::from("Paused").centered().fg(WARN).bold());
    } else {
        for note in notes {
            lines.push(Line::from(note).centered().fg(MUTED));
        }
    }
    if pomodoro.sprints > 0 {
        lines.push(Line::from(""));
        let done = pomodoro.sprints;
        lines.push(
            Line::from(format!(
                "{done} sprint{} done",
                if done == 1 { "" } else { "s" }
            ))
            .centered()
            .fg(MUTED),
        );
    }
    let height = lines.len() as u16;
    frame.render_widget(Paragraph::new(lines), centered(inner, inner.width, height));
}

// ----- Stats -----

fn stats(frame: &mut Frame, area: Rect, app: &App) {
    let outer = frame_block("Your practice", "Esc back");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let days = &app.data.stats.days;
    let today = progress::today();
    let streak = progress::current_streak(days, today);
    let best = progress::longest_streak(days);

    let sprints = app.data.stats.sprints;
    let mut lines = vec![
        Line::from(stats_line(app)).centered().bold(),
        Line::from(format!(
            "{sprints} pomodoro sprint{}",
            if sprints == 1 { "" } else { "s" }
        ))
        .centered()
        .fg(MUTED),
        Line::from(vec![
            Span::styled(
                format!(
                    "{streak} day{} in a row",
                    if streak == 1 { "" } else { "s" }
                ),
                Style::new().fg(ACCENT),
            ),
            Span::styled(format!("  ·  best {best}"), Style::new().fg(MUTED)),
        ])
        .centered(),
        Line::from(""),
    ];

    // As many weeks as fit, up to a year. Labels take 4 columns.
    let weeks = (inner.width.saturating_sub(8) as usize).min(52);
    let grid = progress::heatmap(days, today, weeks);
    let label_style = Style::new().fg(MUTED);
    lines.push(Line::styled(
        format!("    {}", progress::month_labels(&grid)),
        label_style,
    ));
    for (weekday, row) in grid.iter().enumerate() {
        let label = match weekday {
            0 => "Mon ",
            2 => "Wed ",
            4 => "Fri ",
            _ => "    ",
        };
        let mut spans = vec![Span::styled(label, label_style)];
        spans.extend(row.iter().map(|cell| {
            let style = match cell {
                Cell::Day { sessions: 0, .. } => Style::new().fg(MUTED),
                _ => Style::new().fg(ACCENT),
            };
            Span::styled(cell.glyph().to_string(), style)
        }));
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    let mut legend = vec![
        Span::styled("Less ", label_style),
        Span::styled(format!("{} ", progress::EMPTY), label_style),
    ];
    for shade in progress::SHADES {
        legend.push(Span::styled(format!("{shade} "), Style::new().fg(ACCENT)));
    }
    legend.push(Span::styled("More", label_style));
    lines.push(Line::from(legend));

    let width = (weeks as u16 + 4).max(lines.iter().map(|l| l.width() as u16).max().unwrap_or(0));
    let content = centered(inner, width, lines.len() as u16);
    frame.render_widget(Paragraph::new(lines), content);
}

// ----- Settings -----

fn settings(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let outer = frame_block("Settings", "↑↓ move · Enter / ← → change · Esc back");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let settings = &app.data.settings;
    let on_off = |on: bool| if on { " on" } else { "off" }.to_string();
    let no_device = settings.sound && app.sound.is_none();

    // (row, label, value, greyed out?)
    let rows = [
        (
            SETTING_TOUR,
            "Show the tour on next launch",
            Some(on_off(settings.show_tutorial)),
            false,
        ),
        (
            SETTING_WALKTHROUGH,
            "Walkthrough the first time",
            Some(on_off(settings.walkthroughs)),
            false,
        ),
        (
            SETTING_PREFLIGHT,
            "Prep checklist before sessions",
            Some(on_off(settings.preflight)),
            false,
        ),
        (
            SETTING_PATH,
            "Ask before later practices",
            Some(on_off(settings.follow_path)),
            false,
        ),
        (
            SETTING_CLOCK,
            "Session clock",
            Some(format!("‹ {} ›", settings.clock.label())),
            false,
        ),
        (SETTING_SOUND, "Sounds", Some(on_off(settings.sound)), false),
        (
            SETTING_VOLUME,
            "  Volume",
            Some(format!("‹ {}% ›", settings.volume)),
            !settings.sound,
        ),
        (
            SETTING_TONES,
            "  Breathing tones",
            Some(on_off(settings.breath_tones)),
            !settings.sound,
        ),
        (
            SETTING_BACKGROUND,
            "  Background sound",
            Some(format!("‹ {} ›", settings.background.label())),
            !settings.sound,
        ),
        (SETTING_REPLAY, "Replay the tour now", None, false),
    ];

    let label_width = 34;
    let mut lines = Vec::new();
    for (row, label, value, greyed) in rows {
        let is_selected = row == selected;
        let pointer = if is_selected { "› " } else { "  " };
        let label_style = match (is_selected, greyed) {
            (true, _) => Style::new().fg(ACCENT).bold(),
            (false, true) => Style::new().fg(MUTED),
            (false, false) => Style::new(),
        };
        let mut spans = vec![
            Span::styled(pointer, Style::new().fg(ACCENT)),
            Span::styled(format!("{label:<label_width$}"), label_style),
        ];
        if let Some(value) = value {
            let lit = !greyed && value != "off" && !value.contains("off");
            let value_style = if lit {
                Style::new().fg(ACCENT).bold()
            } else {
                Style::new().fg(MUTED)
            };
            spans.push(Span::styled(value, value_style));
        }
        lines.push(Line::from(spans));
        if row == SETTING_CLOCK || row == SETTING_BACKGROUND {
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));
    if no_device {
        lines.push(Line::from("No audio device found, so only the terminal bell rings.").fg(WARN));
    }
    lines.push(Line::from(format!("Your practice: {}", stats_line(app))).fg(MUTED));
    if let Some(path) = storage::data_path() {
        let room = (inner.width as usize).saturating_sub("Saved in ".len() + 2);
        let shown = short_path(&path, dirs::home_dir().as_deref(), room);
        lines.push(Line::from(format!("Saved in {shown}")).fg(MUTED));
    }

    let width = lines.iter().map(|l| l.width() as u16).max().unwrap_or(0);
    let content = centered(inner, width, lines.len() as u16);
    frame.render_widget(Paragraph::new(lines), content);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SaveData;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};
    use ratatui::{Terminal, backend::TestBackend};

    /// Draws a 10 minute session and returns the screen, one string per row.
    fn session_screen(clock: ClockStyle, minimal: bool) -> Vec<String> {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        data.settings.clock = clock;
        let mut app = App::new(data, None, None, Vec::new());
        let mut session = Session::start(library::presets().remove(0), 10);
        session.dim = minimal;
        app.screen = Screen::Session {
            session: Box::new(session),
            ending: false,
        };
        draw(&mut app, 80, 30)
    }

    /// The whole screen at `width` x `height`, one string per row.
    fn draw(app: &mut App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    fn fresh_app() -> App {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        App::new(data, None, None, Vec::new())
    }

    #[test]
    fn a_walkthrough_shows_its_steps_then_a_round_with_a_side() {
        let mut app = fresh_app();
        app.handle_key(KeyEvent::from(KeyCode::Char('i')));
        let setup = app.selected().unwrap().walkthrough.len();
        let text = draw(&mut app, 80, 30).join("\n");
        assert!(text.contains("Raise your right hand."), "{text}");
        assert!(text.contains(&format!("Step 1 of {setup}")), "{text}");

        for _ in 0..setup {
            app.handle_key(KeyEvent::from(KeyCode::Enter));
        }
        let text = draw(&mut app, 80, 30).join("\n");
        assert!(text.contains("Breathe in LEFT · 8"), "{text}");
        assert!(text.contains("Slow at first"), "{text}");
        assert!(text.contains('◀'), "{text}");
    }

    #[test]
    fn a_folder_header_lists_when_to_use_each_one() {
        let mut app = fresh_app();
        app.handle_key(KeyEvent::from(KeyCode::Up));
        let text = draw(&mut app, 110, 30).join("\n");
        assert!(text.contains("Box Breathing"), "{text}");
        assert!(text.contains("Before a call or demo"), "{text}");
    }

    #[test]
    fn a_big_folder_never_hides_names_without_saying_so() {
        for height in [20, 26, 40] {
            let mut app = fresh_app();
            app.handle_key(KeyEvent::from(KeyCode::Tab));
            app.handle_key(KeyEvent::from(KeyCode::Tab));
            let Some(ListRow::Folder(folder)) = app.selected_row() else {
                panic!("not on a folder");
            };
            let names: Vec<String> = app.in_folder(folder).map(|m| m.name.clone()).collect();
            let text = draw(&mut app, 100, height).join("\n");
            let all_there = names.iter().all(|name| text.contains(name.as_str()));
            assert!(all_there || text.contains("more"), "{height}:\n{text}");
        }
    }

    #[test]
    fn the_dim_view_keeps_the_instruction() {
        let screen = session_screen(ClockStyle::Remaining, true).join("\n");
        assert!(screen.contains("Breathe in LEFT"), "{screen}");
    }

    #[test]
    fn library_hints_fit_a_narrow_window() {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        let mut app = App::new(data, None, None, Vec::new());
        let bottom = draw(&mut app, 50, 30).pop().unwrap();
        assert!(
            bottom.starts_with('╰') && bottom.contains("q quit"),
            "{bottom}"
        );
    }

    #[test]
    fn the_session_clock_is_one_small_line() {
        for minimal in [false, true] {
            let screen = session_screen(ClockStyle::Remaining, minimal);
            let rows: Vec<&String> = screen.iter().filter(|r| r.contains("10:00")).collect();
            assert_eq!(rows.len(), 1, "{minimal}: {screen:#?}");
            // It sits at the end of the progress line.
            assert!(rows[0].contains('─'), "{}", rows[0]);
            // No big block digits anywhere.
            assert!(!screen.iter().any(|r| r.contains("▀▀▀")), "{screen:#?}");

            let counting_up = session_screen(ClockStyle::Elapsed, minimal);
            assert!(counting_up.iter().any(|r| r.contains("00:00")));

            let hidden = session_screen(ClockStyle::Hidden, minimal);
            assert!(!hidden.iter().any(|r| r.contains(':')), "{hidden:#?}");
        }
    }

    #[test]
    fn words_wrap_at_spaces() {
        assert_eq!(
            wrap_words("one two three four five", 9),
            ["one two", "three", "four five"]
        );
        assert_eq!(wrap_words("", 10), Vec::<String>::new());
        // A word longer than the width gets its own row rather than vanishing.
        assert_eq!(
            wrap_words("a supercalifragilistic b", 5),
            ["a", "supercalifragilistic", "b"]
        );
    }

    #[test]
    fn short_paths_are_left_alone() {
        let home = Path::new("/home/me");
        let path = Path::new("/home/me/.local/share/meditui/data.json");
        assert_eq!(
            short_path(path, Some(home), 80),
            "~/.local/share/meditui/data.json"
        );
    }

    #[test]
    fn long_paths_lose_their_middle() {
        let home = Path::new("/home/me");
        let path = Path::new("/home/me/.local/share/meditui/data.json");
        assert_eq!(short_path(path, Some(home), 24), "~/.local/.../data.json");
        assert_eq!(short_path(path, Some(home), 15), ".../data.json");
        assert_eq!(short_path(path, Some(home), 5).chars().count(), 5);
    }

    #[test]
    fn paths_outside_home_keep_their_root() {
        let path = Path::new("/tmp/some/very/deep/folder/meditui/data.json");
        assert_eq!(short_path(path, None, 22), "/tmp/.../data.json");
    }
}
