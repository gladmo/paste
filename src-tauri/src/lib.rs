mod clipboard;
mod commands;
mod db;
mod overlay;
mod paste;
mod settings;
mod tray;

use std::str::FromStr;
use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Default toggle hotkey is ⌘⇧V (same as Paste). If it's taken by another app
/// (e.g. the original Paste), registration fails gracefully. `PASTE_HOTKEY`
/// env var overrides it (e.g. "cmd+shift+e") for testing.
fn global_shortcut() -> Shortcut {
    match std::env::var("PASTE_HOTKEY") {
        Ok(value) => Shortcut::from_str(&value).unwrap_or_else(|_| {
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV)
        }),
        Err(_) => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_nspanel::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar-only app like the original Paste (LSUIElement).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let db = db::Db::open(app.handle()).expect("open clipboard database");
            app.manage(Mutex::new(db));

            // Pasting into Chromium/Electron apps requires Accessibility
            // permission (global CGEventPost). Ask once at startup; the toggle
            // still works via post_to_pid fallback until it is granted.
            if !paste::ax_is_trusted() {
                paste::request_ax_permission();
            }

            #[cfg(target_os = "macos")]
            if let Err(e) = overlay::init(app.handle()) {
                eprintln!("overlay init error: {e}");
            }

            if let Err(e) = tray::create_tray(app.handle()) {
                eprintln!("tray init error: {e}");
            }

            // on_shortcut registers the shortcut AND attaches the handler; do
            // NOT call register() again, it fails with eventHotKeyExistsErr.
            if let Err(e) = app.global_shortcut().on_shortcut(global_shortcut(), |app, _s, e| {
                if e.state() == ShortcutState::Pressed {
                    overlay::toggle(app);
                }
            }) {
                eprintln!("warning: failed to register hotkey ({e}); use the tray icon instead");
            }

            clipboard::spawn_watcher(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_overlay,
            commands::debug_log,
            commands::recent_clips,
            commands::view_clips,
            commands::pinned_clips,
            commands::app_icon,
            commands::search_clips,
            commands::delete_clip,
            commands::delete_clips,
            commands::erase_history,
            commands::toggle_pin,
            commands::move_clip_to_pinboard,
            commands::paste_clip,
            commands::copy_clip,
            commands::list_pinboards,
            commands::create_pinboard,
            commands::rename_pinboard,
            commands::set_pinboard_color,
            commands::delete_pinboard,
            settings::get_settings,
            settings::set_settings,
            settings::open_settings,
            settings::running_apps,
            settings::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}