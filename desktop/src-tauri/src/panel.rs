// The panel under the menu bar item: it opens below the item, closes again
// when it loses focus, unless it is pinned; pinned it stays until the item is
// clicked again or it is closed. Moved by its head, pinned or not, it opens
// where it was put, until its position is reset. Made small it shows only the
// counts and stays open, as a pinned one does; a click on the item or on a
// notification that is about no one session opens it big. Its place is that of the big
// panel; smaller or bigger it keeps the edge nearer to the side of its screen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Rect, Theme, WebviewWindow};

use crate::settings::Appearance;
use crate::store::{save, STORE};

pub const LABEL: &str = "panel";
const WIDTH: f64 = 420.0;
const MAX_HEIGHT: f64 = 760.0;

pub static PINNED: AtomicBool = AtomicBool::new(false);
pub static COMPACT: AtomicBool = AtomicBool::new(false);
// a click on the item takes the focus from the panel first: a panel that just
// closed that way stays closed
static CLOSED_AT: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(Default::default);
// where the panel opens below the item, the last time it was worked out
static ANCHOR: LazyLock<Mutex<Option<PhysicalPosition<i32>>>> = LazyLock::new(Default::default);
// where Pulse itself last moved the panel: that move is not the person's
static EXPECTED: LazyLock<Mutex<Option<PhysicalPosition<i32>>>> = LazyLock::new(Default::default);

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = window(app) {
        let _ = w.hide();
    }
    keep_place();
}

// the place it was moved to, written when it closes rather than at every step of a drag
fn keep_place() {
    let store = STORE.lock().expect("store");
    if let Err(error) = save(&store) {
        eprintln!("store: {error}");
    }
}

// a place on one of the screens, else none (a screen that is gone)
fn on_screen(w: &WebviewWindow, at: [i32; 2]) -> bool {
    w.available_monitors().unwrap_or_default().iter().any(|m| {
        let (p, s) = (m.position(), m.size());
        (p.x..p.x + s.width as i32 - 40).contains(&at[0]) && (p.y..p.y + s.height as i32 - 40).contains(&at[1])
    })
}

fn big_width(w: &WebviewWindow) -> i32 {
    (WIDTH * w.scale_factor().unwrap_or(2.0)).round() as i32
}

fn current_width(w: &WebviewWindow) -> i32 {
    w.outer_size().map_or_else(|_| big_width(w), |s| s.width as i32)
}

// whether a panel at x as wide as this sits on the right half of its screen
fn is_right(w: &WebviewWindow, x: i32, width: i32) -> bool {
    let center = x + width / 2;
    let monitors = w.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| (m.position().x..m.position().x + m.size().width as i32).contains(&center));
    monitor.is_some_and(|m| center > m.position().x + m.size().width as i32 / 2)
}

// from the big panel's left edge to where a panel this wide stands, and back
fn x_for(w: &WebviewWindow, big_x: i32, width: i32) -> i32 {
    let big = big_width(w);
    if is_right(w, big_x, big) { big_x + big - width } else { big_x }
}

fn big_x_of(w: &WebviewWindow, x: i32, width: i32) -> i32 {
    if is_right(w, x, width) { x + width - big_width(w) } else { x }
}

fn move_to(w: &WebviewWindow, at: PhysicalPosition<i32>) {
    if w.outer_position().is_ok_and(|p| p == at) {
        return;
    }
    *EXPECTED.lock().expect("panel") = Some(at);
    let _ = w.set_position(at);
}

// big again: the page hears it and grows (App.tsx)
fn make_big(app: &AppHandle) {
    if COMPACT.swap(false, Ordering::SeqCst) {
        let _ = app.emit_to(LABEL, "size", "big");
    }
}

// a click on the item: small it grows, big it closes, closed it opens big
pub fn toggle(app: &AppHandle, rect: Rect) {
    let Some(w) = window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        if COMPACT.load(Ordering::SeqCst) {
            make_big(app);
            let _ = w.set_focus();
        } else {
            hide(app);
        }
        return;
    }
    if CLOSED_AT.lock().expect("panel").is_some_and(|at| at.elapsed() < Duration::from_millis(300)) {
        return;
    }
    show(app, &w, rect);
}

// a click on a notification about no one session: the panel, big
pub fn open_big(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        make_big(app);
        let _ = w.set_focus();
    } else if let Some(rect) = crate::tray::rect(app) {
        show(app, &w, rect);
    }
}

