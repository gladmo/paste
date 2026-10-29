use std::ffi::c_void;
use std::io::Write;
use std::time::{Duration, Instant};

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use foreign_types::ForeignType;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

// CoreGraphics C API: attach the actual character ("v") to a keyboard event.
// Events built with CGEventCreateKeyboardEvent have an *empty* unicode
// string; terminals map keycodes, but every normal text input (browser,
// chat apps, editors) drops key events without a character.
#[allow(improper_ctypes)]
extern "C" {
    fn CGEventKeyboardSetUnicodeString(
        event: *const c_void,
        string_length: isize,
        unicode_string: *const u16,
    );
}

// Accessibility permission (TCC): posting to the global event stream
// (CGEventPost / HID / Session taps) silently fails without it. post_to_pid
// works without permission but Chromium/Electron renderer processes ignore
// pid-direct keyboard events, so only global posting reaches them.
// Permission is granted from the Settings window; the app never auto-prompts
// (unsigned dev builds change their TCC identity on every rebuild, so a
// launch-time prompt would reappear endlessly).
extern "C" {
    fn AXIsProcessTrusted() -> libc::c_uchar;
}

/// Persistent log at /tmp/paste.log — /tmp/paste-run.log has been empty
/// (nohup buffering), so write paste diagnostics to a file explicitly.
/// Every line is prefixed with the elapsed time since app start: paste/hide
/// ordering bugs are pure timing questions, and untimestamped lines cannot
/// sequence them after the fact.
pub fn log_info(msg: &str) {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/paste.log")
    {
        let _ = writeln!(f, "[+{:>9.3}s] {msg}", start.elapsed().as_secs_f64());
    }
}

/// True when the app already has Accessibility permission.
pub fn ax_is_trusted() -> bool {
    let trusted = unsafe { AXIsProcessTrusted() != 0 };
    log_info(&format!("[ax] trusted={trusted}"));
    trusted
}

/// Bring the target app back to the front so its key window exists again.
/// While our panel is up the previous app has no key window, and the window
/// server drops keyboard events posted to multi-window apps without one.
/// Activation is asynchronous, so poll until the app reports itself active
/// (some apps take a few hundred ms to regain frontmost status).
fn activate_target(pid: i32) {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        log_info(&format!("[paste] target pid {pid} not running"));
        return;
    };
    #[allow(deprecated)]
    let _ = app.activateWithOptions(
        NSApplicationActivationOptions::ActivateAllWindows
            | NSApplicationActivationOptions::ActivateIgnoringOtherApps,
    );
    let deadline = Instant::now() + Duration::from_millis(500);
    while !app.isActive() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    log_info(&format!("[paste] activate {pid} -> active={}", app.isActive()));
}

/// Send ⌘V to the app the user was working in before the overlay appeared.
///
/// The panel is a non-activating panel, so that app never lost frontmost
/// status, its key window or its first responder (caret keeps blinking). All
/// we have to do is hide the panel, wait a moment for the key window to
/// settle back, then inject ⌘V.
///
/// Posting strategy:
/// - With Accessibility permission: post to the global HID tap, like a real
///   keypress. This is the only path that reaches Chromium/Electron renderer
///   processes (Feishu, browser web content) and works in every app.
/// - Without permission (CGEventPost silently fails): fall back to
///   post_to_pid, which needs no permission and covers keycode-driven apps
///   (terminals, native text fields).
pub fn send_paste(target_pid: Option<i32>) {
    let ax_trusted = ax_is_trusted();

    if let Some(pid) = target_pid {
        // The panel took the key window while it was up, so the target app
        // must be reactivated: once it is frontmost again its own window
        // becomes the key window and the global ⌘V below lands there (the
        // only path Chromium/Electron renderers accept).
        activate_target(pid);
        std::thread::sleep(Duration::from_millis(150));
    } else {
        std::thread::sleep(Duration::from_millis(100));
    }

    let Some(down) = build_key(KeyCode::ANSI_V, true) else {
        log_info("[paste] cannot build keydown");
        return;
    };
    let Some(up) = build_key(KeyCode::ANSI_V, false) else {
        log_info("[paste] cannot build keyup");
        return;
    };

    if ax_trusted {
        log_info(&format!(
            "[paste] ⌘V -> global HID tap (target={target_pid:?})"
        ));
        down.post(CGEventTapLocation::HID);
        // Electron and some other apps expect a small gap between down/up.
        std::thread::sleep(Duration::from_millis(15));
        up.post(CGEventTapLocation::HID);
    } else {
        match target_pid {
            Some(pid) => {
                log_info(&format!("[paste] ⌘V -> pid {pid} (no AX permission)"));
                down.post_to_pid(pid);
                up.post_to_pid(pid);
            }
            None => {
                log_info("[paste] ⌘V -> session");
                down.post(CGEventTapLocation::Session);
                up.post(CGEventTapLocation::Session);
            }
        }
    }
}

fn build_key(key: u16, down: bool) -> Option<CGEvent> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new_keyboard_event(source, key, down).ok()?;
    event.set_flags(CGEventFlags::CGEventFlagCommand);
    set_unicode(&event, "v");
    Some(event)
}

fn set_unicode(event: &CGEvent, chars: &str) {
    let unicode: Vec<u16> = chars.encode_utf16().collect();
    unsafe {
        CGEventKeyboardSetUnicodeString(
            event.as_ptr() as *const c_void,
            unicode.len() as isize,
            unicode.as_ptr(),
        );
    }
}