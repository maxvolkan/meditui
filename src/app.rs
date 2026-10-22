//! App state and what every key does on every screen.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::library::{self, Category, Guide, Meditation, Tag, Unlock};
use crate::pomodoro::{self, BreakMode, Event, Pomodoro};
use crate::progress;
use crate::session::Session;
use crate::sound::Sound;
use crate::storage::{self, CustomMeditation, SaveData};
use crate::tutorial;

pub const MAX_MINUTES: u64 = 180;
pub const MAX_NAME_LEN: usize = 40;
/// Breath phases shorter than this get no tone and no countdown.
pub const MIN_TONE_SECONDS: f64 = 2.0;

// Rows on the settings screen, top to bottom.
pub const SETTING_TOUR: usize = 0;
pub const SETTING_PREFLIGHT: usize = 1;
pub const SETTING_PATH: usize = 2;
pub const SETTING_CLOCK: usize = 3;
pub const SETTING_SOUND: usize = 4;
pub const SETTING_VOLUME: usize = 5;
pub const SETTING_TONES: usize = 6;
pub const SETTING_BACKGROUND: usize = 7;
pub const SETTING_REPLAY: usize = 8;
pub const SETTINGS_COUNT: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormStep {
    Name,
    Minutes,
}

#[derive(Debug)]
pub struct Summary {
    pub name: String,
    pub sat: Duration,
    pub completed: bool,
}

/// Which screen is showing. Only one at a time, and each carries the
/// little bit of state it needs.
#[derive(Debug)]
pub enum Screen {
    Tutorial {
        page: usize,
    },
    Library,
    ConfirmStart {
        minutes: u64,
    },
    /// "This one usually comes later": start anyway, unlock it, or go back.
    Locked,
    /// The prep checklist before a session: which boxes are ticked, and
    /// which row the cursor is on.
    Preflight {
        minutes: u64,
        checked: Vec<bool>,
        selected: usize,
    },
    ConfirmDelete,
    AddForm {
        step: FormStep,
    },
    /// `ending` is true while asking "end this session?".
    Session {
        // Boxed: a session is much bigger than every other screen.
        session: Box<Session>,
        ending: bool,
    },
    Done(Summary),
    Settings {
        selected: usize,
    },
    /// Your practice so far: totals, streaks and the heatmap.
    Stats,
    /// Choosing sprint lengths and the kind of break.
    PomodoroSetup {
        selected: usize,
    },
    /// A running pomodoro (the state itself is `App::pomodoro`, because a
    /// meditation break hands the screen over to a normal session).
    Pomodoro,
}

/// A folder in the library: one per category, plus one for your own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Folder {
    Category(Category),
    Yours,
}

impl Folder {
    /// Every folder, in list order.
    pub fn all() -> impl Iterator<Item = Folder> {
        Category::ALL
            .into_iter()
            .map(Folder::Category)
            .chain(std::iter::once(Folder::Yours))
    }

    /// Where a meditation lives: its category, or "Your own" if it has none.
    pub fn of(meditation: &Meditation) -> Folder {
        meditation.category.map_or(Folder::Yours, Folder::Category)
    }

    pub fn label(self) -> &'static str {
        match self {
            Folder::Category(category) => category.label(),
            Folder::Yours => "Your own",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Folder::Category(category) => category.blurb(),
            Folder::Yours => "The ones you added with a, and recipes without a category.",
        }
    }

    /// How a closed folder is remembered in the settings.
    fn key(self) -> &'static str {
        match self {
            Folder::Category(Category::FocusReset) => "focus_reset",
            Folder::Category(Category::DownRegulation) => "down_regulation",
            Folder::Category(Category::CognitiveDefusion) => "cognitive_defusion",
            Folder::Category(Category::SensoryRest) => "sensory_rest",
            Folder::Category(Category::DeepUnwind) => "deep_unwind",
            Folder::Yours => "yours",
        }
    }
}

/// One row of the library list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Row {
    Folder(Folder),
    /// A meditation, by its place in `App::library`.
    Item(usize),
}

// Rows on the pomodoro setup screen.
pub const POMO_LENGTHS: usize = 0;
pub const POMO_BREAK: usize = 1;
pub const POMO_MEDITATION: usize = 2;

pub struct App {
    pub screen: Screen,
    pub library: Vec<Meditation>,
    /// The rows on screen: folder headers, and the meditations in open
    /// folders that the filter lets through. `list_state` points into this.
    pub visible: Vec<Row>,
    pub list_state: ListState,
    /// Only show office-safe meditations (`o`).
    pub office_only: bool,
    pub data: SaveData,
    pub name_input: String,
    pub minutes_input: String,
    pub form_error: Option<String>,
    /// A one-line message under the library ("Added ...", save errors).
    pub status: Option<String>,
    pub quit: bool,
    /// Where to save. `None` means "don't save" (used by the tests).
    pub data_path: Option<PathBuf>,
    /// The audio device. `None` if there isn't one (or in tests).
    pub sound: Option<Sound>,
    /// Meditations from recipe files, loaded once at startup.
    recipes: Vec<Meditation>,
    /// A pomodoro in progress, if there is one.
    pub pomodoro: Option<Pomodoro>,
    /// Terminal bell and desktop notifications when a sprint or break ends.
    /// Off by default so tests stay quiet; `main` turns it on.
    pub desktop_alerts: bool,
    launched: Instant,
}

impl App {
    pub fn new(
        data: SaveData,
        data_path: Option<PathBuf>,
        sound: Option<Sound>,
        recipes: Vec<Meditation>,
    ) -> Self {
        let screen = if data.settings.show_tutorial {
            Screen::Tutorial { page: 0 }
        } else {
            Screen::Library
        };
        let mut app = Self {
            screen,
            library: Vec::new(),
            visible: Vec::new(),
            list_state: ListState::default().with_selected(Some(0)),
            office_only: false,
            data,
            name_input: String::new(),
            minutes_input: String::new(),
            form_error: None,
            status: None,
            quit: false,
            data_path,
            sound,
            recipes,
            pomodoro: None,
            desktop_alerts: false,
            launched: Instant::now(),
        };
        app.rebuild_library();
        app
    }

    /// Time since launch, for animations that run outside a session.
    pub fn uptime(&self) -> Duration {
        self.launched.elapsed()
    }

    pub fn selected(&self) -> Option<&Meditation> {
        self.library.get(self.selected_index()?)
    }

    /// The row under the cursor: a folder header or a meditation.
    pub fn selected_row(&self) -> Option<Row> {
        self.visible.get(self.list_state.selected()?).copied()
    }

    /// The selected meditation's place in `library`. `None` on a header.
    fn selected_index(&self) -> Option<usize> {
        match self.selected_row()? {
            Row::Item(index) => Some(index),
            Row::Folder(_) => None,
        }
    }

