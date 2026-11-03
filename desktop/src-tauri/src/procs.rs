// The commands the sessions run right now, from the Mac's process table: every
// command a session starts, in the foreground or the background, writes to
// /private/tmp/claude-<uid>/<project>/<session>/tasks/<id>.output and holds
// that file open while it runs; the processes that hold it are the command
// (the shell and what it started). Read through libproc, without lsof: one
// look costs a few milliseconds.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

use crate::util::mtime_ms;

// sys/proc_info.h (checked against the SDK with offsetof)
const PROC_PIDLISTFDS: i32 = 1;
const PROC_PIDTASKALLINFO: i32 = 2;
const PROC_PIDFDVNODEPATHINFO: i32 = 2;
const PROC_PIDFDSOCKETINFO: i32 = 3;
const PROX_FDTYPE_VNODE: u32 = 1;
const PROX_FDTYPE_SOCKET: u32 = 2;
const VNODE_PATH_SIZE: usize = 1200;
const VNODE_PATH_AT: usize = 176;
const SOCKET_SIZE: usize = 792;
const SOCKET_KIND_AT: usize = 256;
const SOCKET_LPORT_AT: usize = 268;
const SOCKET_TCP_STATE_AT: usize = 344;
const SOCKINFO_TCP: i32 = 2;
const TSI_S_LISTEN: i32 = 1;

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
struct Found {
    pids: Vec<i32>,
    started: f64,
    cpu_ns: f64,
    memory: u64,
    ports: Vec<u16>,
    path: String,
}

fn read_i32(buffer: &[u8], at: usize) -> i32 {
    i32::from_ne_bytes(buffer[at..at + 4].try_into().expect("4 bytes"))
}

// "<session>/tasks/<id>.output" at the end of a path under claude-<uid>
fn task_of(path: &str) -> Option<(String, String)> {
    if !path.contains("/claude-") {
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

fn pids() -> Vec<i32> {
    // SAFETY: a null buffer asks for the count; the second call fills at most the buffer's size
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }
    let mut pids = vec![0i32; count as usize + 64];
    let bytes = (pids.len() * 4) as i32;
    let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr() as *mut c_void, bytes) };
    pids.truncate(n.max(0) as usize);
    pids.retain(|&p| p > 0);
    pids
}

fn task_info(pid: i32) -> Option<libc::proc_taskallinfo> {
    let mut info: libc::proc_taskallinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_taskallinfo>() as i32;
    // SAFETY: the buffer is a proc_taskallinfo of the size given
    let n = unsafe { libc::proc_pidinfo(pid, PROC_PIDTASKALLINFO, 0, &mut info as *mut _ as *mut c_void, size) };
    (n == size).then_some(info)
}

fn fds(pid: i32) -> Vec<libc::proc_fdinfo> {
    let size = std::mem::size_of::<libc::proc_fdinfo>();
    let mut list: Vec<libc::proc_fdinfo> = vec![unsafe { std::mem::zeroed() }; 256];
    loop {
        let bytes = (list.len() * size) as i32;
        // SAFETY: the buffer holds list.len() proc_fdinfo
        let n = unsafe { libc::proc_pidinfo(pid, PROC_PIDLISTFDS, 0, list.as_mut_ptr() as *mut c_void, bytes) };
        if n <= 0 {
            return Vec::new();
        }
        if n < bytes || list.len() >= 1 << 16 {
            list.truncate(n as usize / size);
            return list;
        }
        list = vec![unsafe { std::mem::zeroed() }; list.len() * 4];
    }
}

fn fd_info(pid: i32, fd: i32, flavor: i32, size: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; size];
    // SAFETY: the buffer is as large as the flavor's struct
    let n = unsafe { libc::proc_pidfdinfo(pid, fd, flavor, buffer.as_mut_ptr() as *mut c_void, size as i32) };
    (n as usize == size).then_some(buffer)
}

fn path_of(pid: i32, fd: i32) -> Option<String> {
    let buffer = fd_info(pid, fd, PROC_PIDFDVNODEPATHINFO, VNODE_PATH_SIZE)?;
    let path = &buffer[VNODE_PATH_AT..];
    let end = path.iter().position(|&b| b == 0).unwrap_or(path.len());
    Some(String::from_utf8_lossy(&path[..end]).into_owned())
}

// a TCP socket that listens: its local port
fn listening_port(pid: i32, fd: i32) -> Option<u16> {
    let buffer = fd_info(pid, fd, PROC_PIDFDSOCKETINFO, SOCKET_SIZE)?;
    let is_listening = read_i32(&buffer, SOCKET_KIND_AT) == SOCKINFO_TCP && read_i32(&buffer, SOCKET_TCP_STATE_AT) == TSI_S_LISTEN;
    is_listening.then(|| u16::from_be(read_i32(&buffer, SOCKET_LPORT_AT) as u16))
}

#[repr(C)]
struct Timebase {
    numer: u32,
    denom: u32,
}

extern "C" {
    fn mach_timebase_info(info: *mut Timebase) -> i32;
}

// CPU time in the task info counts in mach ticks
fn ticks_to_ns() -> f64 {
    static RATIO: LazyLock<f64> = LazyLock::new(|| {
        let mut info = Timebase { numer: 0, denom: 0 };
        // SAFETY: fills the struct
        unsafe { mach_timebase_info(&mut info) };
        if info.denom == 0 { 1.0 } else { info.numer as f64 / info.denom as f64 }
    });
    *RATIO
}

// what the look before saw of each command: when, its CPU time, since when hot
static LAST: LazyLock<Mutex<HashMap<String, (f64, f64, Option<f64>)>>> = LazyLock::new(Default::default);

// a look this short after the one before measures too little to say hot
const MIN_SAMPLE_MS: f64 = 20_000.0;
// most of a core
const HOT: f64 = 0.8;

pub fn commands(now: f64) -> Vec<Command> {
    // SAFETY: no arguments
    let uid = unsafe { libc::getuid() };
    let ratio = ticks_to_ns();
    let mut found: HashMap<(String, String), Found> = HashMap::new();
    for pid in pids() {
        let Some(info) = task_info(pid) else { continue };
        if info.pbsd.pbi_uid != uid {
            continue;
        }
        let list = fds(pid);
        let mut tasks = Vec::new();
        let mut ports = Vec::new();
        for fd in &list {
            match fd.proc_fdtype {
                PROX_FDTYPE_VNODE => {
                    if let Some(task) = path_of(pid, fd.proc_fd).and_then(|p| task_of(&p).map(|t| (t, p))) {
                        if !tasks.iter().any(|(t, _): &((String, String), String)| t == &task.0) {
                            tasks.push(task);
                        }
                    }
                }
                PROX_FDTYPE_SOCKET => ports.extend(listening_port(pid, fd.proc_fd)),
                _ => {}
            }
        }
        for (key, path) in tasks {
            let one = found.entry(key).or_default();
            let started = info.pbsd.pbi_start_tvsec as f64 * 1000.0 + info.pbsd.pbi_start_tvusec as f64 / 1000.0;
            if one.pids.is_empty() || started < one.started {
                one.started = started;
            }
            one.pids.push(pid);
            one.cpu_ns += (info.ptinfo.pti_total_user + info.ptinfo.pti_total_system) as f64 * ratio;
            one.memory += info.ptinfo.pti_resident_size;
            one.ports.extend(&ports);
            one.path = path;
        }
    }

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
