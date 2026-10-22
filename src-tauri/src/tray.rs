use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::pause;
use crate::settings::Settings;

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "toggle", "Show Paste", true, None::<&str>)?;

    // Pause Paste: either a submenu of durations or a Resume item.
    let pause_section: Box<dyn tauri::menu::IsMenuItem<tauri::Wry>> = if pause::is_paused() {
        Box::new(MenuItem::with_id(app, "resume", "Resume Paste", true, None::<&str>)?)
    } else {
        let h1 = MenuItem::with_id(app, "pause-1h", "For 1 Hour", true, None::<&str>)?;
        let tomorrow = MenuItem::with_id(app, "pause-tomorrow", "Until Tomorrow", true, None::<&str>)?;
        let forever = MenuItem::with_id(app, "pause-forever", "Indefinitely", true, None::<&str>)?;
        Box::new(Submenu::with_items(
            app,
            "Pause Paste",
            true,
            &[&h1, &tomorrow, &forever],
        )?)
    };

    let erase = MenuItem::with_id(app, "erase", "Erase History…", true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", "Manage Pinboards…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Paste", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &*pause_section,
            &erase,
            &manage,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    // No accelerator here: the global hotkey (⌘⇧V) already handles toggling,
    // and a menu keyEquivalent would create a second Carbon registration.
    let menu = build_menu(app)?;

    let _tray = TrayIconBuilder::with_id("paste-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => crate::overlay::toggle(app),
            "resume" => {
                pause::resume();
                rebuild_menu(app);
            }
            "pause-1h" => {
                pause::pause_for_hours(1);
                rebuild_menu(app);
            }
            "pause-tomorrow" => {
                pause::pause_until_tomorrow();
                rebuild_menu(app);
            }
            "pause-forever" => {
                pause::pause_indefinitely();
                rebuild_menu(app);
            }
            "erase" => {
                // Ask the overlay to run its localized confirmation flow; it
                // is shown first so the dialog is visible.
                crate::overlay::show(app);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.emit("menu:erase-history", ());
                }
            }
            "manage" => {
                crate::overlay::show(app);
                if let Some(w) = app.get_webview_window("overlay") {
                    let _ = w.emit("panel:manage-pinboards", ());
                }
            }
            "settings" => crate::settings::open_window(app),
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

/// Rebuild the tray menu (pause state changes swap the pause section).
pub fn rebuild_menu(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("paste-tray") {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// Show/hide the menu bar icon per settings ("Show in Menu Bar").
pub fn apply_visibility(app: &AppHandle, settings: &Settings) {
    if let Some(tray) = app.tray_by_id("paste-tray") {
        let _ = tray.set_visible(settings.show_menu_bar_icon);
    }
    let _ = app;
}
