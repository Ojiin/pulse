// The Codex app's threads, beside the Claude app's sessions: its state
// database (~/.codex/state_5.sqlite) names each thread, its folder, whether it
// is archived and whether it has a turn not yet read (the CLI marks nothing
// read: what it answered was read in the terminal); the thread's rollout
// (~/.codex/sessions/…/rollout-*.jsonl) says whether a turn runs now, what it
// does and what it answered last. A Codex thread is an item like a chat; its
// id is "codex_" and the thread's UUID. The database is the app's own and may
// change; a failed read is reported as a problem and leaves Codex out.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::inputs::{Inputs, Session, WINDOW_MS};
use crate::live::Live;
use crate::runs::Summary;
use crate::said::{status_of, Said};
use crate::util::{json_lines, parse_time, read_tail};

pub const PREFIX: &str = "codex_";
// a turn's events lie near the rollout's end; a long turn may begin further back
const TAIL: u64 = 1024 * 1024;
// a turn that started but wrote nothing for this long was cut off (the app closed)
const STALE_MS: f64 = 6.0 * 3_600_000.0;

struct Thread {
    id: String,
    title: String,
    cwd: Option<String>,
    rollout: PathBuf,
    archived: bool,
    created_at: Option<f64>,
    // its last turn (the app also bumps updated_at when it migrates a thread)
    updated_at: Option<f64>,
    unread: bool,
}

// what a rollout's end says
#[derive(Clone, Default)]
struct Turn {
    running: bool,
    started: Option<f64>,
    // the last tool call of the running turn, and how many
    tool: Option<String>,
    tools: usize,
    last_message: Option<String>,
    aborted: bool,
}

static CACHE: LazyLock<Mutex<HashMap<PathBuf, (u64, Turn)>>> = LazyLock::new(Default::default);

// Windows writes long paths with \\?\ before them
fn plain(path: &str) -> String {
    path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
}

fn first_line(text: &str, n: usize) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    crate::util::first_chars(line, n)
}

fn threads(db: &Path, since: f64) -> rusqlite::Result<Vec<Thread>> {
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    // the person's own threads, started in the Codex app or CLI: not a
    // subagent's or a review's, nor one another tool drives (those have no source)
    let mut statement = conn.prepare(
        "SELECT t.id, t.name, t.title, t.cwd, t.rollout_path, t.archived, t.created_at_ms, coalesce(t.recency_at_ms, t.updated_at_ms),
                r.first_unread_turn IS NOT NULL AND coalesce(t.originator, '') <> 'codex-tui'
         FROM threads t LEFT JOIN thread_read_receipts r ON r.thread_id = t.id
         WHERE t.thread_source = 'user' AND coalesce(t.recency_at_ms, t.updated_at_ms) > ?1",
    )?;
    let rows = statement.query_map([since as i64], |row| {
        let name: Option<String> = row.get(1)?;
        let title: Option<String> = row.get(2)?;
        let cwd: Option<String> = row.get(3)?;
        let rollout: String = row.get(4)?;
        Ok(Thread {
            id: row.get(0)?,
            title: name.filter(|n| !n.trim().is_empty()).or_else(|| title.map(|t| first_line(&t, 80))).unwrap_or_default(),
            cwd: cwd.map(|c| plain(&c)),
            rollout: PathBuf::from(plain(&rollout)),
            archived: row.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
            created_at: row.get::<_, Option<i64>>(6)?.map(|v| v as f64),
            updated_at: row.get::<_, Option<i64>>(7)?.map(|v| v as f64),
            unread: row.get(8)?,
        })
    })?;
    rows.collect()
}

fn time_of(line: &Value) -> Option<f64> {
    line.get("timestamp").and_then(Value::as_str).and_then(parse_time)
}

fn apply(turn: &mut Turn, line: &Value) {
    let Some(payload) = line.get("payload") else { return };
    if line.get("type").and_then(Value::as_str) != Some("event_msg") {
        return;
    }
    match payload.get("type").and_then(Value::as_str) {
        Some("task_started") => {
            *turn = Turn { running: true, started: time_of(line), last_message: turn.last_message.take(), ..Default::default() };
        }
        Some("task_complete") => {
            turn.running = false;
            turn.aborted = false;
            if let Some(text) = payload.get("last_agent_message").and_then(Value::as_str).filter(|t| !t.trim().is_empty()) {
                turn.last_message = Some(text.to_string());
            }
        }
        Some("turn_aborted") => {
            turn.running = false;
            turn.aborted = true;
        }
        Some("item_completed") if turn.running => {
            let item = payload.get("item");
            let tool = match item.and_then(|i| i.get("type")).and_then(Value::as_str) {
                Some("CommandExecution") => Some("Shell".to_string()),
                Some("FileChange") => Some("Edit".to_string()),
                Some("McpToolCall") => item.and_then(|i| i.get("tool")).and_then(Value::as_str).map(str::to_string).or(Some("MCP".into())),
                Some("CollabAgentToolCall" | "SubAgentActivity") => Some("Agent".to_string()),
                Some("ImageView") => Some("View image".to_string()),
                _ => None,
            };
            if let Some(tool) = tool {
                turn.tool = Some(tool);
                turn.tools += 1;
            }
        }
        _ => {}
    }
}

