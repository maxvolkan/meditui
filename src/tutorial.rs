//! The first-run tour. Shown on launch until it's finished or skipped once,
//! and can be turned back on from settings (or replayed with `?`).

pub enum Row {
    Text(&'static str),
    /// A key (or name) on the left, what it does on the right.
    Key(&'static str, &'static str),
    Blank,
}

pub struct Page {
    pub title: &'static str,
    pub rows: &'static [Row],
}

use Row::{Blank, Key, Text};

pub const PAGES: [Page; 7] = [
    Page {
        title: "Welcome",
        rows: &[
            Text("A quiet place in your terminal."),
            Blank,
            Text("This short tour takes about a minute."),
            Text("You can skip it any time with Esc."),
        ],
    },
    Page {
        title: "Finding a meditation",
        rows: &[
            Text("Your library has guided meditations, in folders."),
            Text("New to this? Start with the Start here folder."),
            Blank,
            Key("↑ ↓  or  j k", "move through the list"),
            Key("← →", "close or open a folder"),
            Key("Tab", "jump to the next folder"),
            Key("Enter", "choose one"),
            Blank,
            Text("Dimmed ones (○) usually come after another practice."),
            Text("You can still start them; it just asks first."),
        ],
    },
    Page {
        title: "Kinds of guidance",
        rows: &[
            Key("Guided breathing", "a pattern to follow, with soft tones"),
            Key("Guided cues", "short prompts that change as you go"),
            Key("Gazing practice", "a still point to rest your eyes on"),
            Key("Silent sit", "no words, just you and the clock"),
            Blank,
            Text("The circle slowly breathes in and out; breathe along with it."),
            Text("Letters in the list: O office-safe · E eyes-open · H home-only."),
        ],
    },
    Page {
        title: "Before you start",
        rows: &[
            Text("Choose a length, then tick off a short checklist."),
            Key("← →", "one minute shorter or longer"),
            Key("↑ ↓", "five minutes at a time"),
            Key("s", "skip the checklist"),
            Blank,
            Text("The first time, a walkthrough shows you how,"),
            Text("then eases you into the pace with slow breaths."),
            Key("i", "replay it from the library"),
        ],
    },
    Page {
        title: "During a session",
        rows: &[
            Key("Space", "pause and resume"),
            Key("h", "dim the screen, or bring the circle back"),
            Key("Esc", "end early (it asks first)"),
            Blank,
            Text("Eyes-closed practices start dim, with tones to follow."),
            Text("A singing bowl rings at the start and the end."),
            Text("Volume, tones and background sounds are in settings."),
        ],
    },
    Page {
        title: "Make it yours",
        rows: &[
            Key("a", "add your own meditation"),
            Key("d", "delete one of yours"),
            Key("o", "show only office-safe ones"),
            Key("p", "pomodoro: work sprints with breaks"),
            Key("t", "your stats and a year of practice"),
            Key("s", "settings"),
            Key("?", "replay this tour"),
            Blank,
            Text("Your meditations, settings and time sat"),
            Text("are saved automatically."),
        ],
    },
    Page {
        title: "Try one now",
        rows: &[
            Text("Let's sit for one minute together."),
            Text("Breathe in as the circle grows, out as it shrinks."),
            Blank,
            Key("Enter", "begin"),
            Key("Esc", "skip to your library"),
        ],
    },
];

pub const LAST_PAGE: usize = PAGES.len() - 1;
