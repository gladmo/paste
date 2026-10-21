mod clipboard;
mod commands;
mod db;
mod overlay;
mod paste;
mod pause;
mod settings;
mod stack;
mod tray;

use std::sync::Mutex;

use tauri::{Emitter, Listener, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Default toggle hotkey is ⌘⇧V (same as Paste). If it's taken by another app
/// (e.g. the original Paste), registration fails gracefully. `PASTE_HOTKEY`
/// env var overrides it (e.g. "cmd+shift+e") for testing.
fn global_shortcut() -> Shortcut {
    match std::env::var("PASTE_HOTKEY") {
        Ok(value) => value.parse::<Shortcut>().unwrap_or_else(|_| {
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV)
        }),
        Err(_) => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV),
    }
}

fn parse_shortcut(s: &str) -> Option<Shortcut> {
    s.parse::<Shortcut>().ok()
}

/// (Re-)register every global shortcut from the current settings. Old
/// registrations are cleared first so changed combos never stay stuck.
fn register_all_shortcuts(app: &tauri::AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let s = settings::load(app);

    let activation = parse_shortcut(&s.activation_shortcut)
        .unwrap_or_else(global_shortcut);
    if let Err(e) = gs.on_shortcut(activation, |app, _s, e| {
        if e.state() == ShortcutState::Pressed {
            overlay::toggle(app);
        }
    }) {
        eprintln!("warning: failed to register activation hotkey ({e})");
    }

    let stack = parse_shortcut(&s.paste_stack_shortcut)
        .unwrap_or_else(|| Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyV));
    if let Err(e) = gs.on_shortcut(stack, |app, _s, e| {
        if e.state() == ShortcutState::Pressed {
            stack::toggle(app);
        }
    }) {
        eprintln!("warning: failed to register stack hotkey ({e})");
    }

    let next = parse_shortcut(&s.next_pinboard_shortcut);
    if let Some(sc) = next {
        let _ = gs.on_shortcut(sc, |app, _s, e| {
            if e.state() == ShortcutState::Pressed {
                overlay::show(app);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.emit("panel:cycle", 1i32);
                }
            }
        });
    }
    let prev = parse_shortcut(&s.previous_pinboard_shortcut);
    if let Some(sc) = prev {
        let _ = gs.on_shortcut(sc, |app, _s, e| {
            if e.state() == ShortcutState::Pressed {
                overlay::show(app);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.emit("panel:cycle", -1i32);
                }
            }
        });
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
            tray::apply_visibility(app.handle(), &settings::load(app.handle()));

            register_all_shortcuts(app.handle());

            // Re-apply settings whenever they change (shortcuts, tray icon,
            // retention cleanup).
            let handle = app.handle().clone();
            app.listen("settings:updated", move |_event| {
                register_all_shortcuts(&handle);
                let s = settings::load(&handle);
                tray::apply_visibility(&handle, &s);
                if let (Ok(mut db), Some(cutoff)) = (
                    handle.state::<Mutex<db::Db>>().lock(),
                    db::Db::capacity_cutoff(&s.history_capacity),
                ) {
                    let _ = db.enforce_capacity(Some(cutoff));
                }
                tray::rebuild_menu(&handle);
            });

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
            commands::get_clip,
            commands::update_clip_content,
            commands::save_clip_image,
            commands::create_text_clip,
            commands::open_preview,
            commands::open_create,
            commands::stack_toggle,
            commands::stack_hide,
            commands::stack_paste,
            pause::pause_status,
            commands::reveal_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
