use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeRTF, NSPasteboardTypeString};
use objc2_foundation::{NSData, NSString};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::db::{Clip, Db};

type DbState<'a> = State<'a, Mutex<Db>>;

fn map_db_err(e: rusqlite::Error) -> String {
    e.to_string()
}

/// Hide the overlay panel (used by the frontend, e.g. on Esc).
#[tauri::command]
pub fn hide_overlay(app: AppHandle) {
    crate::paste::log_info("[debug] hide_overlay invoked");
    crate::overlay::hide(&app);
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

#[tauri::command]
pub fn pinned_clips(state: DbState<'_>, limit: Option<i64>) -> Result<Vec<Clip>, String> {
    crate::paste::log_info("[debug] pinned_clips invoked");
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

/// Put the clip back on the clipboard (text / link / image), hide the panel,
/// then send ⌘V to the app that was frontmost before the overlay.
#[tauri::command]
pub fn paste_clip(app: AppHandle, state: DbState<'_>, id: i64) -> Result<(), String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    let clip = db
        .get_clip(id)
        .map_err(map_db_err)?
        .ok_or_else(|| "clip not found".to_string())?;
    drop(db);

    match clip.content_type.as_str() {
        "image" => write_image_clip(&app, &clip)?,
        "richtext" => write_rich_clip(&app, &clip)?,
        _ => write_text_clip(&app, &clip)?,
    }

    crate::overlay::hide(&app);

    // Wait for the previously active app to regain key window status, then
    // post ⌘V directly to the app that was frontmost before the overlay.
    let target = crate::overlay::target_pid();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        crate::paste::send_paste(target);
    });

    Ok(())
}

/// Put a text/link clip on the clipboard as plain text.
fn write_text_clip(app: &AppHandle, clip: &Clip) -> Result<(), String> {
    app.clipboard()
        .write_text(&clip.content)
        .map_err(|e| e.to_string())?;
    crate::clipboard::mark_last_seen(clip.content.as_bytes());
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
