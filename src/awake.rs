//! Keeps the screen on during a session, so a screensaver or lock screen
//! doesn't come up while your eyes are closed.

use keepawake::{Builder, KeepAwake};

/// Holds the screen on until it's dropped. Best effort: if nothing will
/// listen, the session just runs without it.
pub struct StayAwake {
    _handles: Vec<KeepAwake>,
}

impl StayAwake {
    pub fn new() -> Self {
        let handles = match ask(true, true) {
            Some(both) => vec![both],
            // On Linux these are two different services and either can be
            // missing: many window managers have no screensaver service, and
            // not every distro has logind. Asking for both separately on
            // Windows would undo the first, so only split when we must.
            None => [ask(true, false), ask(false, true)]
                .into_iter()
                .flatten()
                .collect(),
        };
        Self { _handles: handles }
    }
}

fn ask(display: bool, idle: bool) -> Option<KeepAwake> {
    Builder::default()
        .display(display)
        .idle(idle)
        .reason("Meditation session")
        .app_name("meditui")
        .app_reverse_domain("io.github.maxvolkan.meditui")
        .create()
        .ok()
}
