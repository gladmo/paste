use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeRTF, NSPasteboardTypeString};
use objc2_foundation::{NSData, NSString};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::db::{Clip, Db, Pinboard};

type DbState<'a> = State<'a, Mutex<Db>>;

fn map_db_err(e: rusqlite::Error) -> String {
    e.to_string()
}

/// Hide the overlay panel with the fade-out animation (used by the frontend,
/// e.g. on Esc — Rust owns the timing so the close never depends on webview
/// timers, which a hidden WKWebView suspends).
#[tauri::command]
pub fn hide_overlay(app: AppHandle) {
    let _ = crate::overlay::hide_with_animation(&app);
}

/// The overlay UI finished resetting and loading data; reveal the panel.
#[tauri::command]
pub fn overlay_ready(app: AppHandle) {
    crate::overlay::ready(&app);
}

/// Temp debug bridge: frontend logs what it renders.
#[tauri::command]
pub fn debug_log(line: String) {
    crate::paste::log_info(&format!("[render] {line}"));
}

#[tauri::command]
pub fn recent_clips(state: DbState<'_>, limit: Option<i64>) -> Result<Vec<Clip>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.recent(limit.unwrap_or(100)).map_err(map_db_err)
}

/// Clips for one panel view: "history" | "pinned" | "board".
#[tauri::command]
pub fn view_clips(
    state: DbState<'_>,
    view: String,
    pinboard_id: Option<i64>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Clip>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.query_view(&view, pinboard_id, query.as_deref(), limit.unwrap_or(200))
        .map_err(map_db_err)
}

#[tauri::command]
pub fn pinned_clips(state: DbState<'_>, limit: Option<i64>) -> Result<Vec<Clip>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.pinned_clips(limit.unwrap_or(100)).map_err(map_db_err)
}

/// Absolute path of the cached icon PNG for an app; None when unknown.
#[tauri::command]
pub fn app_icon(app: AppHandle, bundle_id: String) -> Option<String> {
    crate::clipboard::app_icon_path(&app, &bundle_id)
}

#[tauri::command]
pub fn search_clips(state: DbState<'_>, query: String) -> Result<Vec<Clip>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.search(&query, 100).map_err(map_db_err)
}

// ---- pinboards ----

#[tauri::command]
pub fn list_pinboards(state: DbState<'_>) -> Result<Vec<Pinboard>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.list_pinboards().map_err(map_db_err)
}

#[tauri::command]
pub fn create_pinboard(
    app: AppHandle,
    state: DbState<'_>,
    name: String,
    color: Option<i64>,
) -> Result<Pinboard, String> {
    let board = {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.create_pinboard(&name, color.unwrap_or(0xb663e0))
            .map_err(map_db_err)?
    };
    let _ = app.emit("pinboards:updated", ());
    Ok(board)
}

#[tauri::command]
pub fn rename_pinboard(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    name: String,
) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.rename_pinboard(id, &name).map_err(map_db_err)?;
    }
    let _ = app.emit("pinboards:updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_pinboard_color(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    color: i64,
) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.set_pinboard_color(id, color).map_err(map_db_err)?;
    }
    let _ = app.emit("pinboards:updated", ());
    Ok(())
}

#[tauri::command]
pub fn delete_pinboard(app: AppHandle, state: DbState<'_>, id: i64) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.delete_pinboard(id)?;
    }
    let _ = app.emit("pinboards:updated", ());
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Move a clip into a pinboard (pinboard_id None returns it to history).
#[tauri::command]
pub fn move_clip_to_pinboard(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    pinboard_id: Option<i64>,
) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.move_clip_to_pinboard(id, pinboard_id)
            .map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

