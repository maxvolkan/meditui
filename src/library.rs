//! The built-in meditations, and the types that describe any meditation.

use std::f64::consts::{PI, TAU};
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Meditation {
    pub name: String,
    pub description: String,
    pub minutes: u64,
    pub guide: Guide,
    /// Added in the app with `a` (and deletable with `d`).
    pub custom: bool,
    /// Loaded from a recipe file in the config folder.
    pub recipe: bool,
    /// Where it's comfortable to practice, shown as badges.
    pub tags: &'static [Tag],
    /// The kind of shift it's for. `None` for your own meditations.
    pub category: Option<Category>,
    /// A prep checklist of its own, replacing the usual one.
    pub prep: Option<Vec<PrepStep>>,
    /// Ring a soft chime every this many minutes.
    pub interval_bell: Option<u64>,
    /// A practice that usually comes after another one.
    pub unlock: Option<Unlock>,
    /// When it helps, in a few words. Shown in the folder list.
    pub when: Option<&'static str>,
    /// Setup steps for the first-time walkthrough, one screen each. Breathing
    /// patterns then get one practice round with the circle.
    pub walkthrough: Vec<&'static str>,
    /// Done with the eyes closed, so sessions start in the dim view.
    pub eyes_closed: bool,
}

/// "Usually comes after practising `after` on `days` different days."
/// It's a soft lock: you can always start anyway.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unlock {
    pub after: &'static str,
    pub days: u32,
}

/// What a meditation is for, as a change of state. Recipe files use the
/// snake_case names, like `down_regulation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    StartHere,
    FocusReset,
    DownRegulation,
    CognitiveDefusion,
    SensoryRest,
    DeepUnwind,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::StartHere,
        Category::FocusReset,
        Category::DownRegulation,
        Category::CognitiveDefusion,
        Category::SensoryRest,
        Category::DeepUnwind,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::StartHere => "Start here",
            Category::FocusReset => "Focus",
            Category::DownRegulation => "Calm down",
            Category::CognitiveDefusion => "Untangle thoughts",
            Category::SensoryRest => "Rest your senses",
            Category::DeepUnwind => "Wind down",
        }
    }

    /// One line on what it's for, shown when its folder is selected.
    pub fn blurb(self) -> &'static str {
        match self {
            Category::StartHere => {
                "New to this? Learn these first. They're simple and work at a desk."
            }
            Category::FocusReset => "For a scattered mind, or a reset between tasks.",
            Category::DownRegulation => "When you're stressed, frustrated or wound up.",
            Category::CognitiveDefusion => {
                "When a thought has its hooks in you: self-criticism, putting things off, feeling stuck."
            }
            Category::SensoryRest => "Rest your eyes and ears from screens and noise.",
            Category::DeepUnwind => "For the end of the day, or before sleep.",
        }
    }
}

/// Where a meditation is comfortable to practice. Recipe files use the
/// snake_case names, like `office_safe`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// Nothing anyone nearby would notice: no odd postures, sounds or mudras.
    OfficeSafe,
    /// The screen is the anchor, so there's no need to close your eyes.
    EyesOpen,
    /// Noticeable movements, sounds or hand positions: best done at home.
    HomeOnly,
}

impl Tag {
    pub fn label(self) -> &'static str {
        match self {
            Tag::OfficeSafe => "office-safe",
            Tag::EyesOpen => "eyes-open",
            Tag::HomeOnly => "home-only",
        }
    }

    /// One letter for the library list: O, E or H.
    pub fn letter(self) -> char {
        match self {
            Tag::OfficeSafe => 'O',
            Tag::EyesOpen => 'E',
            Tag::HomeOnly => 'H',
        }
    }
}

