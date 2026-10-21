use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Tray menu aligned with the original's AppMenuProvider structure. The
/// pause submenu and erase-history confirmation land in P5.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    // No accelerator here: the global hotkey (⌘⇧V) already handles toggling,
    // and a menu keyEquivalent would create a second Carbon registration.
    let show = MenuItem::with_id(app, "toggle", "Show Paste", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let erase = MenuItem::with_id(app, "erase", "Erase History…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Paste", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &erase,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("paste-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => crate::overlay::toggle(app),
            "settings" => crate::settings::open_window(app),
            "erase" => {
                // Ask the overlay to run its localized confirmation flow; it
                // is shown first so the dialog is visible.
                crate::overlay::show(app);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.emit("menu:erase-history", ());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                crate::overlay::toggle(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
