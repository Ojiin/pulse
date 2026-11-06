# Pulse

A menu bar app for the Claude desktop app: what its sessions and routines are
doing, and what of it needs you. A chat and a routine run are the same thing
here, an item. Tauri 2; everything is worked out in Rust (`src-tauri/src`),
the panel (`ui/`, React) only draws. On Windows the menu bar item is an icon in
the notification area (the tray), and the panel opens by the taskbar; see
*Windows and Codex* in the root README.

## What it shows

- **Menu bar:** one coloured symbol per open state with its count (blocked,
  question, result to look at) and a teal hourglass for commands worth a look,
  a grey check when nothing needs you. What runs shows only in the panel and, if asked, on the Windows taskbar. The image is drawn only when a count changes.
- **Panel** (left click on the item; it closes on a click elsewhere unless it
  is pinned; dragged by its head, pinned or not, it opens where it was put,
  until the turning arrow beside the gear takes it back to its place by the item,
  and while the screen of that place is gone it opens by the item, without the
  arrow, until that screen is back; made
  small with the arrows inward it is only a bar of the counts, what runs
  among them when nothing else is open, that stays open like a pinned panel,
  and a click on a count, on the menu bar item or on a notification that is
  about no one session makes it big again; small or big it keeps the edge
  nearer to the side of its screen, and put too low on its screen for even the
  shortest big panel to fit below its head, it rises just enough, and goes
  back down when it is made small again, if the bar fits there), in English: on top a count per open state that has one ("All
  quiet" when nothing needs you and nothing runs), below them the tabs
  *Open*, *Timeline* and *History*. Every group is a card. *Open* begins with
  the card *Running now*, the one place that changes by the second (each
  running item with what it does now and a line per subagent at work: its
  kind, its task, how long; three shown, the rest behind "+n more"); then
  what needs you, a card per state (in each what lies longer than 24 h on
  top, its age on red; below a rule, *Set aside*: *Later*, *Parked* and *Known*, each on a quieter card that its head folds). *Timeline* shows
  today, now, what comes and what was missed, *History* every session the app
  keeps, archived ones too, by its last activity and grouped by day, with
  what it said last (a search through title, folder and that sentence;
  sessions by default, routines on request; loaded when the tab opens). Old and
  archived Codex answers are read on demand, including compressed rollouts, and
  the same sentence is kept when parking a session from History. What
  a session says itself (its STATUS sentence, the app's summary) stays in the
  language it was said in.
- **An item** reads in three lines: its title, its folder, what Claude (or
  Codex) said.
  On its left a disc in the colour of what it wants (blocked, question,
  result, running; pale when parked, later or done), at its foot a grey
  symbol of what it is: a routine (arrows turning), a task planned once (a
  calendar), a session you started (a window with a prompt), one Claude
  started from another (a window with a spark, "from “…”" names the other), a
  subagent (a small window hanging off its session).
- **Commands worth a look** (on top of *Open*): every command a session runs,
  in the foreground or the background, read from the process table (the
  processes that hold its `…/<session>/tasks/<id>.output` open), with its CPU,
  memory, listening ports and last output. Worth a look, by limits in the
  settings: *forgotten?* (in the background, its session quiet for 60 min or
  archived), *running long* (in the background for 2 h), *stuck* (a session
  waits for it in the foreground for 15 min), *running hot* (most of a core for
  10 min). *Known* folds one away (under *Known* at the foot of *Open*,
  where it can be taken back) until the next reminder
  step since it became worth a look (the hours of the reminders, by default
  2, 4, 12 and 24, then daily), or until it is worth a look for another
  reason; then it counts and is told again, to be marked known once more.
  Background work counts as running while its process runs, however old. A
  command that Claude Code moves to the background when it outruns its
  timeout, or that you move there yourself (Ctrl+B), is named on its card by
  its call's description (else its command), like one started in the
  background; one that a subagent of the session runs is not. On Windows, a
  command that a foreground call leaves running (a dev server started with
  `npm run dev &` ahead of a test run in the same call) stays listed after the
  call returns, though Claude Code then deletes the call's output file: Pulse
  asks for its processes through a handle it opened while the file still had
  its name. It shows as *Command left running* and gets the flags of
  background work (*forgotten?* and *running long*, not *stuck*), though it
  does not keep its session running, nor a later shell call that waits for
  approval from showing as blocked. One whose call returns before Pulse's next
  look (within one refresh, as a bare `npm run dev &` usually does) is not
  seen, nor one whose call returned before Pulse last started.
- **Notifications**, as set under the gear in the panel (stored in
  `settings.json` beside the state): per state (blocked, question or open
  points, result) whether it is told at once when it newly needs you and
  whether it is reminded of while it lies, the same for commands; the hours of the reminders (by
  default 2, 4, 12 and 24) and then daily; a summary each morning at a chosen
  time (by default 8:00), or at the first look after it. By default blocked
  and questions are told at once, everything open is reminded of. Quiet hours
  are left to the system's Focus or Do not disturb. A click on a notification
  about one session opens its chat in its app (Claude or Codex).
- **Appearance**, also under the gear: the panel light, dark or as the
  system is (the default); the window's material follows it. On Windows the
  panel lays a ground of its own over the acrylic, 94 % opaque in light and
  90 % in dark, so dark windows behind it no longer grey the head, the tabs and
  the foot. The acrylic's own tint follows the appearance only on Windows 11
  22H2 and later (build 22523+); Windows 10 and Windows 11 21H2 keep the fixed
  light tint from `tauri.windows.conf.json` beneath it. The app icon in the
  foot of the panel follows it too: `assets/app-icon.svg` dark,
  `assets/app-icon-light.svg` light.
- **Taskbar** (Windows), also under the gear: the menu bar's strip of counts on
  the taskbar beside the tray, on by default (`src-tauri/src/taskbar.rs`); a
  second box adds what is running now with its count, off by default and the
  first to go when room is short.
- Right click on the item: start at login, quit.

## An item's life

running → open → done. Open is one of two kinds:

- a **result** (the work is finished, there is only something to read): once
  you opened it in its app (Claude or Codex) it is done, or you close it with
  *Done*; a Codex CLI result is done at once, as it was read in the terminal.
  Claude's unread dot says what was read, but it gives none to the session it
  has on show, so an answer that ends with Claude not in front counts as unread
  too;
- something that **needs you** (a question, a decision, points still open, or
  a block): reading it settles nothing. It stays open until you close it with
  *Done* or the session goes on and says something new; *Later* sets it
  aside (read, still open) below in *Open*.

Which kind it is the session says itself: the `routine-recorder` plugin asks
every session to end each answer with a line `STATUS: worked|waiting|blocked|idle
| one sentence` (worked is a result, waiting needs you, idle had nothing to
do); the chat shows it, as it shows HTML comments and markdown references. Pulse reads it from the record of
a routine run or from the end of a chat's transcript; a chat from before that
is filed by the app's own summary. A session that puts a choice or a plan to you in the middle of its turn asks
you something, not runs. A session that waits for your approval of a call is
blocked, not running:
a shell command open for 30 s without its process, or an edit open as long
(in auto mode a classifier looks at a command for 10 to 15 s before it runs,
and the session goes on by itself whether it allows it or not). When one
answer makes several calls, those that need approval are asked about in the
order they were made, so Pulse looks only at the first one still open: its
30 s run from the answer's last call or, if later, from when the one before it
came back. While a
subagent or a workflow agent of the same session runs a command, or (on macOS)
a command the session left behind still runs, Pulse cannot tell that command
from the session's own shell call (Claude Code names a command's task only once
its call is done), so a call that waits for approval meanwhile shows as
running, not blocked. On Windows a command left running is told apart (it
shows as *Command left running*) and does not hold such a call up.
A session whose turn ended but that still
has a command, workflow or monitor running in the background (or a background
subagent) is running: it waits for that work and goes on when it is done;
unless its last answer said it waits for you, is blocked or parked, which
comes first (the command shows under *Commands to check*). A workflow, run on
this machine or in a remote session, counts as running from the result that
launched it until Claude Code's notification that it ended, for at most 12 h if
that notification never comes (Claude Code was closed mid-run); the card names
it by the workflow's summary. A workflow starts only from the result of a
Workflow call, so a launch that a session quotes anywhere else (in a file it
reads, in a command's output) starts nothing; a notification that work ended
that a session only quotes in a call's result (a file it reads, a command it
runs) ends nothing. An
item that changes after a mark is open again. Items without activity for 14
days drop out.

## Parking a topic

A topic you set aside for later: say so in its chat ("park das Thema erstmal")
and the session sums up where it stands and ends with `STATUS: parked | …`, or
use *Park* on its line in the panel (in *Open* or *History*).
Parked topics sit folded at the bottom of *Open* with where they stand; they
count nowhere and send no notification, only on Mondays the morning summary
names them. Pulse keeps its own note of each, so it stays however old, also
archived, and as a note when the app no longer keeps the session. A click
opens the session in its app (Claude or Codex) to look at it and it stays
parked; it ends when the session goes on or with *Unpark*, which puts it back
in *Open*. When a chat cannot be opened (for example, Claude or Codex is not
installed), on Windows the foot of the panel says why.

## What it reads and writes

- the Claude app's sessions and scheduled tasks
  (`~/Library/Application Support/Claude/claude-code-sessions`) and its unread
  dots (a copy of its localStorage LevelDB, at most every 20 s),
- the transcripts and subagents in `~/.claude/projects` (only what was added
  since the last look, for a running session),
- the runs the `routine-recorder` plugin writes to
  `~/.claude/routine-recorder/runs` and the STATUS comments it asks for (the
  plugin must stay installed),
- the Codex threads in `~/.codex/state_5.sqlite` and the rollouts in
  `~/.codex/sessions` (`CODEX_HOME` moves both), on every platform,
- on Windows: the Claude app's data under `%LOCALAPPDATA%\Claude-Data\<package>`
  or `%APPDATA%\Claude`, `~/.claude` as `%USERPROFILE%\.claude`, and the
  `%TEMP%\claude\…\tasks\<id>.output` files (and those under
  `CLAUDE_CODE_TMPDIR` or `%TMP%`) whose holders are the commands.

It writes its own state (marks, parked topics, what was notified, the last
summary) to
`~/Library/Application Support/local.claude-dashboard.pulse/state.json`
(Windows: `%APPDATA%\local.claude-dashboard.pulse\state.json`) and
the settings to `settings.json` beside it (on Windows the panel's WebView2
profile, with its remembered tab, size and filters, goes to
`%LOCALAPPDATA%\local.claude-dashboard.pulse`), and
opens chats through `claude://` (or `codex://`) links. When a chat cannot be
opened, for example because the Claude or Codex app is not installed on
Windows, the foot of the panel says why for a few seconds (after a click on a
notification, a notification does), instead of Windows' *Get an app* prompt.
A file watcher (FSEvents on macOS, ReadDirectoryChangesW on Windows) triggers a
refresh when the Claude app's files change or a Codex rollout is created;
changes are gathered for 3 s. Codex is also checked every 5 s, by the sizes of
its recent rollouts and a small read-only query of `state_5.sqlite`, and a
change in either triggers a refresh: Codex appends to a rollout through a
handle it keeps open, which the watcher does not report (on Windows for
certain), and a turn read or a thread renamed changes only that database. The
check also finds a Codex installed after Pulse started. A 30 s tick catches
what time alone changes.

## Build

Rust is pinned in `src-tauri/rust-toolchain.toml` (Tauri needs ≥ 1.90).

```bash
npm run setup        # the Tauri CLI and the panel's packages
npm run build        # → src-tauri/target/release/bundle/macos/Pulse.app (Windows: bundle/nsis/Pulse_<version>_x64-setup.exe)
npm run dev          # the panel with hot reload (vite on 5199)
npm test             # the Rust tests
npm --prefix ui run typecheck   # the panel's types
npm run icons        # app icon (assets/app-icon.svg) and menu bar masks, then the icon set
```

On Windows only a release build hands a second start to the Pulse that already
runs; a debug build (`npm run dev`) runs beside an installed Pulse, and both read
and write the same `%APPDATA%\local.claude-dashboard.pulse\state.json` and
`settings.json`. Each saves through a temporary file of its own, so the last to
save wins with its whole file. On either system, a `state.json` that is there
but does not parse at start (cut short, or written by a later build) is kept
beside it as `state.json.bad`, and Pulse starts afresh: no marks, parked topics
or kept panel place, as at a first start.

Mockups of the design rounds: `../design/mockups` (render with `../design/render.sh`).