/// What the session screen shows while you sit.
#[derive(Debug, Clone)]
pub enum Guide {
    /// A breathing pattern that repeats for the whole session.
    Breath(Vec<Phase>),
    /// Short cues, spread evenly across the session.
    Prompts(Vec<&'static str>),
    /// A still point to rest your eyes on, with cues spread across the session.
    Gaze(Vec<&'static str>),
    /// No cues: just the breathing circle and the clock.
    Silent,
}

/// How the breathing circle moves during a phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    Grow,
    Full,
    Shrink,
    Empty,
    /// Moves between two sizes (0.0 to 1.0), for breaths that fill in
    /// stages or never fully empty.
    Partial {
        from: f64,
        to: f64,
    },
    /// Breathe however you like; the circle drifts gently on its own.
    Rest,
}

impl Motion {
    /// `Some(true)` for an in-breath, `Some(false)` for an out-breath,
    /// `None` for a hold.
    pub fn rising(self) -> Option<bool> {
        match self {
            Motion::Grow => Some(true),
            Motion::Shrink => Some(false),
            Motion::Partial { from, to } if to > from => Some(true),
            Motion::Partial { from, to } if to < from => Some(false),
            Motion::Partial { .. } | Motion::Full | Motion::Empty | Motion::Rest => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Phase {
    pub label: &'static str,
    pub seconds: f64,
    pub motion: Motion,
}

/// Where you are in a breathing pattern at a given moment.
#[derive(Debug, Clone, Copy)]
pub struct BreathState {
    /// Which phase of the pattern (0, 1, 2...). Changes when the breath turns.
    pub index: usize,
    pub motion: Motion,
    pub label: &'static str,
    /// 0.0 = smallest circle, 1.0 = biggest.
    pub size: f64,
    pub seconds_left: f64,
    /// How long this whole phase lasts. Very short ones (fast breathing)
    /// get no countdown and no tone.
    pub phase_seconds: f64,
}

impl Meditation {
    pub fn custom(name: String, minutes: u64) -> Self {
        Self {
            name,
            description: "One of yours: a quiet sit with the breathing circle. \
                          Practice whatever you set out to."
                .to_string(),
            minutes,
            guide: Guide::Silent,
            custom: true,
            recipe: false,
            tags: &[],
            category: None,
            prep: None,
            interval_bell: None,
            unlock: None,
            when: None,
            walkthrough: Vec::new(),
            eyes_closed: false,
        }
    }

    /// Whether there's anything to walk through: setup steps, or a
    /// breathing pattern to try once.
    pub fn has_walkthrough(&self) -> bool {
        !self.walkthrough.is_empty() || matches!(self.guide, Guide::Breath(_))
    }

    /// What step `step` of the walkthrough is: the setup steps, then one
    /// practice round for breathing patterns, then the end.
    pub fn walkthrough_stage(&self, step: usize) -> Stage {
        let setup = self.walkthrough.len();
        if step < setup {
            Stage::Setup(self.walkthrough[step])
        } else if step == setup && matches!(self.guide, Guide::Breath(_)) {
            Stage::Round
        } else {
            Stage::Done
        }
    }

    fn tagged(mut self, tags: &'static [Tag]) -> Self {
        self.tags = tags;
        self
    }

    fn when(mut self, when: &'static str) -> Self {
        self.when = Some(when);
        self
    }

    fn walkthrough(mut self, steps: &[&'static str]) -> Self {
        self.walkthrough = steps.to_vec();
        self
    }

    fn eyes_closed(mut self) -> Self {
        self.eyes_closed = true;
        self
    }

    fn unlocks_after(mut self, after: &'static str, days: u32) -> Self {
        self.unlock = Some(Unlock { after, days });
        self
    }
}

impl Guide {
    pub fn kind(&self) -> &'static str {
        match self {
            Guide::Breath(_) => "Guided breathing",
            Guide::Prompts(_) => "Guided cues",
            Guide::Gaze(_) => "Gazing practice",
            Guide::Silent => "Silent sit",
        }
    }

    pub fn cues(&self) -> Option<&[&'static str]> {
        match self {
            Guide::Prompts(cues) | Guide::Gaze(cues) => Some(cues),
            Guide::Breath(_) | Guide::Silent => None,
        }
    }

    /// "In 4 · Hold 4 · Out 4 · Hold 4" for breathing patterns. Long runs of
    /// the same in/out pair are squeezed: "(In 0.75 · Out 0.75) ×20".
    pub fn pattern_summary(&self) -> Option<String> {
        let Guide::Breath(phases) = self else {
            return None;
        };
        let parts: Vec<String> = phases
            .iter()
            .map(|p| {
                let word = match (p.motion, p.motion.rising()) {
                    (Motion::Rest, _) => "Rest",
                    (_, Some(true)) => "In",
                    (_, Some(false)) => "Out",
                    (_, None) => "Hold",
                };
                format!("{word} {}", format_seconds(p.seconds))
            })
            .collect();

        let mut out = Vec::new();
        let mut i = 0;
        while i < parts.len() {
            let mut repeats = 1;
            while i + 2 * (repeats + 1) <= parts.len()
                && parts[i..i + 2] == parts[i + 2 * repeats..i + 2 * repeats + 2]
            {
                repeats += 1;
            }
            if repeats >= 3 {
                out.push(format!("({} · {}) ×{repeats}", parts[i], parts[i + 1]));
                i += 2 * repeats;
            } else {
                out.push(parts[i].clone());
                i += 1;
            }
        }
        Some(out.join(" · "))
    }
}

/// 4.0 -> "4", 5.5 -> "5.5", 0.75 -> "0.75"
fn format_seconds(seconds: f64) -> String {
    let text = format!("{seconds:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Which phase of the pattern we're in after `elapsed`, and how big the circle is.
pub fn breath_at(phases: &[Phase], elapsed: Duration) -> BreathState {
    let cycle: f64 = phases.iter().map(|p| p.seconds).sum();
    if phases.is_empty() || cycle <= 0.0 {
        return BreathState {
            index: 0,
            motion: Motion::Empty,
            label: "",
            size: 0.0,
            seconds_left: 0.0,
            phase_seconds: 0.0,
        };
    }

    let mut t = elapsed.as_secs_f64() % cycle;
    for (index, phase) in phases.iter().enumerate() {
        if t < phase.seconds {
            let progress = t / phase.seconds;
            // Ease in and out so the circle starts and stops gently.
            let eased = (1.0 - (progress * PI).cos()) / 2.0;
            let size = match phase.motion {
                Motion::Grow => eased,
                Motion::Full => 1.0,
                Motion::Shrink => 1.0 - eased,
                Motion::Empty => 0.0,
                Motion::Partial { from, to } => from + (to - from) * eased,
                // A slow, small natural breath, one every 10 seconds.
                Motion::Rest => 0.3 + 0.2 * (TAU * t / 10.0).sin(),
            };
            return BreathState {
                index,
                motion: phase.motion,
                label: phase.label,
                size,
                seconds_left: phase.seconds - t,
                phase_seconds: phase.seconds,
            };
        }
        t -= phase.seconds;
    }

    // Only reachable through floating point rounding at the very end of a cycle.
    let last = phases[phases.len() - 1];
    BreathState {
        index: phases.len() - 1,
        motion: last.motion,
        label: last.label,
        size: match last.motion {
            Motion::Grow | Motion::Full => 1.0,
            Motion::Shrink | Motion::Empty => 0.0,
            Motion::Partial { to, .. } => to,
            Motion::Rest => 0.3,
        },
        seconds_left: 0.0,
        phase_seconds: last.seconds,
    }
}

/// Breath phases shorter than this are fast breathing (bellows, breath of
/// fire): no tone and no countdown.
pub const MIN_TONE_SECONDS: f64 = 2.0;

/// How a walkthrough eases into a breathing pattern: the first breaths are
/// long, and they speed up until the last time through is at the real pace.
/// Holds and rests are never stretched. Fast breathing is cut to 10 breaths
/// and rests to 8 seconds, so it stays quick.
pub fn warm_up(phases: &[Phase]) -> Vec<Phase> {
    let mut fast = 0;
    let mut cycle = Vec::new();
    for &phase in phases {
        if phase.seconds < MIN_TONE_SECONDS {
            fast += 1;
            if fast > 20 {
                continue;
            }
        }
        let seconds = match phase.motion {
            Motion::Rest => phase.seconds.min(8.0),
            _ => phase.seconds,
        };
        cycle.push(Phase { seconds, ..phase });
    }

    let moves = |phase: &Phase| phase.motion.rising().is_some();
    let per_cycle = cycle.iter().filter(|p| moves(p)).count().max(1);
    // Breaths to get from (up to) double length down to the real pace.
    let ramp: usize = if fast > 0 { 10 } else { 4 };
    let times = if fast > 0 {
        1
    } else {
        ramp.div_ceil(per_cycle) + 1
    };

    let mut breaths = 0;
    let mut out = Vec::new();
    for _ in 0..times {
        for &phase in &cycle {
            let mut phase = phase;
            if moves(&phase) {
                let slow = 1.0 - breaths as f64 / ramp as f64;
                if slow > 0.0 {
                    phase.seconds += (phase.seconds * slow).min(4.0);
                }
                breaths += 1;
            }
            out.push(phase);
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    Setup(&'static str),
    Round,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// The nostril a phase is about, for labels that say LEFT or RIGHT in
/// capitals.
pub fn side(label: &str) -> Option<Side> {
    label
        .split(|c: char| !c.is_alphabetic())
        .find_map(|word| match word {
            "LEFT" => Some(Side::Left),
            "RIGHT" => Some(Side::Right),
            _ => None,
        })
}

/// Which cue to show: the session is split into equal slices, one per cue.
pub fn prompt_index(count: usize, elapsed: Duration, total: Duration) -> usize {
    if count == 0 || total.is_zero() {
        return 0;
    }
    let fraction = elapsed.as_secs_f64() / total.as_secs_f64();
    ((fraction * count as f64) as usize).min(count - 1)
}

const fn phase(label: &'static str, seconds: f64, motion: Motion) -> Phase {
    Phase {
        label,
        seconds,
        motion,
    }
}

/// A phase that moves the circle between two sizes.
const fn part(label: &'static str, seconds: f64, from: f64, to: f64) -> Phase {
    Phase {
        label,
        seconds,
        motion: Motion::Partial { from, to },
    }
}

/// `pattern` repeated `times` times, for rounds of fast breathing.
fn rounds(pattern: &[Phase], times: usize) -> Vec<Phase> {
    pattern.repeat(times)
}

fn preset(
    category: Category,
    name: &str,
    minutes: u64,
    description: &str,
    guide: Guide,
) -> Meditation {
    Meditation {
        category: Some(category),
        name: name.to_string(),
        description: description.to_string(),
        minutes,
        guide,
        custom: false,
        recipe: false,
        tags: &[],
        prep: None,
        interval_bell: None,
        unlock: None,
        when: None,
        walkthrough: Vec::new(),
        eyes_closed: false,
    }
}

/// One item on the pre-session checklist: a short title and what to do.
pub type PrepStep = (&'static str, &'static str);

/// The 2-3 things worth doing before this particular meditation starts.
pub fn prep_steps(meditation: &Meditation) -> Vec<PrepStep> {
    if let Some(steps) = &meditation.prep {
        return steps.clone();
    }
    let mut steps = vec![(
        "Chair & posture",
        "Scoot to the front edge of your chair, feet flat, knees a little below \
         your hips. Take your hands off the keyboard and mouse.",
    )];
    let breathing = matches!(meditation.guide, Guide::Breath(_))
        || meditation.category == Some(Category::DownRegulation);
    if breathing {
        steps.push((
            "Breath",
            "Blow your nose, unclench your jaw, and loosen anything tight around \
             your waist so the belly can move.",
        ));
    }
    let eyes_open =
        matches!(meditation.guide, Guide::Gaze(_)) || meditation.tags.contains(&Tag::EyesOpen);
    if eyes_open {
        steps.push((
            "Eyes",
            "Sit about an arm's length back from the screen and let your gaze soften.",
        ));
    }
    if steps.len() < 3 {
        steps.push(if meditation.category == Some(Category::SensoryRest) {
            (
                "Sound",
                "Silence notifications. This one is about the sounds around you, so \
                 take headphones off or turn noise cancelling off.",
            )
        } else {
            (
                "Quiet",
                "Silence notifications and chat. Headphones with noise cancelling \
                 help if it's loud around you.",
            )
        });
    }
    steps
}

/// A calm, natural rhythm for the pulsing circle in sessions that aren't
/// breathing exercises (guided cues and silent sits).
pub const RESTING_BREATH: [Phase; 2] = [
    phase("Breathe in", 4.5, Motion::Grow),
    phase("Breathe out", 5.5, Motion::Shrink),
];

/// The name of the preset the tutorial uses for its one-minute try.
pub const TUTORIAL_PRESET: &str = "Coherent Breathing";

pub fn presets() -> Vec<Meditation> {
    use Motion::*;

    let mut list = vec![
        // ----- Start here -----
        preset(
            Category::StartHere,
            "Nadi Shuddhi",
            5,
            "Alternate nostril breathing, and the first of three steps. Right \
             hand up: the thumb closes the right nostril, the ring and little \
             fingers the left. Switch sides at the top of each in-breath, while \
             the lungs are full. No holds. Start with 3 to 5 minutes and build \
             up to 10 or more.",
            Guide::Breath(vec![
                phase("Breathe in LEFT", 4.0, Grow),
                phase("Switch · breathe out RIGHT", 6.0, Shrink),
                phase("Breathe in RIGHT", 4.0, Grow),
                phase("Switch · breathe out LEFT", 6.0, Shrink),
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("Daily. The one the others build on.")
        .walkthrough(&[
            "Raise your right hand. Your thumb closes the right nostril and your \
             ring finger closes the left.",
            "You switch sides at the top of each breath in. The arrow next to the \
             circle shows which side to use.",
            "The hand position can feel awkward at first. Keeping track of it \
             helps pull your attention away from your thoughts.",
            "It's normal for one nostril to be more blocked than the other, and \
             the blocked side changes through the day.",
        ])
        .eyes_closed(),
        preset(
            Category::StartHere,
            TUTORIAL_PRESET,
            5,
            "Slow, even breathing at about five and a half breaths a minute. No \
             holds, just a gentle wave in and out.",
            Guide::Breath(vec![
                phase("Breathe in", 5.5, Grow),
                phase("Breathe out", 5.5, Shrink),
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Any time. The easiest one.")
        .walkthrough(&[
            "Breathe in as the circle grows.",
            "Breathe out as it shrinks.",
        ]),
        preset(
            Category::StartHere,
            "Box Breathing",
            4,
            "Breathe in, hold, breathe out, hold: four counts each. A simple, \
             steadying pattern for when you feel scattered. Follow the circle.",
            Guide::Breath(vec![
                phase("Breathe in", 4.0, Grow),
                phase("Hold", 4.0, Full),
                phase("Breathe out", 4.0, Shrink),
                phase("Hold", 4.0, Empty),
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Before a call or demo.")
        .walkthrough(&["Breathe in, hold, breathe out, hold.", "Follow the circle."]),
        preset(
            Category::StartHere,
            "Following the Breath",
            5,
            "A curious practice rather than a focusing one: find out, by feel \
             alone, where the breath actually goes. Forget what you know about \
             lungs. If your attention stays on the search, you're doing it right.",
            Guide::Prompts(vec![
                "Close your eyes. For a few breaths, notice the temperature of the air.",
                "Is it the same on the way in as on the way out? Trust what you feel.",
                "Now, where do you feel it? The tip of the nose, inside the nose, the throat?",
                "Follow it inward, as far as you can still feel it.",
                "At the edge where it fades, look one step further.",
                "Lost the trail? No problem. Pick it up again at the nose.",
                "You can also check a spot: does the breath reach the chest? The belly?",
                "Let go of the search. Stretch, let thoughts back in, open your eyes.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("When your head feels crowded.")
        .walkthrough(&[
            "Close your eyes.",
            "Breathe normally. Don't control it.",
            "Find where you feel the breath, and follow it in.",
        ])
        .eyes_closed(),
        preset(
            Category::DownRegulation,
            "4-7-8 Breathing",
            3,
            "In for 4, hold for 7, out slowly for 8. A relaxing pattern that many \
             people use before sleep. If you feel lightheaded, go back to normal \
             breathing for a while.",
            Guide::Breath(vec![
                phase("Breathe in through the nose", 4.0, Grow),
                phase("Hold", 7.0, Full),
                phase("Breathe out slowly through the mouth", 8.0, Shrink),
            ]),
        )
        .when("Wound up, or before sleep.")
        .walkthrough(&[
            "In through your nose for 4.",
            "Hold for 7.",
            "Out through your mouth for 8.",
        ]),
        preset(
            Category::DeepUnwind,
            "Body Scan",
            10,
            "Move your attention slowly from your feet to the top of your head. \
             You're not trying to change anything, just noticing what's there.",
            Guide::Prompts(vec![
                "Settle in. Feel the weight of your body being supported.",
                "Bring attention to your feet. Notice any sensation, or none.",
                "Move up to your ankles and calves.",
                "Your knees and thighs.",
                "Your hips and lower back. Let them be heavy.",
                "Your belly, rising and falling with the breath.",
                "Your chest and upper back.",
                "Your hands, then your arms, all the way up.",
                "Your shoulders and neck. Let them soften.",
                "Your jaw, your eyes, your forehead.",
                "The top of your head.",
                "Now the whole body at once, breathing.",
            ]),
        )
        .when("Tense without knowing it.")
        .walkthrough(&[
            "Sit or lie down, and close your eyes.",
            "Move your attention from your feet to your head.",
            "Just notice. No need to relax anything.",
        ])
        .eyes_closed(),
        preset(
            Category::DeepUnwind,
            "Loving-Kindness",
            10,
            "Metta practice. Silently offer simple wishes of goodwill, starting \
             with yourself and widening out to everyone.",
            Guide::Prompts(vec![
                "Bring yourself to mind, just as you are right now.",
                "Silently: may I be safe. May I be happy. May I be at ease.",
                "Picture someone you love.",
                "May you be safe. May you be happy. May you be at ease.",
                "Picture someone neutral: a neighbor, a cashier.",
                "May you be safe. May you be happy. May you be at ease.",
                "Picture someone you find difficult. Go gently.",
                "May you be safe. May you be happy. May you be at ease.",
                "Now all beings, everywhere.",
                "May all beings be safe, happy, and at ease.",
            ]),
        )
        .when("Frustrated with someone.")
        .walkthrough(&[
            "Silently wish someone well.",
            "Yourself first, then others.",
        ])
        .eyes_closed(),
        preset(
            Category::DownRegulation,
            "5-4-3-2-1 Grounding",
            3,
            "A quick way back to the present when your mind is racing. Use your \
             senses, one at a time.",
            Guide::Prompts(vec![
                "Pause. Look around you.",
                "Name 5 things you can see.",
                "Notice 4 things you can feel: your feet, your clothes, the air.",
                "Listen for 3 things you can hear.",
                "Notice 2 things you can smell.",
                "Notice 1 thing you can taste.",
                "Take one slow breath. You're here.",
            ]),
        )
        .when("Racing mind.")
        .walkthrough(&[
            "You'll use your senses, one at a time.",
            "The cues take you through it.",
        ]),
        // ----- Focus -----
        preset(
            Category::FocusReset,
            "Trataka",
            5,
            "Steady gazing at a single point. Anything works; a candle flame two or \
             three metres away in a still room is the classic choice, and here \
             it's the dot on the screen. First you gaze without blinking, then \
             you close your eyes and hold the afterimage. Watering eyes are fine, \
             but never push through pain: blink whenever you need to.",
            Guide::Gaze(vec![
                "Sit up straight with your eyes closed for a moment.",
                "Open them and rest your gaze on the point. Keep the face soft.",
                "Try not to blink. Stinging or watering is normal. Blink if it hurts.",
                "Thoughts come and go. The point stays where it is.",
                "Let the point fill your attention.",
                "Close your eyes. Find the afterimage and keep it in the centre.",
                "Relax and it gets clearer; drift off and it breaks up. Stay with it.",
                "Rub your palms warm, cup them over your eyes, then open your eyes.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Before deep work.")
        .walkthrough(&[
            "Sit an arm's length from the screen.",
            "Rest your eyes on the dot. Try not to blink.",
            "Blink whenever it hurts.",
        ]),
        preset(
            Category::FocusReset,
            "Kaya Sthairyam",
            5,
            "Body stillness. Sit with a straight spine, make one last adjustment, \
             then don't move at all until the bell: no fidgeting, no scratching. \
             The only thing that moves is the breath.",
            Guide::Prompts(vec![
                "Spine and neck straight, shoulders loose. Close your eyes.",
                "Make one last adjustment. From now on, nothing moves.",
                "Notice what the body is telling you: weight, pressure, warmth.",
                "An itch, an urge to shift? Notice it. Don't obey it.",
                "Only the breath moves. Let it.",
                "When it gets hard, you may hide in the breath. Then come back to the body.",
                "A few slow, deep breaths. Notice the relief in each one.",
                "Stay perfectly still until the bell. Then enjoy moving again.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("Restless and fidgety.")
        .walkthrough(&[
            "Sit up straight and close your eyes.",
            "Make one last adjustment.",
            "Then don't move until the bell.",
        ])
        .eyes_closed(),
        preset(
            Category::FocusReset,
            "Thought Control",
            8,
            "Two drills for taking charge of your attention. First, simply notice \
             each thought as it arrives. Then, with eyes open, use what you see \
             to call up thoughts on purpose, one object at a time. Each drill is \
             worth 5 to 10 minutes on at least three days before you move on.",
            Guide::Prompts(vec![
                "Part 1 · Close your eyes. Notice each thought as it shows up.",
                "Name it lightly: \"Now I'm thinking about lunch.\"",
                "Caught up in a chain of thoughts? Noticing that is the practice.",
                "Bored? That's fine. Keep noticing.",
                "Part 2 · Open your eyes. Look at one object near you.",
                "Notice the thought it sparks, not the object itself.",
                "When your thoughts wander off the object, move to a new one.",
                "Don't linger. Object, thought, next object.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Mind keeps wandering off task.")
        .walkthrough(&[
            "First, eyes closed: notice each thought.",
            "Then, eyes open: let objects spark thoughts.",
        ]),
        preset(
            Category::FocusReset,
            "Urge Surfing",
            4,
            "For the moment you want to switch tabs, check your phone or grab a \
             snack. Giving in once an urge peaks teaches it to come back \
             stronger. Instead, give it your full attention and ride it out: \
             urges rise, crest and fade like waves.",
            Guide::Prompts(vec![
                "Name the urge: a new tab, your phone, a snack, something else.",
                "Give it your full attention. Where do you feel it in the body?",
                "Don't fight it and don't feed it. Just watch.",
                "Is it growing or shrinking right now?",
                "You don't have to resist forever. Only for this moment.",
                "Lean on the breath while you ride it.",
                "Check again: bigger, smaller, or changing shape?",
                "Notice it fading. Each wave you ride makes the next one weaker.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Itching to switch tabs.")
        .walkthrough(&[
            "Name the urge you're feeling.",
            "Watch it rise, peak and fade.",
            "Don't fight it. Don't give in.",
        ]),
        // ----- Calm down -----
        preset(
            Category::DownRegulation,
            "Anuloma Viloma",
            5,
            "The second step of alternate nostril breathing. Hands rest in your \
             lap: there's no blocking this time. Instead, you move your \
             attention: feel the air in the left nostril as you breathe in, the \
             right as you breathe out, then the other way round. It sharpens \
             your feel for subtle sensations. A good routine is a few minutes of \
             Nadi Shuddhi first, then this.",
            Guide::Breath(vec![
                phase("In · feel the LEFT nostril", 4.0, Grow),
                phase("Out · feel the RIGHT nostril", 6.0, Shrink),
                phase("In · feel the RIGHT nostril", 4.0, Grow),
                phase("Out · feel the LEFT nostril", 6.0, Shrink),
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .unlocks_after("Nadi Shuddhi", 14)
        .when("Once Nadi Shuddhi feels easy.")
        .walkthrough(&[
            "Rest your hands in your lap.",
            "No fingers this time. Just your attention.",
            "Feel the air in one nostril at a time.",
        ])
        .eyes_closed(),
        preset(
            Category::DownRegulation,
            "Prana Shuddhi",
            5,
            "The third step of alternate nostril breathing. Breathe through both \
             nostrils and follow the cool air as two streams that rise and meet \
             at the point between the eyebrows. Breathe out from that point. If \
             you can't feel it yet, give the first two steps more time.",
            Guide::Breath(vec![
                phase("In · both streams rise to the brow", 5.0, Grow),
                phase("Out · from the brow, back out", 6.0, Shrink),
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .unlocks_after("Anuloma Viloma", 14)
        .when("Once Anuloma Viloma feels easy.")
        .walkthrough(&[
            "Rest your hands in your lap.",
            "Breathe through both nostrils.",
            "Feel the air rise to between your eyebrows.",
        ])
        .eyes_closed(),
        preset(
            Category::DownRegulation,
            "Three-Part Breath",
            5,
            "Fill the lungs from the bottom up, in three stages: belly, then \
             ribs, then the top of the chest, letting the collarbones lift a \
             little. Then let it all go at once. Feeling lightheaded is normal; \
             stop if it's uncomfortable. Later, try breathing out in reverse \
             order: chest, ribs, belly.",
            Guide::Breath(vec![
                part("In · belly", 2.0, 0.0, 0.33),
                part("In · ribs", 2.0, 0.33, 0.66),
                part("In · collarbones", 2.0, 0.66, 1.0),
                phase("Out · let it all go", 6.0, Shrink),
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("After hours hunched at a desk.")
        .walkthrough(&[
            "Breathe into your belly first.",
            "Then your ribs, then your chest.",
            "Then let it all go.",
        ]),
        preset(
            Category::DownRegulation,
            "Ujjayi",
            5,
            "Ocean breath. Narrow the back of the throat a little so the breath \
             hisses softly on the way in and on the way out. It slows the breath \
             down by itself. Start with about 9 breaths and work up to 33. After \
             a few weeks you can curl the tongue back against the roof of the \
             mouth as well. Audible, so best done at home.",
            Guide::Breath(vec![
                phase("In · soft hiss in the throat", 5.0, Grow),
                phase("Out · the same soft hiss", 7.0, Shrink),
            ]),
        )
        .tagged(&[Tag::HomeOnly])
        .when("To slow your breathing down.")
        .walkthrough(&[
            "Narrow the back of your throat a little.",
            "Like fogging a mirror, with your mouth closed.",
            "Breathe through your nose with a soft hiss.",
        ])
        .eyes_closed(),
        preset(
            Category::DownRegulation,
            "Kapalbhati",
            3,
            "Short, sharp out-breaths made by snapping the belly in; the \
             in-breath happens by itself. After each round, one slow deep breath \
             and a rest while the breath calms down on its own, which is the \
             point. Rounds of 30 here; if that's a lot, do fewer pumps at your \
             own pace and build up. Good for an anxious, racing mind. Practice \
             on an empty stomach. Skip it if you're pregnant or have high blood \
             pressure, heart problems, a hernia or recent belly surgery. Stop if \
             you feel dizzy.",
            Guide::Breath({
                let mut round = rounds(
                    &[
                        part("Snap out", 0.4, 0.8, 0.45),
                        part("Let it in", 0.6, 0.45, 0.8),
                    ],
                    30,
                );
                round.extend([
                    phase("One slow, deep breath in", 5.0, Grow),
                    phase("And out", 6.0, Shrink),
                    phase("Rest · let the breath slow down", 20.0, Rest),
                ]);
                round
            }),
        )
        .tagged(&[Tag::HomeOnly])
        .when("Anxious, racing mind.")
        .walkthrough(&[
            "Skip it if pregnant, or with high blood pressure or heart problems.",
            "Snap your belly in to push air out your nose.",
            "Let the breath in happen by itself.",
        ]),
        preset(
            Category::DownRegulation,
            "Bhastrika",
            2,
            "Bellows breath: a forceful breath in and a forceful breath out, \
             about a second each, driven by the belly. Rounds of 9, with a quiet \
             rest after each one while your heart settles. Energising rather \
             than calming. Get comfortable with Kapalbhati first. Skip it if \
             you're pregnant or have high blood pressure, heart or lung \
             problems. Stop if you feel dizzy.",
            Guide::Breath({
                let mut round = rounds(
                    &[
                        phase("Bellows · in", 1.0, Grow),
                        phase("Bellows · out", 1.0, Shrink),
                    ],
                    9,
                );
                round.push(phase("Rest · let your heart settle", 20.0, Rest));
                round
            }),
        )
        .tagged(&[Tag::HomeOnly])
        .unlocks_after("Kapalbhati", 14)
        .when("When you need energy.")
        .walkthrough(&[
            "Skip it if pregnant, or with blood pressure, heart or lung problems.",
            "Breathe hard in and out through your nose.",
            "Drive it with your belly, like a bellows.",
        ]),
        // ----- Untangle thoughts -----
        preset(
            Category::CognitiveDefusion,
            "Defusing \"I Am\"",
            6,
            "A defusion drill, as in Acceptance and Commitment Therapy. Catch \
             thoughts that glue a label to you (\"I am a bad programmer\") and \
             restate them as what they are: \"My mind is telling me I'm a bad \
             programmer.\" Your mind says all kinds of things, and that doesn't \
             make them true.",
            Guide::Prompts(vec![
                "Notice a harsh thought about yourself. Something that starts \"I am...\".",
                "Say it silently, exactly as it shows up: \"I am a bad programmer.\"",
                "Restate it: \"My mind is telling me I'm a bad programmer.\"",
                "Now look for \"bad programmer\". Can you point to it, or measure it?",
                "What actually happened is just what happened. The label was added.",
                "Find another one. \"I am behind.\" \"I am not good at this.\"",
                "Restate it the same way: \"My mind is telling me...\"",
                "Thoughts, not truths. Let this one sit there unargued.",
                "Come back to the breath. You're the one doing the noticing.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Being hard on yourself.")
        .walkthrough(&[
            "Catch a harsh thought about yourself.",
            "Restate it: \"My mind is telling me...\"",
        ]),
        preset(
            Category::CognitiveDefusion,
            "Let It Be",
            5,
            "For procrastination. When you're putting something off, the mind \
             comes up with good-sounding reasons (\"it'll be quicker tomorrow\"). \
             Arguing back only makes it dig in. So don't argue: notice the \
             reason, let it be, and do the thing anyway.",
            Guide::Prompts(vec![
                "Bring to mind the task you've been putting off.",
                "Notice what your mind says about it. Just listen.",
                "\"Later.\" \"I'll be faster tomorrow.\" Notice each reason as it arrives.",
                "Don't argue with it. Arguing makes it push back harder.",
                "Breathe, step back, and answer: \"Maybe so. Let it be.\"",
                "Inefficient? Let it be. Awkward? Let it be.",
                "Picture the very first, smallest step of the task.",
                "The reasons can come along. You can start anyway.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("Putting something off.")
        .walkthrough(&[
            "Think of the task you're avoiding.",
            "Notice the excuses. Don't argue.",
            "\"Maybe so. Let it be.\"",
        ]),
        preset(
            Category::CognitiveDefusion,
            "Mind and Matter",
            8,
            "Nama-rupa contemplation from the Visuddhimagga: separate what's \
             really there (colour, texture, sound, words) from what your mind \
             adds to it (like, dislike, good, bad, a story). Start with dull \
             objects and work up to ones with more charge.",
            Guide::Prompts(vec![
                "Pick a plain object near you: a mug, the floor, a wall.",
                "What's actually there? Colour, shape, texture, weight.",
                "Now what does your mind add? Like it, dislike it, good, bad?",
                "Notice: the thing is out there. The liking is in here.",
                "Now something with a bit more charge: your phone, an unread message.",
                "Again: what's really there? Then, what are you adding?",
                "Boring isn't in the task, and fun isn't in the game. Look and see.",
                "Go by what you notice, not by what you think you should notice.",
                "Rest. A thing, and your reaction to it. Two different things.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("A task feels dreadful.")
        .walkthrough(&[
            "Look at something near you.",
            "Notice what's really there.",
            "Then notice what your mind adds.",
        ]),
        preset(
            Category::CognitiveDefusion,
            "Neti Neti",
            7,
            "\"Not this, not this.\" A practice from Advaita Vedanta: anything \
             about you that you can point to or put into words (a job, a role, \
             a skill, a feeling, a thought) isn't the one who's aware of it. \
             Keep stepping back.",
            Guide::Prompts(vec![
                "Settle. Feel the body sitting here.",
                "Notice the body's sensations. They are seen. Not this, not this.",
                "Notice whatever emotion is here. It comes and goes. Not this.",
                "Notice the thoughts, even the ones about code and work. Not this.",
                "Your job, your skills, your role. All describable. Not this.",
                "Notice the sense of being someone doing this practice. That's seen too.",
                "What remains, noticing all of it?",
                "Rest as that awareness: no shape, no label. Nothing to do.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe, Tag::EyesOpen])
        .when("A bad day feels like you.")
        .walkthrough(&[
            "Notice your body, feelings and thoughts.",
            "Anything you can notice: not this.",
            "Rest as whatever is noticing.",
        ]),
        // ----- Rest your senses -----
        preset(
            Category::SensoryRest,
            "Rotating Sound",
            5,
            "For minds that hate sitting still. Instead of calming the mind \
             down, wear it out: jump between as many sounds as you can, as fast \
             as you can, without listening to any of them. A few minutes of that \
             and it's ready to rest. Works best somewhere noisy, like a café or \
             a busy office.",
            Guide::Prompts(vec![
                "Close your eyes. Catch a sound. Now the next. And the next.",
                "As many sounds as you can, as fast as you can.",
                "Don't listen to any of them. Tag it and move on.",
                "Voices? Don't follow the words. Next sound.",
                "Faster. Don't let the mind sit down.",
                "Keep going. Fans, footsteps, keys, your own breath.",
                "Now stop chasing. Let the sounds come to you.",
                "Rest. Notice how quiet the mind has become.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("Mind won't sit still.")
        .walkthrough(&[
            "Close your eyes.",
            "Jump from sound to sound, fast.",
            "Don't listen to any of them.",
        ])
        .eyes_closed(),
        preset(
            Category::SensoryRest,
            "Inner Sound",
            6,
            "A listening inquiry. For every sound you notice, ask: is this made \
             by one thing striking or rubbing another? If so, set it aside and \
             keep listening. Check the subtle ones too: the wind, your breath, \
             your heartbeat.",
            Guide::Prompts(vec![
                "Close your eyes. Notice the sounds around you.",
                "Take one. Is it made by something hitting or rubbing something else?",
                "If so, set it aside. Listen for the next one.",
                "Keys, fans, footsteps. Struck sounds. Set them aside.",
                "Now quieter ones: the air moving, your own breath.",
                "Listen inside: your pulse, your heartbeat. What makes these?",
                "Keep setting sounds aside. What's left?",
                "Rest in whatever remains, sound or silence.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("Too much noise today.")
        .walkthrough(&[
            "Close your eyes and pick out a sound.",
            "Made by one thing hitting another? Set it aside.",
            "Keep going, down to your own heartbeat.",
        ])
        .eyes_closed(),
        preset(
            Category::SensoryRest,
            "Shanmukhi Mudra",
            4,
            "Bee breath with the senses closed. Breathe in deeply, then hum on \
             the out-breath like a bee. The hands seal the face: thumbs close the \
             ears, index fingers at the outer corners of the closed eyes (or \
             above the eyebrows), middle fingers at the inner corners, ring \
             fingers beside the nostrils, little fingers at the corners of the \
             mouth. Touch lightly, and never press on the eyes.",
            Guide::Prompts(vec![
                "Sit tall. Close your ears with your thumbs. Close your eyes.",
                "Breathe in deeply. Hum on the way out, like a bee. Three times.",
                "Now the rest of the hands: eyes, nose and mouth, lightly.",
                "Five deep breaths, each one hummed all the way out.",
                "Lower your hands. Sit in the silence the humming left behind.",
                "Who are you here, before the labels come back?",
                "Notice the world returning: the body, sounds, a thought, a to-do.",
                "Take a deep breath, and open your eyes as you breathe out.",
            ]),
        )
        .tagged(&[Tag::HomeOnly])
        .when("Everything feels too loud.")
        .walkthrough(&[
            "Close your ears with your thumbs.",
            "Rest your fingers lightly on your face.",
            "Never press on your eyes.",
            "Breathe in, and hum on the way out.",
        ])
        .eyes_closed(),
        // ----- Wind down -----
        preset(
            Category::DeepUnwind,
            "Day Rewind",
            10,
            "Replay your day backwards, from now to waking up. At each moment, \
             notice what you felt (annoyed, pleased, nervous), acknowledge it, \
             and let it go. Small leftover feelings are what keep a mind busy at \
             night. Best in bed before sleep; take longer if you like. To drift \
             off afterwards, count down slowly from 100.",
            Guide::Prompts(vec![
                "Start with now: lying here, or sitting here.",
                "Just before this. What happened? What did you feel? Let it go.",
                "Keep stepping back. The evening. Small moments count too.",
                "The afternoon: messages, meetings, code. A feeling for each. Let it go.",
                "Lunch. Where were you? How did it feel?",
                "The morning's work. Acknowledge each feeling, then step back.",
                "Getting ready. Breakfast.",
                "Waking up. The first moment of the day. Let it go.",
                "Now count down from 100, a little slower than feels natural.",
                "Don't time the gaps. If you reach 1, you went too fast.",
            ]),
        )
        .tagged(&[Tag::OfficeSafe])
        .when("Replaying the day in bed.")
        .walkthrough(&[
            "Replay your day backwards.",
            "Notice each feeling, then let it go.",
        ])
        .eyes_closed(),
        preset(
            Category::DeepUnwind,
            "Yoga Nidra",
            20,
            "Yogic sleep. Lie down and work up the body with tension and \
             release, adding one muscle group per breath until the whole body is \
             tense, then let everything go. In the deep, drowsy calm that \
             follows, repeat a short resolve about who you want to be. A mat or \
             bed and a blanket help. It's fine if you fall asleep.",
            Guide::Prompts(vec![
                "Lie on your back. Arms by your sides, palms up, feet falling open.",
                "With each out-breath, sink a little further into the floor.",
                "Find what's still holding itself up: neck, lower back, shoulders. Let it drop.",
                "Breathe in and curl your toes. Breathe out and let go. Then add the ankles.",
                "One more group each breath: calves, knees, thighs. Release as you breathe out.",
                "An easy breath. Then add the hips, make fists, and tense the forearms.",
                "An easy breath. Then add the upper arms, the belly, the chest.",
                "Add the shoulder blades, squeezed back. Then the jaw and neck. Then the face.",
                "Now the whole body at once. Tense everything, hold it... and let it all go.",
                "Lie still. Notice the warmth and the heaviness. The floor does the work.",
                "Choose a short resolve about yourself: \"I am...\" or \"I deserve...\".",
                "Repeat it silently and slowly. Let it sink in.",
                "Rest here, aware, at the edge of sleep.",
                "Sit up slowly, eyes closed. Rub your palms warm and cup them over your eyes.",
            ]),
        )
        .tagged(&[Tag::HomeOnly])
        .when("Completely wiped out.")
        .walkthrough(&[
            "Lie on your back, palms up.",
            "Tense your body bit by bit, then let it all go.",
            "Falling asleep is fine.",
        ])
        .eyes_closed(),
        preset(
            Category::FocusReset,
            "Silent Sit",
            15,
            "No guidance, just the breathing circle and a timer.",
            Guide::Silent,
        )
        .when("Just a timer.")
        .walkthrough(&["No cues. A bell rings at the end."]),
    ];
    list.sort_by_key(|m| m.category);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_pattern() -> Vec<Phase> {
        vec![
            phase("in", 4.0, Motion::Grow),
            phase("hold", 4.0, Motion::Full),
            phase("out", 4.0, Motion::Shrink),
            phase("rest", 4.0, Motion::Empty),
        ]
    }

    #[test]
    fn breath_follows_the_pattern_and_loops() {
        let phases = box_pattern();
        assert_eq!(breath_at(&phases, Duration::from_secs(1)).label, "in");
        assert_eq!(breath_at(&phases, Duration::from_secs(5)).label, "hold");
        assert_eq!(breath_at(&phases, Duration::from_secs(9)).label, "out");
        assert_eq!(breath_at(&phases, Duration::from_secs(13)).label, "rest");
        assert_eq!(breath_at(&phases, Duration::from_secs(13)).index, 3);
        // 17s is 1s into the second cycle.
        assert_eq!(breath_at(&phases, Duration::from_secs(17)).label, "in");
    }

    #[test]
    fn circle_grows_then_shrinks() {
        let phases = box_pattern();
        assert!(breath_at(&phases, Duration::ZERO).size < 0.01);
        assert!((breath_at(&phases, Duration::from_secs(2)).size - 0.5).abs() < 0.01);
        assert_eq!(breath_at(&phases, Duration::from_secs(6)).size, 1.0);
        assert_eq!(breath_at(&phases, Duration::from_secs(14)).size, 0.0);
    }

    #[test]
    fn empty_pattern_does_not_panic() {
        assert_eq!(breath_at(&[], Duration::from_secs(3)).size, 0.0);
    }

    #[test]
    fn focus_practices_are_tagged_for_the_office() {
        let presets = presets();
        for name in ["Trataka", "Thought Control", "Urge Surfing"] {
            let m = presets.iter().find(|m| m.name == name).unwrap();
            assert_eq!(m.tags, &[Tag::OfficeSafe, Tag::EyesOpen], "{name}");
            assert!(m.guide.cues().is_some_and(|c| c.len() >= 6), "{name}");
        }
        // Sitting still is done with the eyes closed.
        let stillness = presets.iter().find(|m| m.name == "Kaya Sthairyam").unwrap();
        assert_eq!(stillness.tags, &[Tag::OfficeSafe]);
    }

    #[test]
    fn practices_have_their_tags() {
        use Tag::*;
        let expected: [(&str, &[Tag]); 14] = [
            ("Nadi Shuddhi", &[OfficeSafe]),
            ("Anuloma Viloma", &[OfficeSafe]),
            ("Three-Part Breath", &[OfficeSafe, EyesOpen]),
            ("Ujjayi", &[HomeOnly]),
            ("Bhastrika", &[HomeOnly]),
            ("Kapalbhati", &[HomeOnly]),
            ("Defusing \"I Am\"", &[OfficeSafe, EyesOpen]),
            ("Let It Be", &[OfficeSafe, EyesOpen]),
            ("Mind and Matter", &[OfficeSafe, EyesOpen]),
            ("Neti Neti", &[OfficeSafe, EyesOpen]),
            ("Rotating Sound", &[OfficeSafe]),
            ("Inner Sound", &[OfficeSafe]),
            ("Shanmukhi Mudra", &[HomeOnly]),
            ("Day Rewind", &[OfficeSafe]),
        ];
        let presets = presets();
        for (name, tags) in expected {
            let m = presets
                .iter()
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("{name}"));
            assert_eq!(m.tags, tags, "{name}");
        }
        let nidra = presets.iter().find(|m| m.name == "Yoga Nidra").unwrap();
        assert_eq!(nidra.tags, &[HomeOnly]);
        assert_eq!(nidra.minutes, 20);
    }

    #[test]
    fn fast_breathing_summaries_are_squeezed() {
        let find = |name: &str| presets().into_iter().find(|m| m.name == name).unwrap();
        assert_eq!(
            find("Bhastrika").guide.pattern_summary().as_deref(),
            Some("(In 1 · Out 1) ×9 · Rest 20")
        );
        assert_eq!(
            find("Kapalbhati").guide.pattern_summary().as_deref(),
            Some("(Out 0.4 · In 0.6) ×30 · In 5 · Out 6 · Rest 20")
        );
        assert_eq!(
            find("Three-Part Breath").guide.pattern_summary().as_deref(),
            Some("In 2 · In 2 · In 2 · Out 6")
        );
    }

    #[test]
    fn three_part_breath_fills_in_thirds() {
        let breath = presets()
            .into_iter()
            .find(|m| m.name == "Three-Part Breath")
            .unwrap();
        let Guide::Breath(phases) = breath.guide else {
            panic!("not a breathing pattern");
        };
        let size_at = |secs: f64| breath_at(&phases, Duration::from_secs_f64(secs)).size;
        assert!((size_at(2.0) - 0.33).abs() < 0.01);
        assert!((size_at(4.0) - 0.66).abs() < 0.01);
        assert!(size_at(5.99) > 0.99);
        assert!(size_at(11.99) < 0.01);
    }

    #[test]
    fn motion_direction() {
        assert_eq!(Motion::Grow.rising(), Some(true));
        assert_eq!(Motion::Shrink.rising(), Some(false));
        assert_eq!(Motion::Full.rising(), None);
        assert_eq!(Motion::Partial { from: 0.2, to: 0.8 }.rising(), Some(true));
        assert_eq!(Motion::Partial { from: 0.8, to: 0.2 }.rising(), Some(false));
        assert_eq!(Motion::Partial { from: 0.5, to: 0.5 }.rising(), None);
        assert_eq!(Motion::Rest.rising(), None);
    }

    #[test]
    fn trataka_is_a_gazing_practice() {
        let trataka = presets().into_iter().find(|m| m.name == "Trataka").unwrap();
        assert!(matches!(trataka.guide, Guide::Gaze(_)));
        assert_eq!(trataka.guide.kind(), "Gazing practice");
    }

    #[test]
    fn every_preset_has_a_category_and_the_list_is_grouped() {
        let presets = presets();
        assert!(presets.iter().all(|m| m.category.is_some()));
        assert!(presets.windows(2).all(|w| w[0].category <= w[1].category));
        for category in Category::ALL {
            assert!(
                presets.iter().any(|m| m.category == Some(category)),
                "{category:?}"
            );
        }
    }

    #[test]
    fn every_preset_has_a_short_when_and_walkthrough() {
        for m in presets() {
            let when = m.when.unwrap_or_else(|| panic!("{}", m.name));
            assert!(when.len() <= 40, "{}: {when}", m.name);
            assert!((1..=4).contains(&m.walkthrough.len()), "{}", m.name);
            // Two lines of the 64-column walkthrough box.
            assert!(m.walkthrough.iter().all(|s| s.len() <= 128), "{}", m.name);
        }
    }

    #[test]
    fn eyes_closed_practices_are_not_eyes_open() {
        for m in presets() {
            assert!(
                !(m.eyes_closed && m.tags.contains(&Tag::EyesOpen)),
                "{}",
                m.name
            );
        }
    }

    #[test]
    fn start_here_begins_with_nadi_shuddhi() {
        let names: Vec<String> = presets()
            .into_iter()
            .filter(|m| m.category == Some(Category::StartHere))
            .map(|m| m.name)
            .collect();
        assert_eq!(
            names,
            [
                "Nadi Shuddhi",
                TUTORIAL_PRESET,
                "Box Breathing",
                "Following the Breath"
            ]
        );
    }

    fn warm_up_of(name: &str) -> Vec<Phase> {
        let meditation = presets().into_iter().find(|m| m.name == name).unwrap();
        let Guide::Breath(phases) = meditation.guide else {
            panic!("{name} isn't a breathing pattern");
        };
        warm_up(&phases)
    }

    #[test]
    fn a_warm_up_starts_slow_and_ends_at_the_real_pace() {
        let seconds = |phases: &[Phase]| -> Vec<f64> { phases.iter().map(|p| p.seconds).collect() };
        let nadi = warm_up_of("Nadi Shuddhi");
        // Doubled at first (but never more than 4 seconds longer), easing
        // down, then once through at the real 4 / 6 / 4 / 6.
        assert_eq!(seconds(&nadi), [8.0, 10.0, 6.0, 7.5, 4.0, 6.0, 4.0, 6.0]);

        // Holds stay as they are.
        let box_breathing = warm_up_of("Box Breathing");
        assert!(
            box_breathing
                .iter()
                .filter(|p| p.motion.rising().is_none())
                .all(|p| p.seconds == 4.0)
        );
        assert_eq!(seconds(&box_breathing[box_breathing.len() - 4..]), [4.0; 4]);
    }

    #[test]
    fn a_warm_up_of_fast_breathing_stays_short() {
        let kapalbhati = warm_up_of("Kapalbhati");
        // 30 pumps cut to 10 breaths, and the 20 second rest to 8.
        assert_eq!(kapalbhati.len(), 20 + 3);
        assert_eq!(kapalbhati.last().unwrap().seconds, 8.0);
        assert!(kapalbhati[0].seconds > kapalbhati[18].seconds);
    }

    #[test]
    fn sides_come_from_capital_left_and_right() {
        assert_eq!(side("Breathe in LEFT"), Some(Side::Left));
        assert_eq!(side("Switch · breathe out RIGHT"), Some(Side::Right));
        assert_eq!(side("Close your right nostril"), None);
    }

    #[test]
    fn prep_steps_fit_the_practice() {
        let find = |name: &str| presets().into_iter().find(|m| m.name == name).unwrap();
        let titles = |name: &str| -> Vec<&str> {
            prep_steps(&find(name))
                .iter()
                .map(|(title, _)| *title)
                .collect()
        };
        assert_eq!(
            titles("Nadi Shuddhi"),
            ["Chair & posture", "Breath", "Quiet"]
        );
        assert_eq!(titles("Trataka"), ["Chair & posture", "Eyes", "Quiet"]);
        assert_eq!(titles("Rotating Sound"), ["Chair & posture", "Sound"]);
        assert_eq!(
            titles("Three-Part Breath"),
            ["Chair & posture", "Breath", "Eyes"]
        );
        for m in presets() {
            let n = prep_steps(&m).len();
            assert!((2..=3).contains(&n), "{}: {n}", m.name);
        }
    }

    #[test]
    fn unlocks_point_at_real_presets() {
        let presets = presets();
        let rule = |name: &str| presets.iter().find(|m| m.name == name).unwrap().unlock;
        assert_eq!(
            rule("Anuloma Viloma"),
            Some(Unlock {
                after: "Nadi Shuddhi",
                days: 14
            })
        );
        assert_eq!(rule("Prana Shuddhi").unwrap().after, "Anuloma Viloma");
        assert_eq!(rule("Bhastrika").unwrap().after, "Kapalbhati");
        assert_eq!(rule("Nadi Shuddhi"), None);
        for m in &presets {
            if let Some(unlock) = m.unlock {
                let first = presets.iter().find(|p| p.name == unlock.after);
                assert!(first.is_some_and(|p| p.unlock.is_none() || p.name != m.name));
                assert!(unlock.days > 0, "{}", m.name);
            }
        }
    }

    #[test]
    fn preset_names_are_unique() {
        let presets = presets();
        for (i, a) in presets.iter().enumerate() {
            assert!(
                presets[i + 1..].iter().all(|b| b.name != a.name),
                "{}",
                a.name
            );
        }
    }

    #[test]
    fn prompts_are_spread_evenly() {
        let total = Duration::from_secs(100);
        assert_eq!(prompt_index(4, Duration::ZERO, total), 0);
        assert_eq!(prompt_index(4, Duration::from_secs(26), total), 1);
        assert_eq!(prompt_index(4, Duration::from_secs(99), total), 3);
        assert_eq!(prompt_index(4, Duration::from_secs(500), total), 3);
        assert_eq!(prompt_index(0, Duration::from_secs(5), total), 0);
    }

    #[test]
    fn pattern_summary_reads_nicely() {
        let coherent = presets()
            .into_iter()
            .find(|m| m.name == TUTORIAL_PRESET)
            .unwrap();
        assert_eq!(
            coherent.guide.pattern_summary().as_deref(),
            Some("In 5.5 · Out 5.5")
        );
        assert_eq!(Guide::Silent.pattern_summary(), None);
    }
}
