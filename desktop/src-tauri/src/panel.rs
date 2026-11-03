// The panel under the menu bar item: it opens below the item, closes again
// when it loses focus, unless it is pinned; pinned it stays until the item is
// clicked again or it is closed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Rect, Theme, WebviewWindow};

use crate::settings::Appearance;

pub const LABEL: &str = "panel";
const WIDTH: f64 = 420.0;
const MAX_HEIGHT: f64 = 760.0;

pub static PINNED: AtomicBool = AtomicBool::new(false);
// a click on the item takes the focus from the panel first: a panel that just
// closed that way stays closed
static CLOSED_AT: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(Default::default);

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = window(app) {
        let _ = w.hide();
    }
}

pub fn toggle(app: &AppHandle, rect: Rect) {
    let Some(w) = window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    if CLOSED_AT.lock().expect("panel").is_some_and(|at| at.elapsed() < Duration::from_millis(300)) {
        return;
    }
    let scale = w.scale_factor().unwrap_or(2.0);
    let item = rect.position.to_physical::<f64>(scale);
    let item_size = rect.size.to_physical::<f64>(scale);
    let size = w.outer_size().map(|s| s.width as f64).unwrap_or(WIDTH * scale);
    let mut x = item.x + item_size.width / 2.0 - size / 2.0;
    // keep it on the screen of the item
    if let Ok(Some(monitor)) = w.current_monitor().or_else(|_| w.primary_monitor()) {
        let (left, right) = (monitor.position().x as f64, monitor.position().x as f64 + monitor.size().width as f64);
        x = x.clamp(left + 8.0 * scale, right - size - 8.0 * scale);
    }
    let _ = w.set_position(PhysicalPosition::new(x, item.y + item_size.height + 6.0 * scale));
    let _ = w.show();
    let _ = w.set_focus();
    crate::push_last(app);
}

pub fn is_visible(app: &AppHandle) -> bool {
    window(app).is_some_and(|w| w.is_visible().unwrap_or(false))
}

pub fn on_blur(app: &AppHandle) {
    if !PINNED.load(Ordering::SeqCst) {
        hide(app);
        *CLOSED_AT.lock().expect("panel") = Some(Instant::now());
    }
}

pub fn set_height(app: &AppHandle, height: f64) {
    if let Some(w) = window(app) {
        let _ = w.set_size(LogicalSize::new(WIDTH, height.clamp(200.0, MAX_HEIGHT)));
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