fn show(app: &AppHandle, w: &WebviewWindow, rect: Rect) {
    make_big(app);
    let scale = w.scale_factor().unwrap_or(2.0);
    let item = rect.position.to_physical::<f64>(scale);
    let item_size = rect.size.to_physical::<f64>(scale);
    let size = big_width(w) as f64;
    let mut x = item.x + item_size.width / 2.0 - size / 2.0;
    // keep it on the screen of the item
    if let Ok(Some(monitor)) = w.current_monitor().or_else(|_| w.primary_monitor()) {
        let (left, right) = (monitor.position().x as f64, monitor.position().x as f64 + monitor.size().width as f64);
        x = x.clamp(left + 8.0 * scale, right - size - 8.0 * scale);
    }
    let anchor = PhysicalPosition::new(x.round() as i32, (item.y + item_size.height + 6.0 * scale).round() as i32);
    *ANCHOR.lock().expect("panel") = Some(anchor);
    let placed = STORE.lock().expect("store").panel_at.filter(|&at| on_screen(w, at));
    let [x, y] = placed.unwrap_or([anchor.x, anchor.y]);
    move_to(w, PhysicalPosition::new(x_for(w, x, current_width(w)), y));
    let _ = w.show();
    let _ = w.set_focus();
    crate::push_last(app);
}

pub fn is_visible(app: &AppHandle) -> bool {
    window(app).is_some_and(|w| w.is_visible().unwrap_or(false))
}

pub fn on_blur(app: &AppHandle) {
    if !PINNED.load(Ordering::SeqCst) && !COMPACT.load(Ordering::SeqCst) {
        hide(app);
        *CLOSED_AT.lock().expect("panel") = Some(Instant::now());
    }
}

// the panel moved: away from its place below the item, the person put it
// somewhere; the panel hears whether it has a place of its own
pub fn on_moved(app: &AppHandle, at: PhysicalPosition<i32>) {
    let Some(w) = window(app).filter(|w| w.is_visible().unwrap_or(false)) else { return };
    let near = |a: PhysicalPosition<i32>, b: PhysicalPosition<i32>| (a.x - b.x).abs() <= 2 && (a.y - b.y).abs() <= 2;
    {
        let mut expected = EXPECTED.lock().expect("panel");
        if expected.is_some_and(|e| near(e, at)) {
            *expected = None;
            return;
        }
    }
    let big = PhysicalPosition::new(big_x_of(&w, at.x, current_width(&w)), at.y);
    let is_anchor = ANCHOR.lock().expect("panel").is_some_and(|a| near(a, big));
    let mut store = STORE.lock().expect("store");
    let was_placed = store.panel_at.is_some();
    if !is_anchor {
        store.panel_at = Some([big.x, big.y]);
    }
    if store.panel_at.is_some() != was_placed {
        let _ = app.emit_to(LABEL, "placed", true);
    }
}

pub fn is_placed() -> bool {
    STORE.lock().expect("store").panel_at.is_some()
}

// back below the item
pub fn reset_place(app: &AppHandle) {
    STORE.lock().expect("store").panel_at = None;
    keep_place();
    if let (Some(w), Some(anchor)) = (window(app), *ANCHOR.lock().expect("panel")) {
        move_to(&w, PhysicalPosition::new(x_for(&w, anchor.x, current_width(&w)), anchor.y));
    }
    let _ = app.emit_to(LABEL, "placed", false);
}

// full width unless a width is given (small: as wide as the counts)
pub fn set_size(app: &AppHandle, width: Option<f64>, height: f64) {
    if let Some(w) = window(app) {
        let min = if width.is_some() { 24.0 } else { 200.0 };
        let width = width.map_or(WIDTH, |w| w.clamp(80.0, 640.0));
        let (Ok(at), old) = (w.outer_position(), current_width(&w)) else { return };
        let big_x = big_x_of(&w, at.x, old);
        let _ = w.set_size(LogicalSize::new(width, height.clamp(min, MAX_HEIGHT)));
        let new = (width * w.scale_factor().unwrap_or(2.0)).round() as i32;
        if new != old {
            move_to(&w, PhysicalPosition::new(x_for(&w, big_x, new), at.y));
        }
    }
}

// light or dark as chosen, or as the system is; the material and the page follow
pub fn set_appearance(app: &AppHandle, appearance: Appearance) {
    if let Some(w) = window(app) {
        let _ = w.set_theme(match appearance {
            Appearance::System => None,
            Appearance::Light => Some(Theme::Light),
            Appearance::Dark => Some(Theme::Dark),
        });
    }
}
