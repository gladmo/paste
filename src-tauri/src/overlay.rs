use std::sync::Mutex;
use std::time::{Duration, Instant};

use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::NSWorkspace;
use tauri::{
    utils::config::WindowEffectsConfig, window::Effect, window::EffectState, AppHandle, Emitter,
    LogicalPosition, LogicalSize, Manager, Position, Size, WebviewUrl,
};
use tauri_nspanel::{tauri_panel, ManagerExt, PanelBuilder, PanelLevel, StyleMask};

const PANEL_LABEL: &str = "overlay";

/// Ignore toggle requests that arrive while the previous show/hide fade
/// animation (≈160ms) is still playing, so mashing the hotkey can't make the
/// panel flicker or jump.
const TOGGLE_COOLDOWN: Duration = Duration::from_millis(300);

/// Panel geometry when shown: spans the whole width of the cursor's screen,
/// sits flush against the bottom edge like the macOS Dock.
const PANEL_HEIGHT: f64 = 368.0;
const PANEL_BOTTOM_GAP: f64 = 0.0;

static LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);

/// The logical panel size most recently applied, so show() skips the async
/// resize (and its visible re-layout) unless the size actually changed.
static CURRENT_SIZE: Mutex<Option<(f64, f64)>> = Mutex::new(None);

/// The app that was frontmost before the overlay took over; the ⌘V from
/// paste_clip is posted directly to this pid.
static LAST_TARGET: Mutex<Option<i32>> = Mutex::new(None);

/// Frontmost app while the panel is up. When it changes to something that is
/// not us, the user clicked away and the panel closes.
static LAST_FRONT: Mutex<Option<i32>> = Mutex::new(None);

/// When the panel was last shown. macOS briefly reshuffles the key window
/// right after a non-activating panel grabs key focus while its app is not
/// active; resign notifications inside this grace window are ignored so the
/// panel does not flash away the moment the hotkey opens it.
static SHOWN_AT: Mutex<Option<Instant>> = Mutex::new(None);
const RESIGN_GRACE: Duration = Duration::from_millis(300);

/// Prepare-then-show: `show()` only asks the webview to reset + refetch
/// ("overlay:show"); the panel is actually revealed by `complete_show`
/// when the webview reports readiness (or after a short fallback). This
/// keeps stale content from flashing when the panel reappears.
static SHOW_PENDING: Mutex<bool> = Mutex::new(false);
/// Debug builds reload the webview on hide (dev freshness), so a summon can
/// race a page load — wait long enough that the fallback only fires when the
/// webview is genuinely stuck, never as "gray panel while vite loads".
const SHOW_FALLBACK: Duration =
    Duration::from_millis(if cfg!(debug_assertions) { 4000 } else { 250 });

/// Record the frontmost app if it is not us. Called when the panel shows
/// (the app the user was just using) and when the panel resigns key status
/// (the app the user clicked away to). Never clobbers the stored target
/// while we are still the frontmost app ourselves.
fn capture_target() {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return;
    };
    let pid = app.processIdentifier();
    if pid == std::process::id() as i32 {
        return;
    }
    let name = app
        .localizedName()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "<unknown>".into());
    crate::paste::log_info(&format!("[overlay] target app: {name} pid={pid}"));
    if let Ok(mut last) = LAST_TARGET.lock() {
        *last = Some(pid);
    }
    if let Ok(mut last) = LAST_FRONT.lock() {
        *last = Some(pid);
    }
}

/// The pid ⌘V should be posted to, if we captured one.
pub fn target_pid() -> Option<i32> {
    LAST_TARGET.lock().ok().and_then(|guard| *guard)
}

