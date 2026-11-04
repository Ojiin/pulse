// What Pulse keeps of its own: what the person marked as seen or done, which
// notifications went out for which item, and the day of the last morning
// summary. One JSON file in the app's support folder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::checks::Flag;
use crate::util::support_dir;

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
    // where the person moved the panel (its top left, in pixels of the screen);
    // none: below the menu bar item
    pub panel_at: Option<[i32; 2]>,
}

pub static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(load().unwrap_or_default()));

fn path() -> Result<PathBuf, String> {
    Ok(support_dir()?.join("state.json"))
}

fn load() -> Option<Store> {
    load_from(&path().ok()?)
}

// no file: none; a file that does not read as a store (torn by two Pulses
// saving at once, or written by a later build): none too, and the file is
// kept as state.json.bad, so that the first save does not wipe it
fn load_from(path: &Path) -> Option<Store> {
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(store) => Some(store),
        Err(error) => {
            let bad = path.with_extension("json.bad");
            match std::fs::rename(path, &bad) {
                Ok(()) => eprintln!("store: {error}; kept as {}", bad.display()),
                Err(not_kept) => eprintln!("store: {error}; {not_kept}"),
            }
            None
        }
    }
}

pub fn save(store: &Store) -> Result<(), String> {
    save_to(&path()?, store)
}

// through a tmp file of this process's own: two Pulses saving at once (a dev
// build beside the installed one) each move a whole file into place. A tmp
// file a save could not move is removed
fn save_to(path: &Path, store: &Store) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let text = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_leaves_another_pulses_save_alone_and_no_tmp_behind() {
        let dir = std::env::temp_dir().join(format!("pulse-store-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("state.json");
        // another Pulse between its write and its rename: a build from before
        // the tmp files of their own, and one after
        let theirs = [dir.join("state.json.tmp"), dir.join(format!("state.json.{}.tmp", std::process::id() + 1))];
        for tmp in &theirs {
            std::fs::write(tmp, "{\n  \"marks\": {").expect("theirs");
        }
        let store = Store { is_started: true, panel_at: Some([100, 200]), ..Store::default() };
        save_to(&path, &store).expect("save");
        let read = load_from(&path).expect("whole");
        assert_eq!((read.is_started, read.panel_at), (true, Some([100, 200])));
        for tmp in &theirs {
            assert_eq!(std::fs::read_to_string(tmp).expect("theirs"), "{\n  \"marks\": {");
        }
        assert_eq!(std::fs::read_dir(&dir).expect("dir").count(), 3);

        // a save that cannot be moved into place takes its tmp file with it
        std::fs::remove_file(&path).expect("saved");
        std::fs::create_dir(&path).expect("in the way");
        assert!(save_to(&path, &store).is_err());
        assert_eq!(std::fs::read_dir(&dir).expect("dir").count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_does_not_read_is_kept_and_not_wiped_by_the_first_save() {
        let dir = std::env::temp_dir().join(format!("pulse-store-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let (path, bad) = (dir.join("state.json"), dir.join("state.json.bad"));
        let whole = serde_json::to_vec_pretty(&Store { is_started: true, ..Store::default() }).expect("json");
        // one save's whole file and the tail of a longer one, the tail starting
        // in plain text or inside a character; and a mark of a kind a later
        // build has
        let unread = [
            [&whole[..], b"0000000020\": { \"kind\": \"done\", \"at\": 1.0 }\n  }\n}"].concat(),
            [&whole[..], b"\xa6 out\"\n  }\n}"].concat(),
            br#"{"marks": {"local_a": {"kind": "snoozed", "at": 1.0}}}"#.to_vec(),
        ];
        for bytes in &unread {
            std::fs::write(&path, bytes).expect("state");
            assert!(load_from(&path).is_none());
            assert!(!path.exists());
            // the start's first save
            save_to(&path, &Store::default()).expect("save");
            assert_eq!(&std::fs::read(&bad).expect("kept"), bytes);
        }

        // a whole file is read and no file is none; neither is set aside
        std::fs::remove_file(&bad).expect("kept");
        std::fs::write(&path, &whole).expect("state");
        assert!(load_from(&path).is_some_and(|store| store.is_started));
        std::fs::remove_file(&path).expect("state");
        assert!(load_from(&path).is_none());
        assert!(!bad.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
