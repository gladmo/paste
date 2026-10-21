use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// App settings persisted via tauri-plugin-store (single JSON object).
/// Semantics mirror the original Paste preferences (kPSTPreferences*).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Global hotkey that shows the history panel ("cmd+shift+v").
    pub activation_shortcut: String,
    /// Hotkey that toggles the Paste Stack panel.
    pub paste_stack_shortcut: String,
    /// Hotkeys for cycling pinboards.
    pub next_pinboard_shortcut: String,
    pub previous_pinboard_shortcut: String,
    /// Modifier held while pressing Enter/1-9 to paste as plain text.
    pub plain_text_modifier: String,
    /// Modifier held while pressing 1-9 for quick paste ("cmd").
    pub quick_paste_modifier: String,
    /// Show the menu bar (tray) icon.
    pub show_menu_bar_icon: bool,
    /// Play the copy/paste sound effects.
    pub sound_effects: bool,
    /// Paste directly to the active app (accessibility ⌘V) instead of only
    /// copying to the clipboard.
    pub direct_paste: bool,
    /// Always paste as plain text (original "Always paste as Plain Text").
    pub always_plain_text: bool,
    /// History retention: "forever" | "1month" | "1week" | "1day".
    pub history_capacity: String,
    /// Bundle ids whose copied content must be ignored.
    pub ignored_apps: Vec<String>,
    /// Do not save transient (auto-generated) pasteboard content.
    pub ignore_transient_content: bool,
    /// Do not save passwords / sensitive content when detected.
    pub ignore_confidential_content: bool,
    /// Fetch page titles for link previews.
    pub link_previews: bool,
    /// Launch at login (kept in sync with the autostart plugin).
    pub open_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            activation_shortcut: "cmd+shift+v".into(),
            paste_stack_shortcut: "ctrl+cmd+v".into(),
            next_pinboard_shortcut: "alt+cmd+arrow_right".into(),
            previous_pinboard_shortcut: "alt+cmd+arrow_left".into(),
            plain_text_modifier: "alt".into(),
            quick_paste_modifier: "cmd".into(),
            show_menu_bar_icon: true,
            sound_effects: true,
            direct_paste: true,
            always_plain_text: false,
            history_capacity: "forever".into(),
            ignored_apps: vec![],
            ignore_transient_content: true,
            ignore_confidential_content: true,
            link_previews: true,
            open_at_login: false,
        }
    }
}

const STORE_KEY: &str = "settings";

pub fn load(app: &AppHandle) -> Settings {
    use tauri_plugin_store::StoreExt;
    let Ok(store) = app.store("settings.json") else {
        return Settings::default();
    };
    match store.get(STORE_KEY) {
        Some(v) => serde_json::from_value(v).unwrap_or_default(),
        None => Settings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set(STORE_KEY, serde_json::to_value(settings).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;
    let _ = app.emit("settings:updated", settings);
    Ok(())
}

/// Create (or focus) the settings window. 641×590 pt matches the original.
pub fn open_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let builder = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html#/settings".into()),
    )
    .title("Paste")
    .inner_size(641.0, 590.0)
    .min_inner_size(560.0, 480.0)
    .resizable(true);
    if let Err(e) = builder.build() {
        eprintln!("settings window error: {e}");
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    load(&app)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    save(&app, &settings)?;
    // Keep the autostart plugin in sync with the toggle.
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let enabled = manager.is_enabled().unwrap_or(false);
    if settings.open_at_login != enabled {
        let _ = if settings.open_at_login {
            manager.enable()
        } else {
            manager.disable()
        };
    }
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    open_window(&app);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    pub bundle_id: String,
    pub name: String,
}

/// Running applications (for the "Ignore Applications" picker).
#[tauri::command]
pub fn running_apps(app: AppHandle) -> Vec<RunningApp> {
    use objc2_app_kit::NSWorkspace;
    let own = app.config().identifier.clone();
    let mut out: Vec<RunningApp> = Vec::new();
    if let Some(list) = NSWorkspace::sharedWorkspace().runningApplications() {
        for app in list.iter() {
            let Some(bundle) = app.bundleIdentifier().map(|s| s.to_string()) else {
                continue;
            };
            if bundle.is_empty() || Some(bundle.as_str()) == own.as_deref() {
                continue;
            }
            let name = app.localizedName().map(|s| s.to_string()).unwrap_or_else(|| bundle.clone());
            out.push(RunningApp { bundle_id: bundle, name });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.bundle_id == b.bundle_id);
    out
}

/// Open System Settings → Privacy & Security → Accessibility.
#[tauri::command]
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}
