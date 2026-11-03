// The subagents of a session: Claude Code keeps each in
// <transcript without .jsonl>/subagents/agent-<id>.jsonl, beside a
// .meta.json that says its type, its task and whether it ran in the
// background. Whether one still runs is read from its transcript's end.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use serde_json::Value;

use crate::running::is_running;
use crate::util::{first_chars, mtime_ms, parse_time, read_head, read_tail};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentView {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    pub started_at: f64,
    pub last_at: f64,
    pub is_running: bool,
    pub is_background: bool,
    pub depth: f64,
}

static CACHE: LazyLock<Mutex<HashMap<PathBuf, (f64, SubagentView)>>> = LazyLock::new(Default::default);

const TIMESTAMP: &str = "\"timestamp\":\"";

// the times every `"timestamp":"…"` in a text names, in order
fn timestamps(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(TIMESTAMP).filter_map(move |(i, _)| {
        let rest = &text[i + TIMESTAMP.len()..];
        rest.find('"').map(|end| &rest[..end])
    })
}

// a settings line such as "Thoroughness: medium" that leads some prompts
fn is_setting(line: &str) -> bool {
    let lower = line.to_lowercase();
    ["thoroughness", "effort", "model"].iter().any(|word| {
        lower.strip_prefix(word).is_some_and(|rest| {
            !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') && rest.trim_start().starts_with(':')
        })
    })
}

// its sentences: a split after . ! or ? and the white space that follows
fn sentences(text: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut prev = ' ';
    for c in text.chars() {
        if c.is_whitespace() && matches!(prev, '.' | '!' | '?') {
            parts.push(String::new());
        } else {
            parts.last_mut().expect("one part").push(c);
        }
        prev = c;
    }
    parts
}

// the first entry: when it started, and the task it was given (older
// subagents have no .meta.json, their prompt is the only description)
fn head(path: &Path) -> std::io::Result<(f64, String)> {
    let buffer = read_head(path, 256 * 1024)?;
    let text = String::from_utf8_lossy(&buffer);
    let first = text.split('\n').next().unwrap_or("");
    let (started_at, task) = match serde_json::from_str::<Value>(first) {
        Ok(line) => {
            let at = line.get("timestamp").and_then(Value::as_str).and_then(parse_time).unwrap_or(0.0);
            let task = match line.get("message").and_then(|m| m.get("content")) {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Array(blocks)) => blocks
                    .iter()
                    .map(|b| match b.get("text") {
                        Some(Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => String::new(),
            };
            (at, task)
        }
        Err(_) => (timestamps(&text).next().and_then(parse_time).unwrap_or(0.0), String::new()),
    };
    // its first real sentence, short: settings lines are skipped
    let body = task
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty() && !is_setting(l))
        .collect::<Vec<_>>()
        .join(" ");
    let sentence = sentences(&body).into_iter().find(|s| s.chars().count() > 12).unwrap_or(body);
    let short = if sentence.chars().count() > 110 { format!("{}…", first_chars(&sentence, 107)) } else { sentence };
    Ok((started_at, short))
}

// the last entry's time: a copied or touched file keeps it, its mtime does not
fn last_timestamp(path: &Path, size: u64) -> std::io::Result<f64> {
    let buffer = read_tail(path, size, size.min(64 * 1024))?;
    let text = String::from_utf8_lossy(&buffer);
    Ok(timestamps(&text).last().and_then(parse_time).unwrap_or(0.0))
}

fn view(path: &Path, name: &str) -> Option<(f64, SubagentView)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = mtime_ms(&meta);
    if let Some(hit) = CACHE.lock().expect("subagent cache").get(path) {
        if hit.0 == mtime {
            return Some(hit.clone());
        }
    }
    let meta_path = path.with_extension("meta.json");
    let info: Value = match std::fs::read_to_string(meta_path) {
        Ok(text) => serde_json::from_str(&text).ok()?,
        Err(_) => Value::Object(Default::default()),
    };
    let (started_at, task) = head(path).ok()?;
    let last = last_timestamp(path, meta.len()).ok()?;
    let description = info.get("description").and_then(Value::as_str).filter(|d| !d.is_empty());
    let built = SubagentView {
        id: name.trim_start_matches("agent-").trim_end_matches(".jsonl").to_string(),
        kind: info.get("agentType").and_then(Value::as_str).unwrap_or("Subagent").to_string(),
        description: description.map(str::to_string).unwrap_or(task),
        started_at,
        last_at: if last > 0.0 { last } else { mtime },
        is_running: false,
        is_background: info.get("requestShape").and_then(Value::as_str) == Some("background"),
        depth: info.get("spawnDepth").and_then(Value::as_f64).unwrap_or(1.0),
    };
    CACHE.lock().expect("subagent cache").insert(path.to_path_buf(), (mtime, built.clone()));
    Some((mtime, built))
}

pub fn subagents_of(transcript: &Path, now: f64) -> Vec<SubagentView> {
    let dir = transcript.with_extension("").join("subagents");
    let Ok(names) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut views: Vec<SubagentView> = names
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".jsonl"))
        .filter_map(|name| {
            let path = dir.join(&name);
            let (_, mut v) = view(&path, &name)?;
            v.is_running = now - v.last_at < 30.0 * 60_000.0 && is_running(&path, now, true);
            Some(v)
        })
        .collect();
    views.sort_by(|a, b| b.started_at.total_cmp(&a.started_at));
    views
}