tauri_panel! {
    panel!(OverlayPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(OverlayDelegate {
        window_did_resign_key(notification: &NSNotification) -> (),
    })
}

/// Create the overlay panel (hidden until summoned by hotkey or tray click).
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let panel = PanelBuilder::<_, OverlayPanel>::new(app, PANEL_LABEL)
        .url(WebviewUrl::App("index.html".into()))
        .level(PanelLevel::Floating)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .with_window(|window| window.decorations(false).transparent(true))
        .size(Size::Logical(LogicalSize::new(680.0, 480.0)))
        .build()?;

    let delegate = OverlayDelegate::new();
    let handle = app.clone();
    delegate.window_did_resign_key(move |_notification| {
        // The original app hides its panel from windowDidResignKey: (the
        // selector is present in the Paste binary), so clicking anywhere
        // outside the panel — including another window of the app that was
        // frontmost when the hotkey fired — dismisses it immediately.
        let Ok(panel) = handle.get_webview_panel(PANEL_LABEL) else {
            return;
        };
        if !panel.is_visible() {
            return;
        }
        let shown_at = SHOWN_AT.lock().ok().and_then(|g| *g);
        if shown_at.is_some_and(|t| t.elapsed() < RESIGN_GRACE) {
            // Key-window reshuffle right after show, not a user click-away.
            crate::paste::log_info("[overlay] resign-key ignored (grace)");
            return;
        }
        crate::paste::log_info("[overlay] resign-key -> hide");
        crate::overlay::hide(&handle);
    });
    panel.set_event_handler(Some(delegate.as_ref()));

    panel.hide();

    // The original panel is a translucent HUD material: its colour visibly
    // follows the desktop behind it (two reference screenshots over different
    // wallpapers show the same UI at #27..#2a vs #3d..#61 grey). Apply real
    // macOS vibrancy behind the transparent webview instead of painting a
    // flat fill.
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        if let Err(err) = window.set_effects(Some(WindowEffectsConfig {
            effects: vec![Effect::HudWindow],
            state: Some(EffectState::FollowsWindowActiveState),
            radius: None,
            color: None,
        })) {
            crate::paste::log_info(&format!("[overlay] set_effects failed: {err}"));
        } else {
            crate::paste::log_info("[overlay] vibrancy applied (HudWindow)");
        }
    }

    // Close the panel when the user actually switches apps: watch the
    // frontmost app while the panel is visible. (A non-activating panel
    // frequently resigns its key window without the user doing anything, so
    // resign events alone can no longer be trusted to decide "clicked away".)
    {
        let handle = app.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));
            let Ok(panel) = handle.get_webview_panel(PANEL_LABEL) else {
                continue;
            };
            if !panel.is_visible() {
                continue;
            }
            let Some(front) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
                continue;
            };
            let front_pid = front.processIdentifier();
            let known = LAST_FRONT.lock().ok().and_then(|guard| *guard);
            let changed = known.is_some_and(|last| last != front_pid);
            if changed && front_pid != std::process::id() as i32 {
                crate::paste::log_info(&format!(
                    "[overlay] frontmost changed to pid {front_pid}, closing"
                ));
                crate::overlay::hide(&handle);
            }
        });
    }

    Ok(())
}

/// Show the panel centered at the bottom edge of the screen containing the
/// cursor. The webview is told to start its fade-in only after the panel is
/// actually on screen, so the animation is visible from its first frame.
///
/// Important: we must NOT activate ourselves (no activateIgnoringOtherApps).
/// The panel is a non-activating panel: it becomes the key window without
/// taking frontmost status, so the previously active app keeps its key
/// window and first responder (its caret keeps blinking, like pasteapp.io).
/// A paste ⌘V posted globally then lands in that app's key window with no
/// focus switch at all.
/// Run the panel operation on the main thread, as AppKit requires: the
/// private WindowManagement paths behind show_and_make_key/hide SIGILL when
/// called from a background thread on macOS 14+.
fn with_panel_on_main<F>(app: &AppHandle, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let _ = app.run_on_main_thread(f);
}

pub fn show(app: &AppHandle) {
    crate::paste::log_info("[overlay] show");
    capture_target();
    position_bottom(app);
    if let Ok(mut pending) = SHOW_PENDING.lock() {
        *pending = true;
    }
    // Ask the webview to reset state and reload data while the panel is
    // still hidden; it reveals the panel via the `overlay_ready` command.
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.emit("overlay:show", ());
    }
    // Never leave the panel stuck hidden if the webview is slow or dead.
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(SHOW_FALLBACK);
        complete_show(&handle);
    });
}

