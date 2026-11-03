// The commands the sessions run right now, from the process table: every
// command a session starts, in the foreground or the background, writes to
// <temp>/claude[-<uid>]/<project>/<session>/tasks/<id>.output and holds that
// file open while it runs; the processes that hold it are the command (the
// shell and what it started). On macOS through libproc (procs_mac.rs), on
// Windows through the Restart Manager (procs_win.rs).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[cfg(target_os = "macos")]
use crate::procs_mac::scan;
#[cfg(windows)]
use crate::procs_win::scan;
use crate::util::mtime_ms;

// no process table read elsewhere: no commands
#[cfg(not(any(target_os = "macos", windows)))]
fn scan() -> HashMap<(String, String), Found> {
    HashMap::new()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    // the session's id in the transcript's name, and the command's own id
    pub session: String,
    pub id: String,
    pub started: f64,
    // when it last wrote something
    pub last_output: f64,
    // cores in use since the look before (1.0 is one core fully)
    pub cpu: f64,
    // since when it uses most of a core without a break
    pub hot_since: Option<f64>,
    pub memory: u64,
    pub ports: Vec<u16>,
}

#[derive(Default)]
pub struct Found {
    pub pids: Vec<i32>,
    pub started: f64,
    pub cpu_ns: f64,
    pub memory: u64,
    pub ports: Vec<u16>,
    pub path: String,
}

// "<session>/tasks/<id>.output" at the end of a path under claude or claude-<uid>
pub fn task_of(path: &str) -> Option<(String, String)> {
    let path = path.replace('\\', "/");
    if !path.split('/').any(|part| part == "claude" || part.starts_with("claude-")) {
        return None;
    }
    let mut parts = path.rsplit('/');
    let file = parts.next()?;
    let id = file.strip_suffix(".output")?;
    if parts.next()? != "tasks" {
        return None;
    }
    let session = parts.next()?;
    let is_uuid = session.len() == 36 && session.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (is_uuid && !id.is_empty()).then(|| (session.to_string(), id.to_string()))
}

// what the look before saw of each command: when, its CPU time, since when hot
static LAST: LazyLock<Mutex<HashMap<String, (f64, f64, Option<f64>)>>> = LazyLock::new(Default::default);

// a look this short after the one before measures too little to say hot
const MIN_SAMPLE_MS: f64 = 20_000.0;
// most of a core
const HOT: f64 = 0.8;

pub fn commands(now: f64) -> Vec<Command> {
    let found = scan();
    let mut last = LAST.lock().expect("procs last");
    let mut seen = HashMap::new();
    let mut out = Vec::new();
    for ((session, id), mut one) in found {
        let key = format!("{session}/{id}");
        let (cpu, hot_since, sample) = match last.get(&key) {
            Some(&(at, cpu_ns, hot)) if now - at < MIN_SAMPLE_MS => (0.0, hot, (at, cpu_ns, hot)),
            Some(&(at, cpu_ns, hot)) => {
                let cpu = ((one.cpu_ns - cpu_ns) / 1e6 / (now - at)).max(0.0);
                let hot = if cpu >= HOT { Some(hot.unwrap_or(at)) } else { None };
                (cpu, hot, (now, one.cpu_ns, hot))
            }
            None => (0.0, None, (now, one.cpu_ns, None)),
        };
        seen.insert(key, sample);
        one.ports.sort_unstable();
        one.ports.dedup();
        let last_output = std::fs::metadata(&one.path).map(|m| mtime_ms(&m)).unwrap_or(one.started);
        out.push(Command { session, id, started: one.started, last_output, cpu, hot_since, memory: one.memory, ports: one.ports });
    }
    *last = seen;
    out.sort_by(|a, b| a.started.total_cmp(&b.started));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_of_an_output_path() {
        let session = "7e3c51a2-0b4d-4c8e-9f61-2a5d8b0c4e17";
        let path = format!("/private/tmp/claude-501/-Users-me-Work-shop-api/{session}/tasks/b1x2y3z4w.output");
        assert_eq!(task_of(&path), Some((session.to_string(), "b1x2y3z4w".to_string())));
        assert_eq!(task_of("/private/tmp/claude-501/x/tasks/a.output"), None);
        assert_eq!(task_of(&format!("/tmp/other/{session}/tasks/a.output")), None);
        assert_eq!(task_of(&format!("/private/tmp/claude-501/p/{session}/tasks/a.log")), None);
        let windows = format!(r"C:\Users\me\AppData\Local\Temp\claude\C--Users-me-shop\{session}\tasks\b1x2y3z4w.output");
        assert_eq!(task_of(&windows), Some((session.to_string(), "b1x2y3z4w".to_string())));
    }

    // a command of this very test: a child that holds such a file open and listens on a port
    #[test]
    fn a_running_command_is_found_with_its_port() {
        let session = "00000000-0000-4000-8000-000000000001";
        let dir = std::env::temp_dir().join(format!("claude-test/p/{session}/tasks"));
        std::fs::create_dir_all(&dir).expect("dir");
        let output = dir.join("t1.output");
        let file = std::fs::File::create(&output).expect("file");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listen");
        let port = listener.local_addr().expect("addr").port();
        let found = commands(crate::util::now_ms());
        let me = found.iter().find(|c| c.session == session && c.id == "t1").expect("found");
        assert!(me.ports.contains(&port), "{:?} lacks {port}", me.ports);
        assert!(me.memory > 0);
        drop((file, listener));
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("claude-test"));
    }
}