    /// The folder the cursor is on, or in.
    pub fn current_folder(&self) -> Option<Folder> {
        match self.selected_row()? {
            Row::Folder(folder) => Some(folder),
            // The library may have just been rebuilt under the cursor.
            Row::Item(index) => self.library.get(index).map(Folder::of),
        }
    }

    pub fn has_custom(&self) -> bool {
        !self.data.custom.is_empty()
    }

    pub fn is_open(&self, folder: Folder) -> bool {
        !self.data.settings.closed_folders.contains(folder.key())
    }

    /// How many meditations in `folder` the filter lets through.
    pub fn folder_count(&self, folder: Folder) -> usize {
        self.library
            .iter()
            .filter(|m| Folder::of(m) == folder && self.passes_filters(m))
            .count()
    }

    /// For a practice that usually comes later and hasn't been earned yet:
    /// the rule, and how many days of the earlier practice you have so far.
    pub fn lock_progress(&self, meditation: &Meditation) -> Option<(Unlock, u32)> {
        let unlock = meditation.unlock?;
        let settings = &self.data.settings;
        if !settings.follow_path || settings.unlocked.contains(&meditation.name) {
            return None;
        }
        let days = self
            .data
            .stats
            .practice
            .get(unlock.after)
            .map_or(0, |days| days.len() as u32);
        (days < unlock.days).then_some((unlock, days))
    }