#[tauri::command]
pub fn toggle_pin(app: AppHandle, state: DbState<'_>, id: i64) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.toggle_pin(id).map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

#[tauri::command]
pub fn delete_clip(app: AppHandle, state: DbState<'_>, id: i64) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.delete(id).map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Batch delete (multi-select).
#[tauri::command]
pub fn delete_clips(app: AppHandle, state: DbState<'_>, ids: Vec<i64>) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.delete_many(&ids).map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Erase the clipboard history; pinned items and pinboards are kept.
#[tauri::command]
pub fn erase_history(app: AppHandle, state: DbState<'_>) -> Result<(), String> {
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.erase_history().map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Copy a clip to the clipboard without pasting (context menu "Copy").
#[tauri::command]
pub fn copy_clip(app: AppHandle, state: DbState<'_>, id: i64, plain: Option<bool>) -> Result<(), String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    let clip = db
        .get_clip(id)
        .map_err(map_db_err)?
        .ok_or_else(|| "clip not found".to_string())?;
    drop(db);
    write_clip(&app, &clip, plain.unwrap_or(false))?;
    Ok(())
}

/// Fetch a single clip (preview / create windows).
#[tauri::command]
pub fn get_clip(state: DbState<'_>, id: i64) -> Result<Option<Clip>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.get_clip(id).map_err(map_db_err)
}

/// Save edited text/rich content. For rich clips the RTF representation is
/// regenerated from the edited HTML via textutil so format-preserving paste
/// keeps working.
#[tauri::command]
pub fn update_clip_content(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    content: String,
    plain_content: String,
) -> Result<(), String> {
    let (content_type, old_rtf) = {
        let db = state.lock().map_err(|e| e.to_string())?;
        let clip = db
            .get_clip(id)
            .map_err(map_db_err)?
            .ok_or_else(|| "clip not found".to_string())?;
        (clip.content_type, clip.rtf)
    };
    let rtf_b64: Option<String> = if content_type == "richtext" {
        let html = content.clone();
        let rtf = crate::clipboard::textutil_convert(html.as_bytes(), "html", "rtf")
            .ok_or("textutil html->rtf failed")?;
        Some(BASE64.encode(&rtf))
    } else {
        // Plain edit invalidates any stale rich representation.
        let _ = old_rtf;
        None
    };
    {
        let db = state.lock().map_err(|e| e.to_string())?;
        db.update_clip(id, &content, &plain_content, rtf_b64.as_deref())
            .map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Replace an image clip's stored PNG (base64) — used by rotation.
#[tauri::command]
pub fn save_clip_image(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    png_base64: String,
) -> Result<(), String> {
    let bytes = BASE64
        .decode(&png_base64)
        .map_err(|e| format!("png decode: {e}"))?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!(
        "{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_millis()
    );
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    {
        let db = state.lock().map_err(|e| e.to_string())?;
        db.set_clip_content(id, &path.to_string_lossy())
            .map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Create a text clip from the Create Item window.
#[tauri::command]
pub fn create_text_clip(app: AppHandle, state: DbState<'_>, text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    {
        let mut db = state.lock().map_err(|e| e.to_string())?;
        db.insert_text_clip(&text).map_err(map_db_err)?;
    }
    let _ = app.emit("clips:updated", ());
    Ok(())
}

/// Open the preview window for a clip (460×380).
#[tauri::command]
pub fn open_preview(app: AppHandle, id: i64) {
    if let Some(w) = app.get_webview_window("preview") {
        let _ = w.eval(&format!("window.location.hash = '#/preview?id={id}'"));
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        "preview",
        tauri::WebviewUrl::App(format!("index.html#/preview?id={id}").into()),
    )
    .title("Paste")
    .inner_size(460.0, 400.0)
    .min_inner_size(380.0, 320.0)
    .resizable(true);
    if let Err(e) = builder.build() {
        eprintln!("preview window error: {e}");
    }
}

/// Open the Create Item window.
#[tauri::command]
pub fn open_create(app: AppHandle) {
    if let Some(w) = app.get_webview_window("create") {
        let _ = w.eval("window.location.hash = '#/create'");
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        "create",
        tauri::WebviewUrl::App("index.html#/create".into()),
    )
    .title("Paste")
    .inner_size(400.0, 260.0)
    .resizable(true)
    .min_inner_size(360.0, 220.0);
    if let Err(e) = builder.build() {
        eprintln!("create window error: {e}");
    }
}

/// Reveal a file/folder in Finder (context menu action for file clips).
#[tauri::command]
pub fn reveal_in_finder(path: String) {
    let _ = std::process::Command::new("open").args(["-R", &path]).spawn();
}

// ---- Paste Stack ----

#[tauri::command]
pub fn stack_toggle(app: AppHandle) {
    crate::stack::toggle(&app);
}

#[tauri::command]
pub fn stack_hide(app: AppHandle) {
    crate::stack::hide(&app);
}

/// Paste an item from the Paste Stack into the frontmost app. The stack is
/// a non-activating panel, so the previous app still has key focus and the
/// global ⌘V lands directly in it.
#[tauri::command]
pub fn stack_paste(app: AppHandle, state: DbState<'_>, id: i64) -> Result<(), String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    let clip = db
        .get_clip(id)
        .map_err(map_db_err)?
        .ok_or_else(|| "clip not found".to_string())?;
    drop(db);
    let settings = crate::settings::load(&app);
    write_clip(&app, &clip, settings.always_plain_text)?;
    crate::pause::play_paste(&app);
    crate::stack::hide(&app);
    if settings.direct_paste {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(120));
            crate::paste::send_paste(None);
        });
    }
    Ok(())
}

/// Put the clip back on the clipboard (text / link / image), hide the panel,
/// then send ⌘V to the app that was frontmost before the overlay.
/// `plain` pastes the plain-text representation only.
#[tauri::command]
pub fn paste_clip(
    app: AppHandle,
    state: DbState<'_>,
    id: i64,
    plain: Option<bool>,
) -> Result<(), String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    let clip = db
        .get_clip(id)
        .map_err(map_db_err)?
        .ok_or_else(|| "clip not found".to_string())?;
    drop(db);

    let settings = crate::settings::load(&app);
    let plain = plain.unwrap_or(false) || settings.always_plain_text;
    write_clip(&app, &clip, plain)?;
    crate::pause::play_paste(&app);

    // Animated hide (webview fades via the overlay:hide event); the receiver
    // resolves once panel.hide() really executed on the main thread.
    let target = crate::overlay::target_pid();
    let hidden = crate::overlay::hide_with_animation(&app);

    if !settings.direct_paste {
        // "To clipboard" paste mode: only copy, let the user paste manually.
        return Ok(());
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        // Order the ⌘V against the ACTUAL panel hide, not a fixed sleep:
        // while the panel webview is still the key window the synthetic
        // keystroke lands in the fading webview instead of the target app.
        if hidden
            .recv_timeout(std::time::Duration::from_millis(900))
            .is_err()
        {
            crate::paste::log_info("[paste] panel hide not confirmed within 900ms");
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
        crate::overlay::log_focus_state(&handle);
        crate::paste::send_paste(target);
    });

    Ok(())
}

/// Write a clip onto the system pasteboard according to its type.
fn write_clip(app: &AppHandle, clip: &Clip, plain: bool) -> Result<(), String> {
    match clip.content_type.as_str() {
        "image" => write_image_clip(app, clip),
        "color" => write_color_clip(clip),
        "file" => write_file_clip(clip),
        "richtext" if !plain => write_rich_clip(app, clip),
        _ => {
            let text = if clip.content_type == "richtext" {
                clip.plain_content.clone().unwrap_or_default()
            } else {
                clip.content.clone()
            };
            write_text(app, clip, &text)
        }
    }
}

/// Put an archived NSColor back on the pasteboard (raw bytes kept in `rtf`).
fn write_color_clip(clip: &Clip) -> Result<(), String> {
    let bytes = clip
        .rtf
        .as_deref()
        .map(|b64| BASE64.decode(b64))
        .transpose()
        .map_err(|e| format!("color decode: {e}"))?
        .unwrap_or_default();
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    let data = NSData::with_bytes(&bytes);
    let ok = pb.setData_forType(Some(&data), unsafe { objc2_app_kit::NSPasteboardTypeColor });
    if !ok {
        return Err("pasteboard color write failed".to_string());
    }
    crate::clipboard::mark_last_seen(&bytes);
    Ok(())
}

/// Put file references back on the pasteboard: a public.file-url entry plus
/// the plain-text path list for apps that only read text.
fn write_file_clip(clip: &Clip) -> Result<(), String> {
    let paths: Vec<&str> = clip
        .plain_content
        .as_deref()
        .unwrap_or(&clip.content)
        .split('\n')
        .filter(|l| !l.trim().is_empty())
        .collect();
    if paths.is_empty() {
        return write_text_no_clip(&clip.content);
    }
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    let urls = paths
        .iter()
        .map(|p| format!("file://{p}"))
        .collect::<Vec<_>>()
        .join("\n");
    let data = NSData::with_bytes(urls.as_bytes());
    let _ = pb.setData_forType(Some(&data), &NSString::from_str("public.file-url"));
    let plain = NSString::from_str(&paths.join("\n"));
    let _ = pb.setString_forType(&plain, unsafe { NSPasteboardTypeString });
    crate::clipboard::mark_last_seen(urls.as_bytes());
    Ok(())
}

fn write_text_no_clip(text: &str) -> Result<(), String> {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    let s = NSString::from_str(text);
    let _ = pb.setString_forType(&s, unsafe { NSPasteboardTypeString });
    Ok(())
}

/// Put plain text on the clipboard (text/link clips, or rich clips pasted
/// as plain text).
fn write_text(app: &AppHandle, clip: &Clip, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    crate::clipboard::mark_last_seen(text.as_bytes());
    // Keep dedup consistent for rich clips pasted as plain text.
    if clip.content_type == "richtext" {
        if let Some(rtf) = clip.rtf.as_deref() {
            if let Ok(bytes) = BASE64.decode(rtf) {
                crate::clipboard::mark_last_seen(&bytes);
            }
        }
    }
    Ok(())
}

/// Put a rich text clip back on the clipboard: RTF (original formatting)
/// plus plain text for apps that only read text. ⌘V then pastes formatted
/// code into editors like GoLand / Xcode, matching the source colors.
fn write_rich_clip(_app: &AppHandle, clip: &Clip) -> Result<(), String> {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();

    if let Some(rtf_b64) = clip.rtf.as_deref() {
        let bytes = BASE64
            .decode(rtf_b64)
            .map_err(|e| format!("rtf decode: {e}"))?;
        let data = NSData::with_bytes(&bytes);
        let ok = pb.setData_forType(Some(&data), unsafe { NSPasteboardTypeRTF });
        if !ok {
            return Err("pasteboard rtf write failed".to_string());
        }
        crate::clipboard::mark_last_seen(&bytes);
    }

    if let Some(plain) = clip.plain_content.as_deref() {
        if !plain.is_empty() {
            let s = NSString::from_str(plain);
            let _ = pb.setString_forType(&s, unsafe { NSPasteboardTypeString });
        }
    }
    Ok(())
}

/// Put an image clip (stored as a PNG file) back on the clipboard as PNG
/// data, which is what pasting apps expect.
fn write_image_clip(_app: &AppHandle, clip: &Clip) -> Result<(), String> {
    let bytes = std::fs::read(&clip.content).map_err(|e| e.to_string())?;
    let data = NSData::with_bytes(&bytes);
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    let ok = pb.setData_forType(Some(&data), unsafe { NSPasteboardTypePNG });
    if !ok {
        return Err("pasteboard write failed".to_string());
    }
    crate::clipboard::mark_last_seen(&bytes);
    Ok(())
}
