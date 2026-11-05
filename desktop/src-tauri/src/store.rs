// What Pulse keeps of its own: what the person marked as seen or done, which
// notifications went out for which item, and the day of the last morning
// summary. One JSON file in the app's support folder.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::checks::Flag;
use crate::util::home;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MarkKind {
    Seen,
    Done,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Mark {
    pub kind: MarkKind,
    pub at: f64,
    // how far the item had come when it was marked: what came after is not covered
    #[serde(default)]
    pub shown: Option<f64>,
}

// a topic set aside, with a copy of what it was, so that it outlives its
// session in the app
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Parked {
    pub at: f64,
    pub title: String,
    pub project: String,
    pub said: String,
}

// what was told about an item: the state it was in, since when, and how many
// reminders went out while it lay there
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Told {
    pub state: String,
    pub since: f64,
    pub reminders: u32,
}

// a command marked as known: for what it was worth a look then, and when;
// it holds until the next reminder step of that flag
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Known {
    pub flag: Option<Flag>,
    pub at: f64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Store {
    pub marks: HashMap<String, Mark>,
    pub told: HashMap<String, Told>,
    // the local day ("2026-10-29") of the last morning summary
    pub last_summary: Option<String>,
    // false until the first look: what lies around then is taken as told
    pub is_started: bool,
    // commands the person marked as known ("<session>/<id>"), while they run
    // (the earlier "known", a plain list kept until a restart, is dropped)
    pub known_at: HashMap<String, Known>,
    // topics set aside, by session
    pub parked: HashMap<String, Parked>,
    // taken up again in the panel while the session's last answer still says parked
    pub resumed: HashMap<String, f64>,
    // answers that came with nobody at the app, by session (items.rs)
    pub unseen: HashMap<String, f64>,
    // where the person moved the panel (its top left, in pixels of the screen);
    // none: below the menu bar item
    pub panel_at: Option<[i32; 2]>,
}

pub static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(load().unwrap_or_default()));

fn path() -> Result<PathBuf, String> {
    Ok(home()?.join("Library/Application Support/local.claude-dashboard.pulse/state.json"))
}

fn load() -> Option<Store> {
    let text = std::fs::read_to_string(path().ok()?).ok()?;
    match serde_json::from_str(&text) {
        Ok(store) => Some(store),
        Err(error) => {
            eprintln!("store: {error}");
            None
        }
    }
}

pub fn save(store: &Store) -> Result<(), String> {
    let path = path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