    /// Select a row of the list, clamped to the last row.
    fn select(&mut self, position: usize) {
        if self.visible.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state
                .select(Some(position.min(self.visible.len() - 1)));
        }
    }

    /// Select a particular row, if it's on screen.
    fn select_row(&mut self, row: Row) -> bool {
        match self.visible.iter().position(|r| *r == row) {
            Some(position) => {
                self.select(position);
                true
            }
            None => false,
        }
    }

    /// Select the first meditation on screen, in `folder` if one is given.
    fn select_first_item(&mut self, folder: Option<Folder>) -> bool {
        let position = self.visible.iter().position(|row| match row {
            Row::Item(i) => folder.is_none_or(|f| Folder::of(&self.library[*i]) == f),
            Row::Folder(_) => false,
        });
        match position {
            Some(position) => {
                self.select(position);
                true
            }
            None => false,
        }
    }

    /// Built-in meditations, then recipes, then your own (always last).
    fn rebuild_library(&mut self) {
        self.library = library::presets();
        self.library.extend(self.recipes.iter().cloned());
        self.library.extend(
            self.data
                .custom
                .iter()
                .map(|c| Meditation::custom(c.name.clone(), c.minutes)),
        );
        self.refresh_visible();
    }

    fn passes_filters(&self, meditation: &Meditation) -> bool {
        !self.office_only || meditation.tags.contains(&Tag::OfficeSafe)
    }

    /// Recompute the rows: each folder's header, then its meditations if
    /// it's open. Folders with nothing to show are left out. The cursor stays
    /// where it was if it can, or moves to somewhere nearby.
    fn refresh_visible(&mut self) {
        let keep = self.selected_row();
        let keep_folder = self.current_folder();
        let mut rows = Vec::new();
        for folder in Folder::all() {
            let items: Vec<usize> = (0..self.library.len())
                .filter(|&i| {
                    let m = &self.library[i];
                    Folder::of(m) == folder && self.passes_filters(m)
                })
                .collect();
            if items.is_empty() {
                continue;
            }
            rows.push(Row::Folder(folder));
            if self.is_open(folder) {
                rows.extend(items.into_iter().map(Row::Item));
            }
        }
        self.visible = rows;

        let found = keep.is_some_and(|row| self.select_row(row))
            || keep_folder.is_some_and(|folder| {
                self.select_first_item(Some(folder)) || self.select_row(Row::Folder(folder))
            })
            || self.select_first_item(None);
        if !found {
            self.select(0);
        }
    }

    /// Opens or closes a folder (`None` toggles) and puts the cursor on it.
    fn set_folder_open(&mut self, folder: Folder, open: Option<bool>) {
        let open = open.unwrap_or(!self.is_open(folder));
        let closed = &mut self.data.settings.closed_folders;
        if open {
            closed.remove(folder.key());
        } else {
            closed.insert(folder.key().to_string());
        }
        self.persist();
        self.refresh_visible();
        self.select_row(Row::Folder(folder));
    }

    /// Moves to the next (or previous) folder header, wrapping around.
    fn jump_folder(&mut self, step: isize) {
        let headers: Vec<usize> = self
            .visible
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, Row::Folder(_)))
            .map(|(position, _)| position)
            .collect();
        let current = self.list_state.selected().unwrap_or(0);
        let next = if step > 0 {
            headers.iter().find(|&&p| p > current).or(headers.first())
        } else {
            headers
                .iter()
                .rev()
                .find(|&&p| p < current)
                .or(headers.last())
        };
        if let Some(&position) = next {
            self.select(position);
        }
    }

    /// Puts a meditation on screen and selects it: drops the office filter if
    /// it's hiding it, and opens its folder.
    fn reveal(&mut self, index: usize) {
        let Some(meditation) = self.library.get(index) else {
            return;
        };
        if !self.passes_filters(meditation) {
            self.office_only = false;
        }
        let folder = Folder::of(meditation);
        if !self.is_open(folder) {
            self.data.settings.closed_folders.remove(folder.key());
            self.persist();
        }
        self.refresh_visible();
        self.select_row(Row::Item(index));
    }

    fn persist(&mut self) {
        if let Some(path) = &self.data_path
            && let Err(err) = storage::save(path, &self.data)
        {
            self.status = Some(format!("Couldn't save: {err}"));
        }
    }

    // ----- Sound helpers -----

    fn volume(&self) -> f32 {
        self.data.settings.volume as f32 / 100.0
    }

    /// The audio device, but only when sounds are switched on.
    fn audio(&self) -> Option<&Sound> {
        if self.data.settings.sound {
            self.sound.as_ref()
        } else {
            None
        }
    }

    fn audio_mut(&mut self) -> Option<&mut Sound> {
        if self.data.settings.sound {
            self.sound.as_mut()
        } else {
            None
        }
    }

    fn stop_background(&mut self) {
        if let Some(sound) = self.sound.as_mut() {
            sound.stop_background();
        }
    }

    // ----- Every frame -----

    /// Runs every time round the main loop, key or no key.
    pub fn tick(&mut self) {
        if let Some(sound) = self.sound.as_mut() {
            sound.tick();
        }
        if let Some(event) = self.pomodoro.as_mut().and_then(Pomodoro::tick) {
            self.pomodoro_alert(event);
        }
        let tones = self.data.settings.breath_tones;
        let Screen::Session { session, .. } = &mut self.screen else {
            return;
        };
        if session.is_finished() {
            self.end_session(true);
            return;
        }
        let elapsed = session.elapsed();

        // A chime every N minutes, if the meditation asks for one.
        let mut chime = false;
        if let Some(minutes) = session.meditation.interval_bell {
            let due = elapsed.as_secs() / (minutes.max(1) * 60);
            if due > session.interval_bells {
                session.interval_bells = due;
                chime = true;
            }
        }

        // A sound when the breath turns or a new cue appears. `breath` is
        // Some(state) for breathing patterns, None for text cues.
        let mut tone = None;
        let current = match &session.meditation.guide {
            Guide::Breath(phases) => {
                let state = library::breath_at(phases, elapsed);
                Some((state.index, Some(state)))
            }
            Guide::Prompts(cues) | Guide::Gaze(cues) => Some((
                library::prompt_index(cues.len(), elapsed, session.total()),
                None,
            )),
            Guide::Silent => None,
        };
        if let Some((cue, breath)) = current {
            let previous = session.last_cue.replace(cue);
            // The very first cue is covered by the opening bell.
            if previous.is_some() && previous != Some(cue) {
                match breath {
                    // Fast breathing (bellows, breath of fire) would turn
                    // tones into a buzz, so only slower breaths get one.
                    Some(state) if state.phase_seconds >= MIN_TONE_SECONDS => {
                        tone = state.motion.rising().filter(|_| tones);
                    }
                    Some(_) => {}
                    None => chime = true,
                }
            }
        }

        let volume = self.volume();
        if let Some(sound) = self.audio() {
            if let Some(rising) = tone {
                sound.breath_tone(rising, volume);
            }
            if chime {
                sound.chime(volume);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        match self.screen {
            Screen::Tutorial { page } => self.tutorial_key(key, page),
            Screen::Library => self.library_key(key),
            Screen::ConfirmStart { minutes } => self.confirm_start_key(key, minutes),
            Screen::Locked => self.locked_key(key),
            Screen::Preflight { .. } => self.preflight_key(key),
            Screen::ConfirmDelete => self.confirm_delete_key(key),
            Screen::AddForm { step } => self.form_key(key, step),
            Screen::Session { .. } => self.session_key(key),
            Screen::Done(_) => self.done_key(key),
            Screen::Settings { selected } => self.settings_key(key, selected),
            Screen::PomodoroSetup { selected } => self.pomodoro_setup_key(key, selected),
            Screen::Pomodoro => self.pomodoro_key(key),
            Screen::Stats => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('t')
                ) {
                    self.screen = Screen::Library;
                }
            }
        }
    }

    // ----- Tour -----

    fn tutorial_key(&mut self, key: KeyEvent, page: usize) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.finish_tutorial(),
            KeyCode::Enter if page == tutorial::LAST_PAGE => {
                self.finish_tutorial();
                self.start_tutorial_try();
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                if page < tutorial::LAST_PAGE {
                    self.screen = Screen::Tutorial { page: page + 1 };
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                self.screen = Screen::Tutorial {
                    page: page.saturating_sub(1),
                };
            }
            _ => {}
        }
    }

    /// Finishing or skipping the tour both count as "seen it".
    fn finish_tutorial(&mut self) {
        self.data.settings.show_tutorial = false;
        self.persist();
        self.screen = Screen::Library;
    }

    fn start_tutorial_try(&mut self) {
        let Some(index) = self
            .library
            .iter()
            .position(|m| m.name == library::TUTORIAL_PRESET)
        else {
            return;
        };
        self.reveal(index);
        let meditation = self.library[index].clone();
        self.begin_session(meditation, 1);
    }

    // ----- Library -----

    fn library_key(&mut self, key: KeyEvent) {
        self.status = None;
        let current = self.list_state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.select(current + 1),
            KeyCode::Up | KeyCode::Char('k') => self.select(current.saturating_sub(1)),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::Enter | KeyCode::Char(' ') => match self.selected_row() {
                Some(Row::Folder(folder)) => self.set_folder_open(folder, None),
                Some(Row::Item(_)) => self.choose_selected(),
                None => {}
            },
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(folder) = self.current_folder() {
                    self.set_folder_open(folder, Some(false));
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(Row::Folder(folder)) = self.selected_row() {
                    self.set_folder_open(folder, Some(true));
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => self.open_form(),
            KeyCode::Char('d') | KeyCode::Char('D') => match self.selected() {
                Some(meditation) if meditation.custom => self.screen = Screen::ConfirmDelete,
                Some(meditation) if meditation.recipe => {
                    self.status = Some(
                        "That's a recipe: edit or delete its file in ~/.config/meditui/recipes."
                            .to_string(),
                    );
                }
                Some(_) => {
                    self.status = Some("Built-in meditations can't be deleted.".to_string());
                }
                None => {}
            },
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.screen = Screen::Settings { selected: 0 };
            }
            KeyCode::Char('?') => self.screen = Screen::Tutorial { page: 0 },
            KeyCode::Char('t') | KeyCode::Char('T') => self.screen = Screen::Stats,
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.screen = Screen::PomodoroSetup { selected: 0 };
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.office_only = !self.office_only;
                self.refresh_visible();
            }
            KeyCode::Tab => self.jump_folder(1),
            KeyCode::BackTab => self.jump_folder(-1),
            _ => {}
        }
    }

    /// Enter on a meditation: choose the length, unless it usually comes
    /// later, in which case say so first.
    fn choose_selected(&mut self) {
        let Some(meditation) = self.selected() else {
            return;
        };
        self.screen = if self.lock_progress(meditation).is_some() {
            Screen::Locked
        } else {
            Screen::ConfirmStart {
                minutes: meditation.minutes,
            }
        };
    }

    fn locked_key(&mut self, key: KeyEvent) {
        let Some(meditation) = self.selected() else {
            self.screen = Screen::Library;
            return;
        };
        let (name, minutes) = (meditation.name.clone(), meditation.minutes);
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => self.screen = Screen::ConfirmStart { minutes },
            KeyCode::Char('u') => {
                self.data.settings.unlocked.insert(name);
                self.persist();
                self.screen = Screen::ConfirmStart { minutes };
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Library;
            }
            _ => {}
        }
    }

    fn confirm_start_key(&mut self, key: KeyEvent, minutes: u64) {
        let adjust = |minutes: u64| Screen::ConfirmStart {
            minutes: minutes.clamp(1, MAX_MINUTES),
        };
        match key.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
                self.screen = adjust(minutes.saturating_sub(1));
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') | KeyCode::Char('=') => {
                self.screen = adjust(minutes + 1);
            }
            KeyCode::Down | KeyCode::Char('j') => self.screen = adjust(minutes.saturating_sub(5)),
            KeyCode::Up | KeyCode::Char('k') => self.screen = adjust(minutes + 5),
            KeyCode::Enter | KeyCode::Char('y') => {
                if let Some(meditation) = self.selected().cloned() {
                    if self.data.settings.preflight {
                        let steps = library::prep_steps(&meditation).len();
                        self.screen = Screen::Preflight {
                            minutes,
                            checked: vec![false; steps],
                            selected: 0,
                        };
                    } else {
                        self.begin_session(meditation, minutes);
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                self.screen = Screen::Library;
            }
            _ => {}
        }
    }

    fn preflight_key(&mut self, key: KeyEvent) {
        let Screen::Preflight {
            minutes,
            checked,
            selected,
        } = &mut self.screen
        else {
            return;
        };
        let minutes = *minutes;
        let last = checked.len().saturating_sub(1);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
            // Tick the box and move on to the next one.
            KeyCode::Char(' ') | KeyCode::Char('x') => {
                if let Some(tick) = checked.get_mut(*selected) {
                    *tick = !*tick;
                    *selected = (*selected + 1).min(last);
                }
            }
            KeyCode::Char(c @ '1'..='9') => {
                let index = c as usize - '1' as usize;
                if let Some(tick) = checked.get_mut(index) {
                    *tick = !*tick;
                }
            }
            // Ready (or skipping): start, whatever's ticked.
            KeyCode::Enter | KeyCode::Char('s') => {
                if let Some(meditation) = self.selected().cloned() {
                    self.begin_session(meditation, minutes);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Backing out of a pomodoro break's checklist returns to the
                // "ready for a break?" screen rather than the library.
                if let Some(pomodoro) = &mut self.pomodoro {
                    pomodoro.phase = pomodoro::Phase::WorkDone;
                    self.screen = Screen::Pomodoro;
                } else {
                    self.screen = Screen::Library;
                }
            }
            _ => {}
        }
    }

    fn confirm_delete_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Your own meditations are always the last ones in `library`.
                let first_custom = self.library.len() - self.data.custom.len();
                if let Some(index) = self.selected_index()
                    && let Some(custom_index) = index.checked_sub(first_custom)
                    && custom_index < self.data.custom.len()
                {
                    let removed = self.data.custom.remove(custom_index);
                    self.rebuild_library();
                    self.status = Some(format!("Deleted \"{}\".", removed.name));
                    self.persist();
                }
                self.screen = Screen::Library;
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Library;
            }
            _ => {}
        }
    }

    // ----- Add form -----

    fn open_form(&mut self) {
        self.name_input.clear();
        self.minutes_input = "10".to_string();
        self.form_error = None;
        self.screen = Screen::AddForm {
            step: FormStep::Name,
        };
    }

    fn close_form(&mut self) {
        self.name_input.clear();
        self.minutes_input.clear();
        self.form_error = None;
        self.screen = Screen::Library;
    }

    fn form_key(&mut self, key: KeyEvent, step: FormStep) {
        match (key.code, step) {
            (KeyCode::Esc, FormStep::Name) => self.close_form(),
            (KeyCode::Esc, FormStep::Minutes) => {
                self.form_error = None;
                self.screen = Screen::AddForm {
                    step: FormStep::Name,
                };
            }
            (KeyCode::Enter, FormStep::Name) => {
                if self.name_input.trim().is_empty() {
                    self.form_error = Some("Give it a name first.".to_string());
                } else {
                    self.form_error = None;
                    self.screen = Screen::AddForm {
                        step: FormStep::Minutes,
                    };
                }
            }
            (KeyCode::Enter, FormStep::Minutes) => self.submit_form(),
            (KeyCode::Backspace, FormStep::Name) => {
                self.name_input.pop();
            }
            (KeyCode::Backspace, FormStep::Minutes) => {
                self.minutes_input.pop();
            }
            (KeyCode::Char(c), FormStep::Name) => {
                if self.name_input.chars().count() < MAX_NAME_LEN {
                    self.name_input.push(c);
                }
            }
            (KeyCode::Char(c), FormStep::Minutes)
                if c.is_ascii_digit() && self.minutes_input.len() < 3 =>
            {
                self.minutes_input.push(c);
            }
            _ => {}
        }
    }

    fn submit_form(&mut self) {
        match self.minutes_input.parse::<u64>() {
            Ok(minutes) if (1..=MAX_MINUTES).contains(&minutes) => {
                let name = self.name_input.trim().to_string();
                self.data.custom.push(CustomMeditation {
                    name: name.clone(),
                    minutes,
                });
                self.rebuild_library();
                // Make sure the new one is on screen, and selected.
                self.reveal(self.library.len() - 1);
                self.close_form();
                self.status = Some(format!("Added \"{name}\"."));
                self.persist();
            }
            Ok(0) => self.form_error = Some("It needs at least 1 minute.".to_string()),
            Ok(_) => self.form_error = Some(format!("Keep it to {MAX_MINUTES} minutes or less.")),
            Err(_) => self.form_error = Some("Type a number of minutes, like 10.".to_string()),
        }
    }

    // ----- Session -----

    fn begin_session(&mut self, meditation: Meditation, minutes: u64) {
        self.screen = Screen::Session {
            session: Box::new(Session::start(meditation, minutes)),
            ending: false,
        };
        let volume = self.volume();
        let background = self.data.settings.background;
        if let Some(sound) = self.audio_mut() {
            sound.bell(volume);
            sound.start_background(background, volume);
        }
    }

    fn session_key(&mut self, key: KeyEvent) {
        let Screen::Session { session, ending } = &mut self.screen else {
            return;
        };
        let mut end_now = false;
        let mut toggle_view = false;
        match key.code {
            KeyCode::Char('h') => toggle_view = true,
            KeyCode::Char(' ') | KeyCode::Char('p') => {
                if *ending {
                    // "Keep going": un-ask, and resume if we paused to ask.
                    *ending = false;
                    if session.is_paused() {
                        session.toggle_pause();
                    }
                } else {
                    session.toggle_pause();
                }
            }
            // Esc asks first, so a bumped key doesn't end a sit by accident.
            KeyCode::Esc | KeyCode::Char('q') => {
                if *ending {
                    end_now = true;
                } else {
                    *ending = true;
                    if !session.is_paused() {
                        session.toggle_pause();
                    }
                }
            }
            _ => {}
        }
        let paused = session.is_paused();

        if toggle_view {
            self.data.settings.minimal_view = !self.data.settings.minimal_view;
            self.persist();
            return;
        }
        if end_now {
            self.end_session(false);
        } else if let Some(sound) = self.audio() {
            sound.pause_background(paused);
        }
    }

    fn end_session(&mut self, completed: bool) {
        let Screen::Session { session, .. } = std::mem::replace(&mut self.screen, Screen::Library)
        else {
            return;
        };
        self.stop_background();

        let sat = session.elapsed();
        if completed {
            self.data.stats.sessions += 1;
            let today = progress::day_key(progress::today());
            *self.data.stats.days.entry(today.clone()).or_insert(0) += 1;
            self.data
                .stats
                .practice
                .entry(session.meditation.name.clone())
                .or_default()
                .insert(today);
        }
        self.data.stats.seconds += sat.as_secs();
        self.persist();

        if completed && self.data.settings.sound {
            match self.sound.as_ref() {
                Some(sound) => sound.closing_bell(self.volume()),
                None => ring_terminal_bell(),
            }
        }
        // A meditation break: hand back to the pomodoro instead of the summary.
        if let Some(pomodoro) = &mut self.pomodoro {
            pomodoro.phase = pomodoro::Phase::BreakDone;
            self.screen = Screen::Pomodoro;
            self.desktop_alert("Break's over. Ready for the next sprint?");
            return;
        }
        self.screen = Screen::Done(Summary {
            name: session.meditation.name,
            sat,
            completed,
        });
    }

    fn done_key(&mut self, key: KeyEvent) {
        if matches!(
            key.code,
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') | KeyCode::Char('q')
        ) {
            self.screen = Screen::Library;
        }
    }

    // ----- Settings -----

    fn settings_key(&mut self, key: KeyEvent, selected: usize) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') => {
                self.stop_background(); // in case a background preview is playing
                self.screen = Screen::Library;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::Settings {
                    selected: (selected + 1).min(SETTINGS_COUNT - 1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::Settings {
                    selected: selected.saturating_sub(1),
                };
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if selected == SETTING_REPLAY {
                    self.stop_background();
                    self.screen = Screen::Tutorial { page: 0 };
                } else {
                    self.change_setting(selected, 1);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => self.change_setting(selected, 1),
            KeyCode::Left | KeyCode::Char('h') => self.change_setting(selected, -1),
            _ => {}
        }
    }

    /// Changes one setting (`step` is 1 or -1 for volume and background),
    /// saves, and plays a preview so you can hear the difference.
    fn change_setting(&mut self, row: usize, step: isize) {
        let settings = &mut self.data.settings;
        match row {
            SETTING_TOUR => settings.show_tutorial = !settings.show_tutorial,
            SETTING_PREFLIGHT => settings.preflight = !settings.preflight,
            SETTING_PATH => settings.follow_path = !settings.follow_path,
            SETTING_CLOCK => settings.clock = settings.clock.cycle(step),
            SETTING_SOUND => settings.sound = !settings.sound,
            SETTING_VOLUME => {
                settings.volume = (settings.volume as isize + step * 10).clamp(10, 100) as u8;
            }
            SETTING_TONES => settings.breath_tones = !settings.breath_tones,
            SETTING_BACKGROUND => settings.background = settings.background.cycle(step),
            _ => return,
        }
        self.persist();

        let volume = self.volume();
        let tones = self.data.settings.breath_tones;
        let background = self.data.settings.background;
        match row {
            SETTING_SOUND if self.data.settings.sound => {
                if let Some(sound) = self.audio() {
                    sound.chime(volume);
                }
            }
            SETTING_SOUND => self.stop_background(),
            SETTING_VOLUME => {
                if let Some(sound) = self.audio_mut() {
                    sound.chime(volume);
                    sound.set_background_volume(volume);
                }
            }
            SETTING_TONES if tones => {
                if let Some(sound) = self.audio() {
                    sound.breath_tone(true, volume);
                }
            }
            SETTING_BACKGROUND => {
                if let Some(sound) = self.audio_mut() {
                    sound.start_background(background, volume);
                }
            }
            _ => {}
        }
    }
}

