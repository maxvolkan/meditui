//! Recipes: meditations written as TOML files in
//! ~/.config/meditui/recipes/*.toml. Run `medi recipe` to print an example.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::app::MAX_MINUTES;
use crate::library::{Category, Guide, Meditation, Motion, Phase, PrepStep, Tag};

/// Printed by `medi recipe`: a working recipe with every option explained.
pub const EXAMPLE: &str = r#"# A meditui recipe. Save it as ~/.config/meditui/recipes/<anything>.toml
# and it shows up in your library the next time you start medi.

name = "Slow Box"
minutes = 6
description = "Box breathing at a slower pace, for winding down."

# Optional. One of: start_here, focus_reset, down_regulation,
# cognitive_defusion, sensory_rest, deep_unwind
category = "down_regulation"

# Optional. Any of: office_safe, eyes_open, home_only
tags = ["office_safe", "eyes_open"]

# Optional: when it helps, in a few words. Shown when its folder is selected.
when = "Winding down after work."

# Optional: short steps shown one at a time the first time (and with i). A
# breathing pattern then gets a slow warm-up with the circle.
walkthrough = ["Breathe in, hold, breathe out, hold.", "Five seconds each."]

# Optional: done with the eyes closed, so the session starts dim.
eyes_closed = false

# Optional: your own checklist before it starts (replaces the usual one).
prep = ["Feet flat on the floor", "Phone face down"]

# Optional: a soft chime every this many minutes.
interval_bell = 2

# Guidance. Pick ONE:
#
# - Text cues, spread evenly over the session:
#     cues = ["Settle in.", "Notice the breath.", "Rest here."]
#   Add `gaze = true` to show a still point instead of the breathing circle.
#   (Put these lines up here, above any [[breath]] blocks.)
#
# - A breathing pattern that repeats, as [[breath]] blocks like the ones
#   below. motion is one of: in, out, hold, hold_empty, rest
#
# - Neither, for a silent sit.

[[breath]]
label = "Breathe in"
seconds = 5
motion = "in"

[[breath]]
label = "Hold"
seconds = 5
motion = "hold"

[[breath]]
label = "Breathe out"
seconds = 5
motion = "out"

[[breath]]
label = "Hold"
seconds = 5
motion = "hold_empty"
"#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    name: String,
    minutes: u64,
    #[serde(default)]
    description: String,
    category: Option<Category>,
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    prep: Vec<String>,
    when: Option<String>,
    #[serde(default)]
    walkthrough: Vec<String>,
    #[serde(default)]
    eyes_closed: bool,
    interval_bell: Option<u64>,
    #[serde(default)]
    breath: Vec<Step>,
    #[serde(default)]
    cues: Vec<String>,
    #[serde(default)]
    gaze: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    label: String,
    seconds: f64,
    motion: StepMotion,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum StepMotion {
    In,
    Out,
    Hold,
    HoldEmpty,
    Rest,
}

impl From<StepMotion> for Motion {
    fn from(motion: StepMotion) -> Self {
        match motion {
            StepMotion::In => Motion::Grow,
            StepMotion::Out => Motion::Shrink,
            StepMotion::Hold => Motion::Full,
            StepMotion::HoldEmpty => Motion::Empty,
            StepMotion::Rest => Motion::Rest,
        }
    }
}

pub fn recipes_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("meditui").join("recipes"))
}

/// Loads every `.toml` file in `dir`, in file-name order. Returns the
/// meditations, plus one message per file that couldn't be used. A missing
/// folder just means no recipes.
pub fn load_dir(dir: &Path) -> (Vec<Meditation>, Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();

    let mut found = Vec::new();
    let mut problems = Vec::new();
    for path in paths {
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        match fs::read_to_string(&path)
            .map_err(|err| err.to_string())
            .and_then(|text| parse(&text))
        {
            Ok(meditation) => found.push(meditation),
            Err(err) => problems.push(format!("{file}: {err}")),
        }
    }
    (found, problems)
}