/// Reveal the panel if a show is still pending. Returns whether this call
/// performed the reveal (first consumer wins; later calls are no-ops).
fn complete_show(app: &AppHandle) -> bool {
    let was_pending = SHOW_PENDING.lock().map(|mut p| std::mem::replace(&mut *p, false)).unwrap_or(false);
    if !was_pending {
        return false;
    }
    let handle = app.clone();
    with_panel_on_main(app, move || {
        let Ok(panel) = handle.get_webview_panel(PANEL_LABEL) else {
            crate::paste::log_info("[overlay] show: panel not found");
            return;
        };
        panel.show_and_make_key();
        if let Ok(mut shown) = SHOWN_AT.lock() {
            *shown = Some(Instant::now());
        }
        // Drive the fade-in from the moment the panel is really visible —
        // rAF guessing in the frontend raced the main-thread reveal.
        if let Some(window) = handle.get_webview_window(PANEL_LABEL) {
            let _ = window.emit("overlay:revealed", ());
        }
    });
    true
}

/// Called by the frontend (`overlay_ready`) once the UI is reset and the
/// data is loaded — the panel is revealed right then, so the user never
/// sees the previous state flash before the fade-in.
pub fn ready(app: &AppHandle) {
    if complete_show(app) {
        crate::paste::log_info("[overlay] ready → show");
    }
}

pub fn hide(app: &AppHandle) {
    crate::paste::log_info("[overlay] hide");
    if let Ok(mut pending) = SHOW_PENDING.lock() {
        *pending = false;
    }
    if let Ok(mut last) = LAST_FRONT.lock() {
        *last = None;
    }
    let window = app.get_webview_window(PANEL_LABEL);
    let handle = app.clone();
    with_panel_on_main(app, move || {
        if let Some(window) = &window {
            let _ = window.emit("overlay:hide", ());
        }
        let Ok(panel) = handle.get_webview_panel(PANEL_LABEL) else {
            return;
        };
        panel.hide();
    });
}

/// Hide with the frontend fade-out animation: notify the webview, wait for
/// the transition, then actually hide the panel.
fn hide_with_animation(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.emit("overlay:hide", ());
    }
    std::thread::sleep(Duration::from_millis(170));
    hide(app);
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_panel(PANEL_LABEL)
        .map(|panel| panel.is_visible())
        .unwrap_or(false)
}

pub fn toggle(app: &AppHandle) {
    if let Ok(mut last) = LAST_TOGGLE.lock() {
        if last.is_some_and(|t| t.elapsed() < TOGGLE_COOLDOWN) {
            return;
        }
        *last = Some(Instant::now());
    }
    if is_visible(app) {
        hide_with_animation(app);
    } else {
        show(app);
    }
}

/// Size the panel to the cursor's screen and place it at its bottom edge.
///
/// CGDisplayBounds is Quartz space (bottom-left origin), but set_position
/// treats the y axis as top-left (winit convention), so the bottom edge in
/// top-left terms is `screen_top - panel_height - gap`.
fn position_bottom(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };

    // Cursor location (Quartz global space: points, bottom-left origin).
    let cursor = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .ok()
        .and_then(|source| CGEvent::new(source).ok())
        .map(|event| event.location());

    // Display containing the cursor; fall back to the main display.
    let displays = CGDisplay::active_displays().unwrap_or_default();
    let display = cursor
        .and_then(|c| {
            displays
                .iter()
                .map(|id| CGDisplay::new(*id))
                .find(|d| d.bounds().contains(&c))
        })
        .or_else(|| displays.first().map(|id| CGDisplay::new(*id)));
    let Some(display) = display else {
        return;
    };

    let bounds = display.bounds();
    let screen_top = bounds.origin.y + bounds.size.height;
    let x = bounds.origin.x;
    let y = screen_top - PANEL_HEIGHT - PANEL_BOTTOM_GAP;

    let target = (bounds.size.width, PANEL_HEIGHT);
    let size_changed = CURRENT_SIZE
        .lock()
        .map(|c| *c != Some(target))
        .unwrap_or(true);
    if size_changed {
        if let Ok(mut c) = CURRENT_SIZE.lock() {
            *c = Some(target);
        }
        let _ = window.set_size(Size::Logical(LogicalSize::new(target.0, target.1)));
        // Wait for the async resize to finish before positioning/displaying.
        std::thread::sleep(Duration::from_millis(60));
    }
    let _ = window.set_position(Position::Logical(LogicalPosition::new(x, y)));
}