impl App {
    // ----- Pomodoro -----

    fn pomodoro_setup_key(&mut self, key: KeyEvent, selected: usize) {
        let rows = if self.data.pomodoro.break_mode == BreakMode::Meditation {
            3
        } else {
            2
        };
        let step = match key.code {
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => 1,
            KeyCode::Left | KeyCode::Char('h') => -1,
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::PomodoroSetup {
                    selected: (selected + 1).min(rows - 1),
                };
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.screen = Screen::PomodoroSetup {
                    selected: selected.saturating_sub(1),
                };
                return;
            }
            KeyCode::Enter => {
                self.start_pomodoro();
                return;
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Library;
                return;
            }
            _ => return,
        };
        let settings = &mut self.data.pomodoro;
        match selected {
            POMO_LENGTHS => settings.cycle_preset(step),
            POMO_BREAK => settings.break_mode = settings.break_mode.toggled(),
            POMO_MEDITATION => {
                let names: Vec<&String> = self.library.iter().map(|m| &m.name).collect();
                let current = settings
                    .meditation
                    .as_ref()
                    .and_then(|name| names.iter().position(|n| *n == name));
                let next = match current {
                    Some(index) => {
                        (index as isize + step).rem_euclid(names.len() as isize) as usize
                    }
                    None => 0,
                };
                settings.meditation = names.get(next).map(|name| name.to_string());
            }
            _ => {}
        }
        self.persist();
    }

    /// Starts a pomodoro with the saved settings.
    pub fn start_pomodoro(&mut self) {
        self.pomodoro = Some(Pomodoro::start(&self.data.pomodoro));
        self.screen = Screen::Pomodoro;
        let volume = self.volume();
        if let Some(sound) = self.audio() {
            sound.chime(volume);
        }
    }

    /// The meditation for meditation breaks: the one you picked, or a calm
    /// breathing default.
    pub fn break_meditation(&self) -> Option<usize> {
        let wanted = self.data.pomodoro.meditation.as_deref();
        self.library
            .iter()
            .position(|m| Some(m.name.as_str()) == wanted)
            .or_else(|| {
                self.library
                    .iter()
                    .position(|m| m.name == library::TUTORIAL_PRESET)
            })
    }

    fn pomodoro_key(&mut self, key: KeyEvent) {
        let Some(pomodoro) = &mut self.pomodoro else {
            self.screen = Screen::Library;
            return;
        };

        // Esc asks first; a second Esc stops, Space keeps going.
        if pomodoro.stopping {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('y') => {
                    self.pomodoro = None;
                    self.stop_background();
                    self.screen = Screen::Library;
                }
                KeyCode::Char(' ') | KeyCode::Char('n') | KeyCode::Enter => {
                    pomodoro.stopping = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => pomodoro.stopping = true,
            KeyCode::Char(' ') | KeyCode::Enter => match pomodoro.phase {
                pomodoro::Phase::Work(_) | pomodoro::Phase::Break(_) => {
                    if key.code == KeyCode::Char(' ')
                        && let Some(clock) = pomodoro.clock_mut()
                    {
                        clock.toggle_pause();
                    }
                }
                pomodoro::Phase::WorkDone => self.start_break(),
                pomodoro::Phase::BreakDone => {
                    pomodoro.begin_work();
                    let volume = self.volume();
                    if let Some(sound) = self.audio() {
                        sound.chime(volume);
                    }
                }
                pomodoro::Phase::MeditationBreak => {}
            },
            _ => {}
        }
    }

    fn start_break(&mut self) {
        let Some(pomodoro) = &mut self.pomodoro else {
            return;
        };
        let rest = pomodoro.settings.rest;
        if pomodoro.settings.break_mode == BreakMode::Blank {
            pomodoro.phase = pomodoro::Phase::Break(crate::session::Countdown::start(rest));
            return;
        }
        pomodoro.phase = pomodoro::Phase::MeditationBreak;
        let Some(index) = self.break_meditation() else {
            return;
        };
        self.reveal(index);
        let meditation = self.library[index].clone();
        if self.data.settings.preflight {
            let steps = library::prep_steps(&meditation).len();
            self.screen = Screen::Preflight {
                minutes: rest,
                checked: vec![false; steps],
                selected: 0,
            };
        } else {
            self.begin_session(meditation, rest);
        }
    }

    fn pomodoro_alert(&mut self, event: Event) {
        let volume = self.volume();
        match event {
            Event::SprintDone => {
                self.data.stats.sprints += 1;
                self.persist();
                if let Some(sound) = self.audio() {
                    sound.closing_bell(volume);
                }
                self.desktop_alert("Sprint done. Finish your thought, then take your break.");
            }
            Event::BreakOver => {
                if let Some(sound) = self.audio() {
                    sound.chime(volume);
                }
                self.desktop_alert("Break's over. Ready for the next sprint?");
            }
        }
    }

    /// The terminal bell (so unfocused tmux windows and tabs light up) and a
    /// desktop notification. Off in tests.
    fn desktop_alert(&self, message: &'static str) {
        if !self.desktop_alerts {
            return;
        }
        ring_terminal_bell();
        // Sending a notification talks to the desktop over D-Bus, which can be
        // slow or missing, so it gets its own thread and never holds up the UI.
        std::thread::spawn(move || {
            let _ = notify_rust::Notification::new()
                .summary("meditui")
                .body(message)
                .show();
        });
    }
}

