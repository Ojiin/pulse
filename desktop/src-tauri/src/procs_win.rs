// The commands' processes on Windows. Claude Code writes each command's output
// to %TEMP%\claude\<project>\<session>\tasks\<id>.output; a file that cannot
// be opened without sharing write access is held open by a writer, and the
// Restart Manager names the processes that hold it. Their times and memory
// come from the process itself, their listening ports from the TCP tables.

use std::collections::HashMap;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_MORE_DATA, FILETIME};
use windows_sys::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::RestartManager::{RmEndSession, RmGetList, RmRegisterResources, RmStartSession, CCH_RM_SESSION_KEY, RM_PROCESS_INFO};
use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

use crate::procs::{task_of, Found};

const SHARING_VIOLATION: i32 = 32;
// from 1601 to 1970 in 100 ns steps
const EPOCH_DIFF: u64 = 116_444_736_000_000_000;

// every <id>.output under the temp folder's claude and claude-* folders
fn outputs() -> Vec<PathBuf> {
    let mut list = Vec::new();
    let roots = std::fs::read_dir(std::env::temp_dir()).into_iter().flatten().filter_map(Result::ok);
    for root in roots.filter(|e| e.file_name().to_str().is_some_and(|n| n == "claude" || n.starts_with("claude-"))) {
        for project in children(&root.path()) {
            for session in children(&project) {
                for file in children(&session.join("tasks")) {
                    if file.extension().is_some_and(|x| x == "output") {
                        list.push(file);
                    }
                }
            }
        }
    }
    list
}

fn children(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok).map(|e| e.path()).collect()
}

// a writer holds it: it does not open while others may write
fn is_held(path: &Path) -> bool {
    match std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(path) {
        Ok(_) => false,
        Err(e) => e.raw_os_error() == Some(SHARING_VIOLATION),
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

// the processes that hold the file open
fn holders(path: &Path) -> Vec<u32> {
    let mut session = 0u32;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    // SAFETY: the key buffer holds CCH_RM_SESSION_KEY + 1 characters
    if unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) } != 0 {
        return Vec::new();
    }
    let name = wide(path);
    let names = [name.as_ptr()];
    let mut pids = Vec::new();
    // SAFETY: one resource name, no applications or services
    if unsafe { RmRegisterResources(session, 1, names.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null()) } == 0 {
        let mut list: Vec<RM_PROCESS_INFO> = Vec::new();
        for _ in 0..3 {
            let (mut needed, mut count, mut reasons) = (0u32, list.len() as u32, 0u32);
            // SAFETY: the buffer holds `count` entries
            let status = unsafe { RmGetList(session, &mut needed, &mut count, list.as_mut_ptr(), &mut reasons) };
            if status == ERROR_MORE_DATA {
                list = vec![unsafe { std::mem::zeroed() }; needed as usize + 4];
                continue;
            }
            if status == 0 {
                pids = list[..count as usize].iter().map(|p| p.Process.dwProcessId).collect();
            }
            break;
        }
    }
    // SAFETY: the session was started above
    unsafe { RmEndSession(session) };
    pids
}

fn ticks(t: &FILETIME) -> u64 {
    (t.dwHighDateTime as u64) << 32 | t.dwLowDateTime as u64
}

// started (ms since 1970), CPU time (ns), memory in use
fn process_info(pid: u32) -> Option<(f64, f64, u64)> {
    // SAFETY: a handle for queries only, closed below
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    let mut memory: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the handle is open; each out pointer is a struct of its type
    let times = unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) } != 0;
    let has_memory = unsafe { K32GetProcessMemoryInfo(handle, &mut memory, size) } != 0;
    unsafe { CloseHandle(handle) };
    if !times {
        return None;
    }
    let started = ticks(&created).saturating_sub(EPOCH_DIFF) as f64 / 10_000.0;
    let cpu_ns = (ticks(&kernel) + ticks(&user)) as f64 * 100.0;
    Some((started, cpu_ns, if has_memory { memory.WorkingSetSize as u64 } else { 0 }))
}

// one TCP table of listening sockets with their owners, as raw bytes
fn tcp_table(family: u16) -> Vec<u8> {
    let mut size = 0u32;
    // SAFETY: a null buffer asks for the size
    unsafe { GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, family as u32, TCP_TABLE_OWNER_PID_LISTENER, 0) };
    for _ in 0..3 {
        let mut buffer = vec![0u8; size as usize + 1024];
        size = buffer.len() as u32;
        // SAFETY: the buffer is `size` bytes long
        let status = unsafe { GetExtendedTcpTable(buffer.as_mut_ptr() as *mut c_void, &mut size, 0, family as u32, TCP_TABLE_OWNER_PID_LISTENER, 0) };
        if status == 0 {
            buffer.truncate(size as usize);
            return buffer;
        }
    }
    Vec::new()
}

// the rows that follow the table's count
fn rows<T: Copy>(table: &[u8]) -> Vec<T> {
    if table.len() < 4 {
        return Vec::new();
    }
    let count = u32::from_ne_bytes(table[..4].try_into().expect("4 bytes")) as usize;
    let at = std::mem::align_of::<T>().max(4);
    let size = std::mem::size_of::<T>();
    (0..count)
        .take_while(|i| at + (i + 1) * size <= table.len())
        // SAFETY: inside the buffer; read unaligned as the buffer is bytes
        .map(|i| unsafe { std::ptr::read_unaligned(table[at + i * size..].as_ptr() as *const T) })
        .collect()
}

// by process: the TCP ports it listens on
fn listening() -> HashMap<u32, Vec<u16>> {
    let mut ports: HashMap<u32, Vec<u16>> = HashMap::new();
    let port = |raw: u32| u16::from_be(raw as u16);
    for row in rows::<MIB_TCPROW_OWNER_PID>(&tcp_table(AF_INET)) {
        ports.entry(row.dwOwningPid).or_default().push(port(row.dwLocalPort));
    }
    for row in rows::<MIB_TCP6ROW_OWNER_PID>(&tcp_table(AF_INET6)) {
        ports.entry(row.dwOwningPid).or_default().push(port(row.dwLocalPort));
    }
    ports
}

pub fn scan() -> HashMap<(String, String), Found> {
    let mut found: HashMap<(String, String), Found> = HashMap::new();
    let held: Vec<(PathBuf, (String, String))> = outputs()
        .into_iter()
        .filter_map(|p| task_of(&p.to_string_lossy()).map(|t| (p, t)))
        .filter(|(p, _)| is_held(p))
        .collect();
    if held.is_empty() {
        return found;
    }
    let ports = listening();
    for (path, key) in held {
        let one = found.entry(key).or_default();
        for pid in holders(&path) {
            let Some((started, cpu_ns, memory)) = process_info(pid) else { continue };
            if one.pids.is_empty() || started < one.started {
                one.started = started;
            }
            one.pids.push(pid as i32);
            one.cpu_ns += cpu_ns;
            one.memory += memory;
            one.ports.extend(ports.get(&pid).into_iter().flatten());
        }
        one.path = path.to_string_lossy().into_owned();
    }
    // a file held by a process that is gone by now
    found.retain(|_, one| !one.pids.is_empty());
    found
}
