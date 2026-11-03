// What the person chose for the notifications on the Mac: per open state
// whether it is told at once and whether it is reminded of while it lies,
// after how many hours, and the morning summary; and how the panel looks.
// One JSON file beside the state; without one, Pulse tells as it always did.

use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::items::State;
use crate::util::support_dir;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub at_once: bool,
    pub remind: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub blocked: Rule,
    // a question, a decision or points still open
    pub waiting: Rule,
    pub review: Rule,
    // a command that runs long, was forgotten, hangs or spins
    #[serde(default = "commands_rule")]
    pub commands: Rule,
    #[serde(default)]
    pub limits: Limits,
    // after how many hours an open item is reminded of, ascending
    pub remind_hours: Vec<u32>,
    // and then every 24 hours after the last of them
    pub remind_daily: bool,
    pub summary: bool,
    // minutes after midnight, local time
    pub summary_at: u32,
    // on Mondays the summary names the parked topics
    #[serde(default = "yes")]
    pub parked_weekly: bool,
    #[serde(default)]
    pub appearance: Appearance,
}

// how the panel looks: as the system does, or always light or dark
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

fn yes() -> bool {
    true
}

// in minutes: when a command is worth a look (src/checks.rs)
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    // in the background, its session quiet for
    pub forgotten_min: u32,
    // in the background, running for
    pub long_min: u32,
    // in the foreground, the session waiting for it for
    pub stuck_min: u32,
    // most of a core without a break for
    pub hot_min: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { forgotten_min: 60, long_min: 120, stuck_min: 15, hot_min: 10 }
    }
}

fn commands_rule() -> Rule {
    Rule { at_once: true, remind: true }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            blocked: Rule { at_once: true, remind: true },
            waiting: Rule { at_once: true, remind: true },
            review: Rule { at_once: false, remind: true },
            commands: commands_rule(),
            limits: Limits::default(),
            remind_hours: vec![2, 4, 12, 24],
            remind_daily: true,
            summary: true,
            summary_at: 8 * 60,
            parked_weekly: true,
            appearance: Appearance::System,
        }
    }
}

pub const MAX_HOURS: u32 = 720;
const MAX_STEPS: usize = 10;

impl Settings {
    pub fn rule(&self, state: State) -> Option<Rule> {
        match state {
            State::Blocked => Some(self.blocked),
            State::Waiting => Some(self.waiting),
            State::Review => Some(self.review),
            State::Running | State::Quiet => None,
        }
    }

    // the hours in order, each once; what is out of range is refused
    pub fn checked(mut self) -> Result<Settings, String> {
        if self.remind_hours.iter().any(|&h| h == 0 || h > MAX_HOURS) {
            return Err(format!("Reminders: 1 to {MAX_HOURS} hours"));
        }
        self.remind_hours.sort_unstable();
        self.remind_hours.dedup();
        if self.remind_hours.len() > MAX_STEPS {
            return Err(format!("Reminders: at most {MAX_STEPS} times"));
        }
        let l = self.limits;
        if [l.forgotten_min, l.long_min, l.stuck_min, l.hot_min].iter().any(|&m| m == 0 || m > MAX_HOURS * 60) {
            return Err(format!("Commands: 1 minute to {MAX_HOURS} hours"));
        }
        if self.summary_at >= 24 * 60 {
            return Err("Morning summary: invalid time".into());
        }
        Ok(self)
    }
}

// the settings, and why the file could not be read if it could not
pub static SETTINGS: LazyLock<Mutex<(Settings, Option<String>)>> = LazyLock::new(|| Mutex::new(load()));

fn path() -> Result<PathBuf, String> {
    Ok(support_dir()?.join("settings.json"))
}

// no file: the defaults; a file that cannot be read: the defaults too, and
// the panel says so
fn load() -> (Settings, Option<String>) {
    let path = match path() {
        Ok(path) => path,
        Err(error) => return (Settings::default(), Some(error)),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(error) => return (Settings::default(), Some(error.to_string())),
    };
    match serde_json::from_str::<Settings>(&text).map_err(|e| e.to_string()).and_then(Settings::checked) {
        Ok(settings) => (settings, None),
        Err(error) => (Settings::default(), Some(error)),
    }
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let path = path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_are_put_in_order_and_out_of_range_is_refused() {
        let with = |hours: Vec<u32>| Settings { remind_hours: hours, ..Settings::default() }.checked();
        assert_eq!(with(vec![24, 2, 4, 2]).map(|s| s.remind_hours), Ok(vec![2, 4, 24]));
        assert!(with(vec![0]).is_err());
        assert!(with(vec![MAX_HOURS + 1]).is_err());
        assert!(with((1..=11).collect()).is_err());
        assert_eq!(with(vec![]).map(|s| s.remind_hours), Ok(vec![]));
        assert!(Settings { summary_at: 24 * 60, ..Settings::default() }.checked().is_err());
        assert!(Settings { limits: Limits { hot_min: 0, ..Limits::default() }, ..Settings::default() }.checked().is_err());
    }

    #[test]
    fn a_file_from_before_the_commands_reads_with_their_defaults() {
        let mut old = serde_json::to_value(Settings::default()).expect("json");
        let map = old.as_object_mut().expect("object");
        map.remove("commands");
        map.remove("limits");
        map.remove("appearance");
        let read: Settings = serde_json::from_value(old).expect("read");
        assert_eq!((read.commands, read.limits, read.appearance), (commands_rule(), Limits::default(), Appearance::System));
    }
}