/// The terminal bell (BEL character), used when there's no audio device.
fn ring_terminal_bell() {
    let mut out = std::io::stdout();
    let _ = out.write_all(b"\x07");
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    // No save path and no audio device, so tests never touch your real data
    // file or make a sound.
    fn test_app(data: SaveData) -> App {
        App::new(data, None, None, Vec::new())
    }

    /// Tour already seen, prep checklist off: straight to the point.
    fn returning_user() -> App {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        data.settings.preflight = false;
        test_app(data)
    }

    #[test]
    fn first_launch_opens_the_tour_and_skipping_turns_it_off() {
        let mut app = test_app(SaveData::default());
        assert!(matches!(app.screen, Screen::Tutorial { page: 0 }));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Library));
        assert!(!app.data.settings.show_tutorial);
    }

    #[test]
    fn finishing_the_tour_starts_a_one_minute_try() {
        let mut app = test_app(SaveData::default());
        for _ in 0..tutorial::LAST_PAGE {
            press(&mut app, KeyCode::Right);
        }
        press(&mut app, KeyCode::Enter);
        assert!(!app.data.settings.show_tutorial);
        let Screen::Session { session, .. } = &app.screen else {
            panic!("expected a session");
        };
        assert_eq!(session.meditation.name, library::TUTORIAL_PRESET);
        assert_eq!(session.total(), Duration::from_secs(60));
    }

    #[test]
    fn settings_can_turn_the_tour_back_on() {
        let mut app = returning_user();
        assert!(matches!(app.screen, Screen::Library));
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Enter);
        assert!(app.data.settings.show_tutorial);
    }

    #[test]
    fn sound_settings_change() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Down); // prep checklist
        press(&mut app, KeyCode::Enter);
        assert!(app.data.settings.preflight);
        press(&mut app, KeyCode::Down); // unlocks
        press(&mut app, KeyCode::Enter);
        assert!(!app.data.settings.follow_path);
        press(&mut app, KeyCode::Down); // session clock
        press(&mut app, KeyCode::Right);
        assert_eq!(app.data.settings.clock, storage::ClockStyle::Elapsed);
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.data.settings.clock, storage::ClockStyle::Hidden);
        press(&mut app, KeyCode::Down); // sound
        press(&mut app, KeyCode::Enter);
        assert!(!app.data.settings.sound);
        press(&mut app, KeyCode::Down); // volume
        for _ in 0..20 {
            press(&mut app, KeyCode::Left);
        }
        assert_eq!(app.data.settings.volume, 10);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.data.settings.volume, 20);
        press(&mut app, KeyCode::Down); // breathing tones
        press(&mut app, KeyCode::Enter);
        assert!(!app.data.settings.breath_tones);
        press(&mut app, KeyCode::Down); // background
        press(&mut app, KeyCode::Right);
        assert_eq!(
            app.data.settings.background,
            storage::Background::BrownNoise
        );
        press(&mut app, KeyCode::Down); // replay tour
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Tutorial { page: 0 }));
    }

    #[test]
    fn adding_and_deleting_your_own() {
        let mut app = returning_user();
        let presets = app.library.len();

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "Evening sit");
        press(&mut app, KeyCode::Enter);
        // The length starts at 10; replace it with 25.
        press(&mut app, KeyCode::Backspace);
        press(&mut app, KeyCode::Backspace);
        type_text(&mut app, "25");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.library.len(), presets + 1);
        let added = app.selected().unwrap();
        assert!(added.custom);
        assert_eq!((added.name.as_str(), added.minutes), ("Evening sit", 25));

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(app.library.len(), presets);
        assert!(app.data.custom.is_empty());
    }

    #[test]
    fn bad_lengths_are_rejected() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "x");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Backspace);
        press(&mut app, KeyCode::Backspace);
        type_text(&mut app, "0");
        press(&mut app, KeyCode::Enter);
        assert!(app.form_error.is_some());
        assert!(app.data.custom.is_empty());
    }

    #[test]
    fn builtins_cannot_be_deleted() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('d'));
        assert!(matches!(app.screen, Screen::Library));
        assert!(app.status.is_some());
    }

    #[test]
    fn escape_asks_before_ending_a_session() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Session { ending: true, .. }));
        press(&mut app, KeyCode::Char(' '));
        assert!(matches!(app.screen, Screen::Session { ending: false, .. }));
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(
            app.screen,
            Screen::Done(Summary {
                completed: false,
                ..
            })
        ));
    }

    #[test]
    fn h_hides_the_visuals_and_is_remembered() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('h'));
        assert!(app.data.settings.minimal_view);
        assert!(matches!(app.screen, Screen::Session { ending: false, .. }));
        press(&mut app, KeyCode::Char('h'));
        assert!(!app.data.settings.minimal_view);
    }

    #[test]
    fn office_filter_hides_home_only_practices() {
        let mut app = returning_user();
        let all = app.visible.len();
        press(&mut app, KeyCode::Char('o'));
        assert!(app.visible.len() < all);
        assert!(app.visible.iter().all(|row| match row {
            Row::Item(i) => app.library[*i].tags.contains(&Tag::OfficeSafe),
            Row::Folder(folder) => app.folder_count(*folder) > 0,
        }));
        assert!(app.selected().unwrap().tags.contains(&Tag::OfficeSafe));
        press(&mut app, KeyCode::Char('o'));
        assert_eq!(app.visible.len(), all);
    }

    fn headers(app: &App) -> Vec<Folder> {
        app.visible
            .iter()
            .filter_map(|row| match row {
                Row::Folder(folder) => Some(*folder),
                Row::Item(_) => None,
            })
            .collect()
    }

    #[test]
    fn the_library_is_grouped_into_folders() {
        let app = returning_user();
        assert_eq!(
            app.visible[0],
            Row::Folder(Folder::Category(Category::FocusReset))
        );
        // Every category has a folder; "Your own" only shows up once it has something.
        assert_eq!(headers(&app), Category::ALL.map(Folder::Category));
        // The cursor starts on a meditation, not a header.
        assert!(app.selected().is_some());
        // Each meditation sits under its own folder's header.
        let mut folder = None;
        for row in &app.visible {
            match row {
                Row::Folder(f) => folder = Some(*f),
                Row::Item(i) => assert_eq!(Some(Folder::of(&app.library[*i])), folder),
            }
        }
    }

    #[test]
    fn folders_open_and_close_and_stay_that_way() {
        let mut app = returning_user();
        let all = app.visible.len();
        let first = Folder::Category(Category::FocusReset);
        let inside = app.folder_count(first);

        // Left on a meditation closes its folder and lands on the header.
        press(&mut app, KeyCode::Left);
        assert_eq!(app.selected_row(), Some(Row::Folder(first)));
        assert!(!app.is_open(first));
        assert_eq!(app.visible.len(), all - inside);
        assert!(app.data.settings.closed_folders.contains("focus_reset"));

        // Enter on a header doesn't start anything; it opens the folder.
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Library));
        assert!(app.is_open(first));
        press(&mut app, KeyCode::Char('h'));
        press(&mut app, KeyCode::Right);
        assert!(app.is_open(first));

        // Closed folders are remembered next time.
        press(&mut app, KeyCode::Left);
        let again = test_app(app.data.clone());
        assert!(!again.is_open(first));
        assert_eq!(again.visible.len(), all - inside);
    }

    #[test]
    fn tab_jumps_between_folders() {
        let mut app = returning_user();
        for folder in Category::ALL.into_iter().skip(1) {
            press(&mut app, KeyCode::Tab);
            assert_eq!(
                app.selected_row(),
                Some(Row::Folder(Folder::Category(folder)))
            );
        }
        // Past the last folder, back round to the first.
        press(&mut app, KeyCode::Tab);
        assert_eq!(
            app.selected_row(),
            Some(Row::Folder(Folder::Category(Category::FocusReset)))
        );
        press(&mut app, KeyCode::BackTab);
        assert_eq!(
            app.selected_row(),
            Some(Row::Folder(Folder::Category(Category::DeepUnwind)))
        );
    }

    #[test]
    fn the_office_filter_hides_empty_folders() {
        let mut app = returning_user();
        app.office_only = true;
        app.refresh_visible();
        for folder in headers(&app) {
            assert!(app.folder_count(folder) > 0, "{folder:?}");
        }
        assert!(app.selected().is_some());
    }

    #[test]
    fn an_empty_list_is_harmless() {
        let mut app = returning_user();
        app.library.clear();
        app.refresh_visible();
        assert!(app.selected().is_none());
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down);
        assert!(matches!(app.screen, Screen::Library));
    }

    #[test]
    fn adding_while_filtered_shows_the_new_one() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('o'));
        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "Desk sit");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert!(!app.office_only);
        assert_eq!(app.selected().unwrap().name, "Desk sit");
        assert_eq!(app.current_folder(), Some(Folder::Yours));
    }

    #[test]
    fn a_new_one_opens_your_folder() {
        let mut app = returning_user();
        app.data.settings.closed_folders.insert("yours".to_string());
        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "Desk sit");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert!(app.is_open(Folder::Yours));
        assert_eq!(app.selected().unwrap().name, "Desk sit");
    }

    fn select_named(app: &mut App, name: &str) {
        let index = app.library.iter().position(|m| m.name == name).unwrap();
        app.reveal(index);
        assert_eq!(app.selected().unwrap().name, name);
    }

    #[test]
    fn later_practices_ask_first() {
        let mut app = returning_user();
        select_named(&mut app, "Anuloma Viloma");
        let (rule, days) = app.lock_progress(app.selected().unwrap()).unwrap();
        assert_eq!((rule.after, rule.days, days), ("Nadi Shuddhi", 14, 0));

        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Locked));
        press(&mut app, KeyCode::Char('n'));
        assert!(matches!(app.screen, Screen::Library));

        // Start anyway: just this once.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('y'));
        assert!(matches!(app.screen, Screen::ConfirmStart { .. }));
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Locked));

        // Unlock for good: it never asks again.
        press(&mut app, KeyCode::Char('u'));
        assert!(matches!(app.screen, Screen::ConfirmStart { .. }));
        assert!(app.data.settings.unlocked.contains("Anuloma Viloma"));
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::ConfirmStart { .. }));
    }

    #[test]
    fn practice_days_open_the_next_step() {
        let mut app = returning_user();
        select_named(&mut app, "Nadi Shuddhi");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        app.end_session(true);
        // A second session the same day counts once.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        app.end_session(true);
        assert_eq!(app.data.stats.practice["Nadi Shuddhi"].len(), 1);
        // Ending early doesn't count.
        press(&mut app, KeyCode::Enter);
        select_named(&mut app, "Box Breathing");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        app.end_session(false);
        assert!(!app.data.stats.practice.contains_key("Box Breathing"));
        press(&mut app, KeyCode::Enter); // back to the library

        let days = &mut app.data.stats.practice.get_mut("Nadi Shuddhi").unwrap();
        for day in 1..14 {
            days.insert(format!("2026-01-{day:02}"));
        }
        select_named(&mut app, "Anuloma Viloma");
        assert!(app.lock_progress(app.selected().unwrap()).is_none());
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::ConfirmStart { .. }));
    }

    #[test]
    fn the_path_can_be_turned_off() {
        let mut app = returning_user();
        app.data.settings.follow_path = false;
        select_named(&mut app, "Bhastrika");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::ConfirmStart { .. }));
    }

    #[test]
    fn the_prep_checklist_comes_before_the_session() {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        let mut app = test_app(data);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        let Screen::Preflight { checked, .. } = &app.screen else {
            panic!("expected the checklist, got {:?}", app.screen);
        };
        assert!(checked.iter().all(|c| !c));

        let steps = library::prep_steps(app.selected().unwrap()).len();
        assert!(steps >= 2);
        press(&mut app, KeyCode::Char(' ')); // tick the first, cursor moves down
        // Tick the last one directly by its number.
        press(
            &mut app,
            KeyCode::Char(char::from_digit(steps as u32, 10).unwrap()),
        );
        let Screen::Preflight {
            checked, selected, ..
        } = &app.screen
        else {
            panic!("left the checklist early");
        };
        assert!(checked[0] && checked[steps - 1]);
        assert_eq!(checked.iter().filter(|c| **c).count(), 2);
        assert_eq!(*selected, 1);

        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Session { .. }));
    }

    #[test]
    fn the_prep_checklist_can_be_skipped_or_backed_out_of() {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        let mut app = test_app(data);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Library));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('s'));
        assert!(matches!(app.screen, Screen::Session { .. }));
    }

    #[test]
    fn finishing_a_session_marks_today() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        app.end_session(true);
        let today = progress::day_key(progress::today());
        assert_eq!(app.data.stats.days.get(&today), Some(&1));
        assert_eq!(
            progress::current_streak(&app.data.stats.days, progress::today()),
            1
        );

        // Ending early counts the time but not the day.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        app.end_session(false);
        assert_eq!(app.data.stats.days.get(&today), Some(&1));
    }

    #[test]
    fn t_opens_your_stats_and_esc_comes_back() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('t'));
        assert!(matches!(app.screen, Screen::Stats));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Library));
    }

    #[test]
    fn recipes_sit_between_the_built_ins_and_your_own() {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        data.custom.push(CustomMeditation {
            name: "Mine".to_string(),
            minutes: 5,
        });
        let recipe = crate::recipes::parse("name = \"From file\"\nminutes = 3").unwrap();
        let mut app = App::new(data, None, None, vec![recipe]);
        let names: Vec<&str> = app
            .library
            .iter()
            .rev()
            .take(2)
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(names, ["Mine", "From file"]);

        // Recipes can't be deleted from the app; your own still can.
        press(&mut app, KeyCode::End);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.selected().unwrap().name, "From file");
        press(&mut app, KeyCode::Char('d'));
        assert!(matches!(app.screen, Screen::Library));
        assert!(app.status.as_deref().unwrap().contains("recipe"));
        press(&mut app, KeyCode::End);
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert!(app.data.custom.is_empty());
        assert_eq!(app.library.last().unwrap().name, "From file");
    }

    fn pomodoro_phase(app: &App) -> &pomodoro::Phase {
        &app.pomodoro
            .as_ref()
            .expect("a pomodoro should be running")
            .phase
    }

    #[test]
    fn a_blank_pomodoro_round_trip() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('p'));
        assert!(matches!(app.screen, Screen::PomodoroSetup { selected: 0 }));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Pomodoro));
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::Work(_)));

        // Run the sprint out.
        app.pomodoro.as_mut().unwrap().phase =
            pomodoro::Phase::Work(crate::session::Countdown::start(0));
        app.tick();
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::WorkDone));
        assert_eq!(app.data.stats.sprints, 1);

        // It waits at 00:00 until you're ready.
        app.tick();
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::WorkDone));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::Break(_)));

        app.pomodoro.as_mut().unwrap().phase =
            pomodoro::Phase::Break(crate::session::Countdown::start(0));
        app.tick();
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::BreakDone));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::Work(_)));

        // Esc asks, Space keeps going, Esc Esc stops.
        press(&mut app, KeyCode::Esc);
        assert!(app.pomodoro.as_ref().unwrap().stopping);
        press(&mut app, KeyCode::Char(' '));
        assert!(!app.pomodoro.as_ref().unwrap().stopping);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert!(app.pomodoro.is_none());
        assert!(matches!(app.screen, Screen::Library));
    }

    #[test]
    fn a_meditation_break_hands_back_to_the_pomodoro() {
        let mut app = returning_user();
        app.data.pomodoro.break_mode = BreakMode::Meditation;
        app.data.pomodoro.meditation = Some("Box Breathing".to_string());
        app.start_pomodoro();
        app.pomodoro.as_mut().unwrap().phase = pomodoro::Phase::WorkDone;

        press(&mut app, KeyCode::Enter);
        let Screen::Session { session, .. } = &app.screen else {
            panic!("expected the break meditation, got {:?}", app.screen);
        };
        assert_eq!(session.meditation.name, "Box Breathing");
        assert_eq!(session.total(), Duration::from_secs(5 * 60));

        app.end_session(true);
        assert!(matches!(app.screen, Screen::Pomodoro));
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::BreakDone));
    }

    #[test]
    fn backing_out_of_a_break_checklist_returns_to_the_pomodoro() {
        let mut data = SaveData::default();
        data.settings.show_tutorial = false;
        data.pomodoro.break_mode = BreakMode::Meditation;
        let mut app = test_app(data);
        app.start_pomodoro();
        app.pomodoro.as_mut().unwrap().phase = pomodoro::Phase::WorkDone;
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Preflight { .. }));
        // With no meditation picked, breaks use the calm default.
        assert_eq!(app.selected().unwrap().name, library::TUTORIAL_PRESET);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Pomodoro));
        assert!(matches!(pomodoro_phase(&app), pomodoro::Phase::WorkDone));
    }

    #[test]
    fn pomodoro_setup_changes_are_saved() {
        let mut app = returning_user();
        press(&mut app, KeyCode::Char('p'));
        press(&mut app, KeyCode::Right);
        assert_eq!((app.data.pomodoro.work, app.data.pomodoro.rest), (40, 10));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Right);
        assert_eq!(app.data.pomodoro.break_mode, BreakMode::Meditation);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Right);
        assert_eq!(
            app.data.pomodoro.meditation.as_deref(),
            Some(app.library[0].name.as_str())
        );
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::Library));
    }

    #[test]
    fn length_can_be_adjusted_before_starting() {
        let mut app = returning_user();
        let default = app.selected().unwrap().minutes;
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Up);
        assert!(matches!(app.screen, Screen::ConfirmStart { minutes } if minutes == default + 6));
        for _ in 0..50 {
            press(&mut app, KeyCode::Down);
        }
        assert!(matches!(app.screen, Screen::ConfirmStart { minutes: 1 }));
    }
}