/// Turns one recipe file's text into a meditation, or explains what's wrong.
pub fn parse(text: &str) -> Result<Meditation, String> {
    let recipe: Recipe = toml::from_str(text).map_err(|err| err.message().to_string())?;

    let name = recipe.name.trim();
    if name.is_empty() {
        return Err("name is empty".to_string());
    }
    if !(1..=MAX_MINUTES).contains(&recipe.minutes) {
        return Err(format!("minutes must be between 1 and {MAX_MINUTES}"));
    }
    if recipe.interval_bell == Some(0) {
        return Err("interval_bell must be at least 1 minute".to_string());
    }
    if !recipe.breath.is_empty() && !recipe.cues.is_empty() {
        return Err("use either [[breath]] or cues, not both".to_string());
    }
    if recipe.gaze && !recipe.breath.is_empty() {
        return Err("gaze goes with cues, not with [[breath]]".to_string());
    }
    for (i, step) in recipe.breath.iter().enumerate() {
        if !(step.seconds > 0.0 && step.seconds <= 600.0) {
            return Err(format!(
                "breath step {}: seconds must be between 0 and 600",
                i + 1
            ));
        }
    }

    // Labels, cues and tags are `&'static` in the app, because the built-in
    // meditations are written into the code. Recipes are read once at
    // startup, so leaking their few strings to match is fine.
    let cues: Vec<&'static str> = recipe.cues.into_iter().map(leak).collect();
    let guide = if !recipe.breath.is_empty() {
        Guide::Breath(
            recipe
                .breath
                .into_iter()
                .map(|step| Phase {
                    label: leak(step.label),
                    seconds: step.seconds,
                    motion: step.motion.into(),
                })
                .collect(),
        )
    } else if recipe.gaze {
        Guide::Gaze(cues)
    } else if !cues.is_empty() {
        Guide::Prompts(cues)
    } else {
        Guide::Silent
    };

    let description = if recipe.description.trim().is_empty() {
        "From your recipes folder.".to_string()
    } else {
        recipe.description.trim().to_string()
    };
    let prep = (!recipe.prep.is_empty()).then(|| {
        recipe
            .prep
            .into_iter()
            .map(|step| -> PrepStep { (leak(step), "") })
            .collect()
    });

    Ok(Meditation {
        name: name.to_string(),
        description,
        minutes: recipe.minutes,
        guide,
        custom: false,
        recipe: true,
        tags: Box::leak(recipe.tags.into_boxed_slice()),
        category: recipe.category,
        prep,
        interval_bell: recipe.interval_bell,
        unlock: None,
        when: recipe
            .when
            .map(|when| when.trim().to_string())
            .filter(|when| !when.is_empty())
            .map(leak),
        walkthrough: recipe.walkthrough.into_iter().map(leak).collect(),
        eyes_closed: recipe.eyes_closed,
    })
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_recipe_works() {
        let m = parse(EXAMPLE).unwrap();
        assert_eq!(m.name, "Slow Box");
        assert_eq!(m.minutes, 6);
        assert!(m.recipe && !m.custom);
        assert_eq!(m.category, Some(Category::DownRegulation));
        assert_eq!(m.tags, &[Tag::OfficeSafe, Tag::EyesOpen]);
        assert_eq!(m.interval_bell, Some(2));
        assert!(m.when.is_some());
        assert_eq!(m.walkthrough.len(), 2);
        assert!(!m.eyes_closed);
        assert_eq!(
            m.guide.pattern_summary().as_deref(),
            Some("In 5 · Hold 5 · Out 5 · Hold 5")
        );
        let prep: Vec<&str> = m.prep.unwrap().iter().map(|(title, _)| *title).collect();
        assert_eq!(prep, ["Feet flat on the floor", "Phone face down"]);
    }

    #[test]
    fn cues_gaze_and_silent_recipes() {
        let cues = parse("name = \"C\"\nminutes = 3\ncues = [\"a\", \"b\"]").unwrap();
        assert!(matches!(cues.guide, Guide::Prompts(ref c) if c == &["a", "b"]));

        let gaze = parse("name = \"G\"\nminutes = 3\ncues = [\"look\"]\ngaze = true").unwrap();
        assert!(matches!(gaze.guide, Guide::Gaze(_)));

        let silent = parse("name = \"S\"\nminutes = 20").unwrap();
        assert!(matches!(silent.guide, Guide::Silent));
        assert_eq!(silent.description, "From your recipes folder.");
        assert_eq!(silent.category, None);
    }

    #[test]
    fn mistakes_are_explained() {
        let error = |text: &str| parse(text).unwrap_err();
        assert!(error("minutes = 3").contains("name"));
        assert!(error("name = \"x\"\nminutes = 0").contains("between 1"));
        assert!(error("name = \" \"\nminutes = 3").contains("empty"));
        assert!(error("name = \"x\"\nminutes = 3\ninterval_bell = 0").contains("interval_bell"));
        assert!(error("name = \"x\"\nminutes = 3\ncolour = \"red\"").contains("colour"));
        assert!(error("name = \"x\"\nminutes = 3\ntags = [\"loud\"]").contains("loud"));
        let both = "name = \"x\"\nminutes = 3\ncues = [\"a\"]\n\
                    [[breath]]\nlabel = \"in\"\nseconds = 4\nmotion = \"in\"";
        assert!(error(both).contains("not both"));
        let bad_step = "name = \"x\"\nminutes = 3\n\
                        [[breath]]\nlabel = \"in\"\nseconds = 0\nmotion = \"in\"";
        assert!(error(bad_step).contains("step 1"));
    }

    #[test]
    fn a_folder_of_recipes_loads_the_good_and_reports_the_bad() {
        let dir = std::env::temp_dir().join(format!("meditui-recipes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("b_good.toml"), "name = \"Second\"\nminutes = 4").unwrap();
        fs::write(dir.join("a_good.toml"), "name = \"First\"\nminutes = 2").unwrap();
        fs::write(dir.join("broken.toml"), "name = ").unwrap();
        fs::write(dir.join("notes.txt"), "not a recipe").unwrap();

        let (found, problems) = load_dir(&dir);
        fs::remove_dir_all(&dir).unwrap();

        let names: Vec<&str> = found.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["First", "Second"]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("broken.toml: "), "{problems:?}");
    }

    #[test]
    fn the_example_recipes_in_the_repo_all_load() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("recipes");
        let (found, problems) = load_dir(&dir);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(found.len(), 4);
    }

    #[test]
    fn a_missing_folder_means_no_recipes() {
        let (found, problems) = load_dir(Path::new("/definitely/not/a/real/folder"));
        assert!(found.is_empty() && problems.is_empty());
    }
}
