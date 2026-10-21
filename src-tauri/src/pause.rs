use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

/// Pause deadline in unix ms; None = not paused. i64::MAX = paused
/// indefinitely (until resumed or the app quits).
static PAUSED_UNTIL: Mutex<Option<i64>> = Mutex::new(None);

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as i64
}

pub fn is_paused() -> bool {
    PAUSED_UNTIL
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|deadline| deadline > now_ms() || deadline == i64::MAX)
}

pub fn deadline() -> Option<i64> {
    PAUSED_UNTIL.lock().ok().and_then(|g| *g)
}

pub fn pause_until(ms: i64) {
    if let Ok(mut g) = PAUSED_UNTIL.lock() {
        *g = Some(ms);
    }
}

pub fn pause_for_hours(hours: i64) {
    pause_until(now_ms() + hours * 3_600_000);
}

/// Pause until the start of tomorrow (local midnight ≈ next day 00:00).
pub fn pause_until_tomorrow() {
    // libSystem time zones are heavy; 24h is a fine approximation and
    // matches the visible countdown closely enough.
    pause_for_hours(24);
}

pub fn pause_indefinitely() {
    pause_until(i64::MAX);
}

pub fn resume() {
    if let Ok(mut g) = PAUSED_UNTIL.lock() {
        *g = None;
    }
}

/// Remaining ms if paused (None when running).
pub fn remaining_ms() -> Option<i64> {
    deadline().map(|d| {
        if d == i64::MAX {
            i64::MAX
        } else {
            (d - now_ms()).max(0)
        }
    })
}

/// Play the original sound effects. `afplay` is a macOS built-in so no
/// extra decoder dependency is needed; resource files are bundled via
/// tauri.conf `bundle.resources`.
fn play(app: &AppHandle, file: &str) {
    use tauri::Manager;
    let settings = crate::settings::load(app);
    if !settings.sound_effects {
        return;
    }
    let Ok(path) = app.path().resolve(file, tauri::path::BaseDirectory::Resource) else {
        return;
    };
    let _ = std::process::Command::new("afplay")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn play_copy(app: &AppHandle) {
    play(app, "sounds/Copy.aiff");
}

pub fn play_paste(app: &AppHandle) {
    play(app, "sounds/Paste.aiff");
}

#[tauri::command]
pub fn pause_status() -> Option<i64> {
    remaining_ms()
}
