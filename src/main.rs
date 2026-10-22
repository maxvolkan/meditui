mod app;
mod art;
mod library;
mod pomodoro;
mod progress;
mod recipes;
mod session;
mod sound;
mod storage;
mod tutorial;
mod ui;

use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyEventKind},
};

use app::App;

// How long to wait for a key before redrawing anyway. 50ms = 20 frames a
// second, enough for the breathing circle to move smoothly.
const TICK_RATE: Duration = Duration::from_millis(50);

const HELP: &str = "\
medi: meditation in your terminal

  medi              open the app
  medi pomo         start a pomodoro (your last work/break lengths)
  medi pomo 25/5    start a pomodoro: 25 min work, 5 min break
  medi recipe       print an example recipe file (see below)
  medi help         show this

Recipes are your own meditations written as TOML files in
~/.config/meditui/recipes/. Try:

  mkdir -p ~/.config/meditui/recipes
  medi recipe > ~/.config/meditui/recipes/slow-box.toml
";

fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut pomodoro = None;
    match args.first().map(String::as_str) {
        None => {}
        Some("pomo" | "pomodoro") => {
            pomodoro = match args.get(1) {
                Some(lengths) => match pomodoro::parse_lengths(lengths) {
                    Ok(lengths) => Some(Some(lengths)),
                    Err(err) => {
                        eprintln!("medi pomo: {err}");
                        std::process::exit(2);
                    }
                },
                None => Some(None),
            };
        }
        Some("recipe") => {
            print!("{}", recipes::EXAMPLE);
            return Ok(());
        }
        Some("help" | "-h" | "--help") => {
            print!("{HELP}");
            return Ok(());
        }
        Some(other) => {
            eprint!("medi: unknown command '{other}'\n\n{HELP}");
            std::process::exit(2);
        }
    }

    let (recipes, problems) = recipes::recipes_dir()
        .map(|dir| recipes::load_dir(&dir))
        .unwrap_or_default();

    // Open the audio device before the screen switches over, so anything the
    // audio system prints can't land on top of the app.
    let sound = sound::Sound::open();
    let mut app = App::new(storage::load(), storage::data_path(), sound, recipes);
    app.desktop_alerts = true;
    // `medi pomo [25/5]` goes straight into a pomodoro (the tour can wait).
    if let Some(lengths) = pomodoro {
        if let Some((work, rest)) = lengths {
            app.data.pomodoro.work = work;
            app.data.pomodoro.rest = rest;
        }
        app.start_pomodoro();
    }
    if let Some(first) = problems.first() {
        app.status = Some(match problems.len() {
            1 => format!("Skipped a recipe. {first}"),
            n => format!("Skipped {n} recipes. {first}"),
        });
    }

    let terminal = ratatui::init();
    let result = run(terminal, &mut app);

    ratatui::restore();

    result
}

fn run(mut terminal: DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.quit {
        //Rendering
        terminal.draw(|f| ui::render(f, app))?;

        //Input handling (only when a key is waiting, so the timer keeps moving)
        if event::poll(TICK_RATE)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }

        //Time passing (finishes sessions even when no key is pressed)
        app.tick();
    }
    Ok(())
}
