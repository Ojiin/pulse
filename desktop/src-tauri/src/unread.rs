// The sidebar's unread dots, as the Claude app keeps them: a localStorage
// entry ("epitaxy-unread-v1") in Chromium's LevelDB under the app's support
// folder. The app holds that database open, so this reads a copy and never
// touches the original. The format is the app's own and may change; a
// failed read answers None and the caller falls back to a guess.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::util::mtime_ms;

// Chromium's localStorage key: "_" + origin + "\0\1" + name
const KEY: &[u8] = b"_https://claude.ai\x00\x01epitaxy-unread-v1";

// the stamp of the files read, when, and the ids
static CACHE: LazyLock<Mutex<Option<(f64, Instant, BTreeSet<String>)>>> = LazyLock::new(Default::default);
// the app writes its storage every few seconds while a chat runs: copying
// it that often costs more than a dot that shows up a little later
const AT_MOST: Duration = Duration::from_secs(20);
static COPIES: AtomicU64 = AtomicU64::new(0);

// a value is led by \0 (UTF-16LE text) or \1 (Latin-1 text)
fn decode(raw: &[u8]) -> String {
    match raw.split_first() {
        Some((0, rest)) => {
            let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        Some((_, rest)) => String::from_utf8_lossy(rest).into_owned(),
        None => String::new(),
    }
}

fn read_copy(source: &Path, files: &[String]) -> Result<BTreeSet<String>, String> {
    // its own folder per read: two reads may overlap
    let n = COPIES.fetch_add(1, Ordering::Relaxed);
    let copy = std::env::temp_dir().join(format!("pulse-ldb-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&copy).map_err(|e| e.to_string())?;
    let result = (|| {
        for f in files {
            std::fs::copy(source.join(f), copy.join(f)).map_err(|e| e.to_string())?;
        }
        let options = rusty_leveldb::Options { create_if_missing: false, ..Default::default() };
        let mut db = rusty_leveldb::DB::open(&copy, options).map_err(|e| e.to_string())?;
        let raw = db.get(KEY);
        drop(db);
        let mut ids = BTreeSet::new();
        if let Some(raw) = raw {
            let parsed: Value = serde_json::from_str(&decode(&raw)).map_err(|e| e.to_string())?;
            for key in ["unreadIds", "explicitUnreadIds"] {
                if let Some(list) = parsed.get("state").and_then(|s| s.get(key)).and_then(Value::as_array) {
                    ids.extend(list.iter().filter_map(Value::as_str).map(str::to_string));
                }
            }
        }
        Ok(ids)
    })();
    let _ = std::fs::remove_dir_all(&copy);
    result
}

pub fn read_unread(source: &Path) -> Option<BTreeSet<String>> {
    let files: Vec<String> = std::fs::read_dir(source)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|f| f != "LOCK")
        .collect();
    let stamp = files
        .iter()
        .filter_map(|f| std::fs::metadata(source.join(f)).ok())
        .map(|m| mtime_ms(&m))
        .fold(0.0, f64::max);
    if let Some((cached, at, ids)) = CACHE.lock().expect("unread cache").as_ref() {
        if *cached == stamp || at.elapsed() < AT_MOST {
            return Some(ids.clone());
        }
    }
    match read_copy(source, &files) {
        Ok(ids) => {
            *CACHE.lock().expect("unread cache") = Some((stamp, Instant::now(), ids.clone()));
            Some(ids)
        }
        Err(error) => {
            eprintln!("unread: {error}");
            None
        }
    }
}
