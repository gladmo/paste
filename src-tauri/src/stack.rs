use std::sync::Mutex;
use std::time::{Duration, Instant};

use core_graphics::display::CGDisplay;
use objc2_foundation::NSNotification;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};
use tauri_nspanel::{
    tauri_panel, ManagerExt, PanelBuilder, PanelLevel, StyleMask,
};

const PANEL_LABEL: &str = "stack";

/// Storyboard geometry (PasteStack.storyboardc): 264pt wide, up to 438pt
/// tall, 44pt rows, dark vibrancy.
const STACK_W: f64 = 264.0;
const STACK_H: f64 = 438.0;
const STACK_GAP: f64 = 8.0;

static LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
const TOGGLE_COOLDOWN: Duration = Duration::from_millis(250);

tauri_panel! {
    panel!(StackPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(StackDelegate {
        window_did_resign_key(notification: &NSNotification) -> (),
    })
}

/// Hide the stack when the user switches to another app (same UX as the
/// main overlay).
fn watch_frontmost(app: &AppHandle) {
    use objc2_app_kit::NSWorkspace;
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(400));
        let Some(panel) = handle.get_webview_panel(PANEL_LABEL) else {
            return;
        };
        if !panel.is_visible() {
            continue;
        }
        let Some(front) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
            continue;
        };
        if front.processIdentifier() != std::process::id() as i32 {
            hide(&handle);
        }
    });
}

/// Create the stack panel if needed (hidden).
fn ensure_panel(app: &AppHandle) {
    if app.get_webview_panel(PANEL_LABEL).is_some() {
        return;
    }
    let builder = PanelBuilder::<_, StackPanel>::new(app, PANEL_LABEL)
        .url(WebviewUrl::App("index.html#/stack".into()))
        .level(PanelLevel::Floating)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .with_window(|window| window.decorations(false).transparent(true))
        .size(tauri::Size::Logical(tauri::LogicalSize::new(STACK_W, STACK_H)));
    if let Err(e) = builder.build() {
        eprintln!("stack panel error: {e}");
    }
    if let Some(panel) = app.get_webview_panel(PANEL_LABEL) {
        panel.hide();
    }
    watch_frontmost(app);
}

/// Place the panel against the right screen edge, vertically centered.
fn position(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };
    let displays = CGDisplay::active_displays().unwrap_or_default();
    let Some(display) = displays.first().map(|id| CGDisplay::new(*id)) else {
        return;
    };
    let bounds = display.bounds();
    // CG bounds use a bottom-left origin; tauri positions top-left.
    let x = bounds.origin.x + bounds.size.width - STACK_W - STACK_GAP;
    let cg_bottom = bounds.origin.y + bounds.size.height / 2.0 - STACK_H / 2.0;
    let y_top = bounds.origin.y + bounds.size.height - (cg_bottom + STACK_H);
    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y_top)));
}

pub fn toggle(app: &AppHandle) {
    if let Ok(mut last) = LAST_TOGGLE.lock() {
        if last.is_some_and(|t| t.elapsed() < TOGGLE_COOLDOWN) {
            return;
        }
        *last = Some(Instant::now());
    }
    ensure_panel(app);
    if is_visible(app) {
        hide(app);
    } else {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    ensure_panel(app);
    position(app);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(panel) = handle.get_webview_panel(PANEL_LABEL) {
            panel.show_and_make_key();
        }
        if let Some(window) = handle.get_webview_window(PANEL_LABEL) {
            let _ = window.emit("stack:show", ());
        }
    });
}

pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(PANEL_LABEL) {
            let _ = window.emit("stack:hide", ());
        }
        if let Some(panel) = handle.get_webview_panel(PANEL_LABEL) {
            panel.hide();
        }
    });
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_panel(PANEL_LABEL)
        .map(|p| p.is_visible())
        .unwrap_or(false)
}
