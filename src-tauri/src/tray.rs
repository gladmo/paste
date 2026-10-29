use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::pause;
use crate::settings::Settings;

// CFPreferences lives outside core-foundation 0.10's Rust wrappers; declare
// the one entry point we need (same pattern as paste.rs's CGEvent externs).
extern "C" {
    fn CFPreferencesCopyAppValue(
        key: core_foundation_sys::string::CFStringRef,
        app_id: core_foundation_sys::string::CFStringRef,
    ) -> core_foundation_sys::base::CFTypeRef;
}

/// The user's preferred UI language tag, e.g. "zh-Hans-CN" (AppleLanguages
/// from the global preferences domain — the same source `defaults read -g`
/// shows). None when the system refuses to answer.
fn system_language() -> Option<String> {
    unsafe {
        let key = CFString::new("AppleLanguages");
        // The global domain: kCFPreferencesAnyApplication is the literal
        // CFString "kCFPreferencesAnyApplication" (NOT NULL — passing null
        // segfaults inside CFPreferences).
        let any_app = CFString::new("kCFPreferencesAnyApplication");
        let value =
            CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), any_app.as_concrete_TypeRef());
        if value.is_null() {
            return None;
        }
        let langs = CFArray::<CFString>::wrap_under_create_rule(value as *const _);
        langs.get(0).map(|s| s.to_string())
    }
}

/// Resolve the effective locale, mirroring the frontend's detectLocale():
/// an explicit setting wins; "" follows the system (zh→Hant when the tag
/// says Hant/TW/HK/MO, else Hans; everything else falls back to English).
fn effective_locale(lang: &str) -> &'static str {
    match lang {
        "en" => "en",
        "zh-Hans" => "zh-Hans",
        "zh-Hant" => "zh-Hant",
        _ => {
            let tag = system_language().unwrap_or_default().to_lowercase();
            if tag.starts_with("zh") {
                if ["hant", "tw", "hk", "mo"].iter().any(|m| tag.contains(m)) {
                    "zh-Hant"
                } else {
                    "zh-Hans"
                }
            } else {
                "en"
            }
        }
    }
}

/// Tray label set for one locale. Canonical copies live in `src/i18n`
/// (original keys for the strings the original app ships; `replica.tray.*`
/// for the ones it does not) — keep this table in sync with them.
struct TrayLabels {
    show: &'static str,
    pause: &'static str,
    pause_1h: &'static str,
    pause_tomorrow: &'static str,
    pause_forever: &'static str,
    resume: &'static str,
    erase: &'static str,
    manage: &'static str,
    settings: &'static str,
    quit: &'static str,
}

fn tray_labels(locale: &str) -> TrayLabels {
    match locale {
        "zh-Hans" => TrayLabels {
            // replica.tray.* / general.action.pause-paste / general.action.erase-history /
            // general.action.manage-pinboards / general.settings / general.quit-paste
            show: "显示 Paste",
            pause: "暂停 Paste",
            pause_1h: "1 小时",
            pause_tomorrow: "直到明天",
            pause_forever: "无限期",
            resume: "恢复 Paste",
            erase: "删除历史…",
            manage: "管理粘贴板…",
            settings: "设置…",
            quit: "退出 Paste",
        },
        "zh-Hant" => TrayLabels {
            show: "顯示 Paste",
            pause: "暫停 Paste",
            pause_1h: "1 小時",
            pause_tomorrow: "直到明天",
            pause_forever: "無限期",
            resume: "恢復 Paste",
            erase: "删除历史记录…",
            manage: "管理剪貼夾…",
            settings: "設定…",
            quit: "退出 Paste",
        },
        _ => TrayLabels {
            show: "Show Paste",
            pause: "Pause Paste",
            pause_1h: "For 1 Hour",
            pause_tomorrow: "Until Tomorrow",
            pause_forever: "Indefinitely",
            resume: "Resume Paste",
            erase: "Erase History…",
            manage: "Manage Pinboards…",
            settings: "Settings…",
            quit: "Quit Paste",
        },
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    // Language changes must rebuild the tray (settings:updated listener);
    // "" follows the system language like the webview UI does.
    let lang = crate::settings::load(app).language;
    let l = tray_labels(effective_locale(&lang));
    let show = MenuItem::with_id(app, "toggle", l.show, true, None::<&str>)?;

    // Pause Paste: either a submenu of durations or a Resume item.
    let pause_section: Box<dyn tauri::menu::IsMenuItem<tauri::Wry>> = if pause::is_paused() {
        Box::new(MenuItem::with_id(app, "resume", l.resume, true, None::<&str>)?)
    } else {
        let h1 = MenuItem::with_id(app, "pause-1h", l.pause_1h, true, None::<&str>)?;
        let tomorrow = MenuItem::with_id(app, "pause-tomorrow", l.pause_tomorrow, true, None::<&str>)?;
        let forever = MenuItem::with_id(app, "pause-forever", l.pause_forever, true, None::<&str>)?;
        Box::new(Submenu::with_items(app, l.pause, true, &[&h1, &tomorrow, &forever])?)
    };

    let erase = MenuItem::with_id(app, "erase", l.erase, true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", l.manage, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", l.settings, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;

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

/// Rebuild the tray menu. Called for pause-state changes (the pause section
/// swaps) and from the settings:updated listener so a language change is
/// reflected immediately, like every window.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The CFPreferences extern is the risky bit — verify it actually
    /// answers on a real system (the test process needs no GUI).
    #[test]
    fn system_language_answers_and_locale_falls_back() {
        let lang = system_language();
        assert!(lang.is_some(), "AppleLanguages must be readable");
        assert!(!lang.unwrap().is_empty());
        for tag in ["", "fr", "zh-Hans-x", "zh_TW"] {
            assert!(["en", "zh-Hans", "zh-Hant"].contains(&effective_locale(tag)));
        }
        assert_eq!(effective_locale("zh-Hant"), "zh-Hant");
        assert_eq!(effective_locale("zh-Hans"), "zh-Hans");
        assert_eq!(effective_locale("en"), "en");
    }
}