fn turn_of(rollout: &Path) -> Option<Turn> {
    let size = std::fs::metadata(rollout).ok()?.len();
    if let Some((known, turn)) = CACHE.lock().expect("codex cache").get(rollout) {
        if *known == size {
            return Some(turn.clone());
        }
    }
    let length = size.min(TAIL);
    let buffer = read_tail(rollout, size, length).ok()?;
    let mut turn = Turn::default();
    for line in json_lines(&String::from_utf8_lossy(&buffer), length < size) {
        apply(&mut turn, &line);
    }
    CACHE.lock().expect("codex cache").insert(rollout.to_path_buf(), (size, turn.clone()));
    Some(turn)
}

// the answer's last paragraph: where a turn says what came of it
fn conclusion(text: &str) -> String {
    let paragraph = text.split("\n\n").map(str::trim).filter(|p| !p.is_empty() && !p.starts_with("STATUS:")).last().unwrap_or("");
    // as plain text: no bold, no code marks
    let line = paragraph.replace("**", "").replace('`', "").split_whitespace().collect::<Vec<_>>().join(" ");
    let short = crate::util::first_chars(&line, 200);
    if short.len() < line.len() { format!("{short}…") } else { short }
}

pub fn read(codex: &Path, inputs: &mut Inputs) {
    let db = codex.join("state_5.sqlite");
    if !db.exists() {
        return;
    }
    let now = inputs.now;
    let list = match threads(&db, now - WINDOW_MS) {
        Ok(list) => list,
        Err(error) => {
            inputs.problems.push(format!("Codex threads unreadable: {error}"));
            return;
        }
    };
    for t in list {
        let id = format!("{PREFIX}{}", t.id);
        let turn = if t.archived { None } else { turn_of(&t.rollout) };
        let turn = turn.unwrap_or_default();
        let written = std::fs::metadata(&t.rollout).map(|m| crate::util::mtime_ms(&m)).unwrap_or(0.0);
        let running = turn.running && now - written < STALE_MS;
        let message = turn.last_message.as_deref().unwrap_or("");
        // a STATUS line, if Codex was asked for one (AGENTS.md), says it as for a chat
        let said = status_of(message);
        let detail = match (&said, turn.aborted) {
            (_, true) => "Interrupted".to_string(),
            (Some(s), _) if !s.text.is_empty() => s.text.clone(),
            _ => conclusion(message),
        };
        // without one, an answer that ends in a question asks the person something
        let asks = said.is_none() && !turn.aborted && message.trim_end().ends_with('?');
        if running {
            inputs.running.insert(id.clone());
            inputs.in_turn.insert(id.clone());
            inputs.live.insert(id.clone(), Live::of_codex(turn.started, turn.tool.clone(), turn.tools));
        }
        if let Some(said) = said {
            inputs.said.insert(id.clone(), Said { state: said.state, text: said.text });
        }
        if t.unread {
            inputs.unread.insert(id.clone());
        }
        inputs.sessions.push(Session {
            id,
            title: Some(t.title).filter(|t| !t.is_empty()),
            cwd: t.cwd,
            archived: t.archived,
            created_at: t.created_at,
            last_activity: t.updated_at.or(Some(written)),
            summary: Summary { category: None, needs_action: if asks { detail.clone() } else { String::new() }, detail },
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str, extra: Value) -> Value {
        let mut payload = serde_json::json!({ "type": kind });
        payload.as_object_mut().expect("object").extend(extra.as_object().expect("object").clone());
        serde_json::json!({ "timestamp": "2026-11-03T18:43:53.723Z", "type": "event_msg", "payload": payload })
    }

    #[test]
    fn a_turn_runs_until_it_completes() {
        let mut turn = Turn::default();
        apply(&mut turn, &event("task_started", serde_json::json!({})));
        apply(&mut turn, &event("item_completed", serde_json::json!({ "item": { "type": "CommandExecution" } })));
        assert!(turn.running);
        assert_eq!((turn.tool.as_deref(), turn.tools), (Some("Shell"), 1));
        assert!(turn.started.is_some());
        apply(&mut turn, &event("task_complete", serde_json::json!({ "last_agent_message": "Done.\n\nSTATUS: worked | Tests pass" })));
        assert!(!turn.running);
        let said = status_of(turn.last_message.as_deref().unwrap_or("")).expect("status");
        assert_eq!((said.state.as_str(), said.text.as_str()), ("worked", "Tests pass"));
    }

    #[test]
    fn an_aborted_turn_keeps_the_answer_before() {
        let mut turn = Turn::default();
        apply(&mut turn, &event("task_complete", serde_json::json!({ "last_agent_message": "First answer" })));
        apply(&mut turn, &event("task_started", serde_json::json!({})));
        apply(&mut turn, &event("turn_aborted", serde_json::json!({ "reason": "interrupted" })));
        assert!(!turn.running && turn.aborted);
        assert_eq!(turn.last_message.as_deref(), Some("First answer"));
    }

    #[test]
    fn the_conclusion_is_the_last_paragraph() {
        assert_eq!(conclusion("Looked at it.\n\nShould I merge\n it?\n\nSTATUS: waiting | x"), "Should I merge it?");
        assert_eq!(plain(r"\\?\C:\Users\me\work"), r"C:\Users\me\work");
    }
}
