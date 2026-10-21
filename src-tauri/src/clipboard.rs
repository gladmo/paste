use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSBitmapImageRepPropertyKey, NSPasteboard,
    NSPasteboardTypeHTML, NSPasteboardTypeRTF, NSPasteboardTypeTIFF, NSWorkspace,
};
use objc2_foundation::{NSData, NSDictionary, NSString};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Hash of the raw clipboard content we last captured, so the watcher skips
/// unchanged content instead of re-inserting it (images have no stable
/// textual representation to dedupe against in the DB).
static LAST_SEEN: Mutex<Option<u64>> = Mutex::new(None);

/// Tell the watcher to ignore the next poll: used right after we write our
/// own paste-back into the clipboard so it is not stored again.
pub fn mark_last_seen(bytes: &[u8]) {
    if let Ok(mut seen) = LAST_SEEN.lock() {
        *seen = Some(hash(bytes));
    }
}

fn hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Snapshot the frontmost app — the most reliable proxy for "which app just
/// copied" (macOS exposes no public pasteboard-owner API). Best effort; the
/// paste tool's panel is non-activating so it never steals frontmost.
fn snapshot_frontmost(app: &AppHandle) -> (Option<String>, Option<String>) {
    let Some(app_) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return (None, None);
    };
    let bundle_id = app_.bundleIdentifier().map(|s| s.to_string());
    let name = app_.localizedName().map(|s| s.to_string());
    let Some(bundle_id) = bundle_id else {
        return (None, name);
    };
    // Never attribute clips to ourselves.
    if bundle_id == app.config().identifier {
        return (None, None);
    }
    (Some(bundle_id), name)
}

/// Encode an NSImage (e.g. an app icon) as PNG and cache it under
/// <app_data_dir>/icons/<bundle_id>.png. Returns the path on success.
fn save_icon_png(app: &AppHandle, bundle_id: &str, icon: &objc2_app_kit::NSImage) -> Option<String> {
    let dir = app
        .path()
        .app_data_dir()
        .ok()?
        .join("icons");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{bundle_id}.png"));
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    let tiff = icon.TIFFRepresentation()?;
    let rep = NSBitmapImageRep::imageRepWithData(&tiff)?;
    let png = unsafe {
        rep.representationUsingType_properties(
            NSBitmapImageFileType::PNG,
            &NSDictionary::<NSBitmapImageRepPropertyKey, objc2::runtime::AnyObject>::new(),
        )
    }?;
    std::fs::write(&path, png.iter().collect::<Vec<u8>>()).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// Cache the icon of the app with `bundle_id` (lookup via LaunchServices).
fn cache_app_icon(app: &AppHandle, bundle_id: &str) {
    let ws = NSWorkspace::sharedWorkspace();
    let ns_bundle_id = NSString::from_str(bundle_id);
    let Some(url) = ws.URLForApplicationWithBundleIdentifier(&ns_bundle_id) else {
        return;
    };
    let Some(path_str) = url.path().map(|p| p.to_string()) else {
        return;
    };
    let icon = ws.iconForFile(&NSString::from_str(&path_str));
    let _ = save_icon_png(app, bundle_id, &icon);
}

/// Absolute path of the cached icon PNG for `bundle_id` (looks it up and
/// caches it on first use). The frontend wraps this with convertFileSrc.
pub fn app_icon_path(app: &AppHandle, bundle_id: &str) -> Option<String> {
    let path = app
        .path()
        .app_data_dir()
        .ok()?
        .join("icons")
        .join(format!("{bundle_id}.png"));
    if !path.exists() {
        cache_app_icon(app, bundle_id);
    }
    if !path.exists() {
        return None;
    }
    Some(path.to_string_lossy().into_owned())
}

fn same_as_last(bytes: &[u8]) -> bool {
    LAST_SEEN.lock().ok().map(|s| *s == Some(hash(bytes))).unwrap_or(false)
}

fn set_last_seen(bytes: &[u8]) {
    if let Ok(mut seen) = LAST_SEEN.lock() {
        *seen = Some(hash(bytes));
    }
}

fn detect_link(text: &str) -> &'static str {
    let t = text.trim();
    if t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("www.")
        || t.contains("://")
    {
        "link"
    } else {
        "text"
    }
}

/// Fetch the <title> of a URL so link cards can show it. Best effort; a slow
/// or failing request just yields no title.
fn fetch_link_title(url: &str) -> Option<String> {
    let full = if url.starts_with("www.") {
        format!("https://{url}")
    } else {
        url.to_string()
    };
    let resp = ureq::get(&full)
        .timeout(Duration::from_secs(4))
        .call()
        .ok()?;
    let mut buf = Vec::new();
    resp.into_reader().take(131_072).read_to_end(&mut buf).ok()?;
    let html = String::from_utf8_lossy(&buf);
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let gt = lower[start..].find('>')? + start + 1;
    let end = lower[gt..].find("</title")? + gt;
    let title = html[gt..end].trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// Fetch the page title for a freshly stored link clip and update the DB.
fn fetch_title_async(app: AppHandle, id: i64, url: String) {
    std::thread::spawn(move || {
        if let Some(title) = fetch_link_title(&url) {
            let ok = app
                .state::<Mutex<crate::db::Db>>()
                .lock()
                .ok()
                .and_then(|db| db.set_title(id, Some(&title)).ok())
                .is_some();
            if ok {
                let _ = app.emit("clips:updated", ());
            }
        }
    });
}

/// Read the image on the general pasteboard as TIFF bytes (the type macOS
/// exposes for copied images). Must run on the main thread.
fn read_tiff() -> Option<Vec<u8>> {
    let pb = NSPasteboard::generalPasteboard();
    let data = pb.dataForType(unsafe { NSPasteboardTypeTIFF })?;
    Some(data.iter().collect())
}

/// Read raw bytes of a pasteboard type (e.g. public.rtf / public.html).
fn read_pb_data(pasteboard_type: &NSString) -> Option<Vec<u8>> {
    let pb = NSPasteboard::generalPasteboard();
    let data = pb.dataForType(pasteboard_type)?;
    Some(data.iter().collect())
}

/// Read the rich text representation of the clipboard: RTF bytes, or the
/// HTML representation when no RTF is present. Must run on the main thread.
fn read_rich() -> Option<(Vec<u8>, bool)> {
    if let Some(rtf) = read_pb_data(unsafe { NSPasteboardTypeRTF }) {
        return Some((rtf, true));
    }
    read_pb_data(unsafe { NSPasteboardTypeHTML }).map(|html| (html, false))
}

/// Convert clipboard bytes between rich formats with /usr/bin/textutil
/// (macOS-native; spawn cost is negligible vs. the 500 ms poll interval).
pub(crate) fn textutil_convert(input: &[u8], from: &str, to: &str) -> Option<Vec<u8>> {
    let mut child = std::process::Command::new("/usr/bin/textutil")
        .args(["-convert", to, "-stdin", "-stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(input).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let _ = from; // input type is inferred from the bytes
    Some(output.stdout)
}

/// A rich text capture: HTML for display, plain text for search / fallback
/// paste, and the original RTF bytes (base64) for format-preserving paste.
struct RichClip {
    html: String,
    plain: String,
    rtf_b64: String,
}

/// Build the rich clip from the raw clipboard bytes (RTF, or HTML when the
/// source only provided HTML). The same bytes are used for dedup hashing.
fn rich_from_bytes(app: &AppHandle, bytes: Vec<u8>, is_rtf: bool) -> Option<RichClip> {
    let html = if is_rtf {
        String::from_utf8_lossy(&textutil_convert(&bytes, "rtf", "html")?).into_owned()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    if html.trim().is_empty() {
        return None;
    }
    let rtf = if is_rtf {
        bytes
    } else {
        textutil_convert(html.as_bytes(), "html", "rtf")?
    };
    // Prefer the pasteboard's own plain-text representation; fall back to
    // extracting text from the RTF (some sources only write RTF).
    let plain = match app.clipboard().read_text() {
        Ok(t) if !t.trim().is_empty() => t,
        _ => textutil_convert(&rtf, "rtf", "txt")
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default(),
    };
    Some(RichClip {
        html,
        plain,
        rtf_b64: BASE64.encode(rtf),
    })
}

/// Convert TIFF data to PNG bytes and write them to the app data dir.
/// Returns the absolute path of the stored file.
fn save_tiff_as_png(tiff: &[u8], app: &AppHandle) -> Result<String, String> {
    let data = NSData::with_bytes(tiff);
    let rep = NSBitmapImageRep::imageRepWithData(&data)
        .ok_or_else(|| "cannot decode image".to_string())?;
    let png = unsafe {
        rep.representationUsingType_properties(
            NSBitmapImageFileType::PNG,
            &NSDictionary::<NSBitmapImageRepPropertyKey, AnyObject>::new(),
        )
    }
    .ok_or_else(|| "cannot encode png".to_string())?;

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
    let path = dir.join(name);
    std::fs::write(&path, png.iter().collect::<Vec<u8>>()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Poll the system clipboard and store new clips (text / link / image). Runs
/// on a background thread; emits `clips:updated` whenever a clip is stored.
    pub fn spawn_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let clipboard = app.clipboard();
        loop {
            std::thread::sleep(Duration::from_millis(500));

            // Frontmost app at this instant = the app that wrote the
            // clipboard (the paste panel itself never becomes frontmost).
            let (src_app, src_name) = snapshot_frontmost(&app);

            // Rich text has the highest priority: without this, the plain
            // text representation of a styled copy would swallow it as a
            // boring text/link clip. Dedup hashes the raw RTF/HTML bytes.
            if let Some((bytes, is_rtf)) = read_rich() {
                if same_as_last(&bytes) {
                    continue;
                }
                set_last_seen(&bytes);
                let Some(rich) = rich_from_bytes(&app, bytes, is_rtf) else {
                    continue;
                };
                let inserted = app
                    .state::<Mutex<crate::db::Db>>()
                    .lock()
                    .map(|mut db| {
                        db.insert(
                            "richtext",
                            &rich.html,
                            &rich.plain,
                            Some(&rich.rtf_b64),
                            None,
                            src_app.as_deref(),
                            src_name.as_deref(),
                        )
                    })
                    .ok()
                    .and_then(|r| r.ok())
                    .flatten();
                if inserted.is_some() {
                    if let Some(bundle) = &src_app {
                        cache_app_icon(&app, bundle);
                    }
                    let _ = app.emit("clips:updated", ());
                }
                continue;
            }

            let mut captured: Option<(&'static str, String)> = None;

            if let Ok(text) = clipboard.read_text() {
                if !text.trim().is_empty() {
                    captured = Some((detect_link(&text), text));
                }
            }

            if captured.is_none() {
                if let Some(tiff) = read_tiff() {
                    if !same_as_last(&tiff) {
                        match save_tiff_as_png(&tiff, &app) {
                            Ok(path) => {
                                set_last_seen(&tiff);
                                captured = Some(("image", path));
                            }
                            Err(e) => eprintln!("[clipboard] image failed: {e}"),
                        }
                    }
                }
            } else if let Some((_, text)) = &captured {
                if same_as_last(text.as_bytes()) {
                    continue;
                }
                set_last_seen(text.as_bytes());
            }

            let Some((ctype, content)) = captured else {
                continue;
            };
            let inserted = app
                .state::<Mutex<crate::db::Db>>()
                .lock()
                .map(|mut db| {
                    db.insert(
                        ctype,
                        &content,
                        &content,
                        None,
                        None,
                        src_app.as_deref(),
                        src_name.as_deref(),
                    )
                })
                .ok()
                .and_then(|r| r.ok())
                .flatten();
            if let Some(id) = inserted {
                if ctype == "link" {
                    fetch_title_async(app.clone(), id, content.clone());
                }
                if let Some(bundle) = &src_app {
                    cache_app_icon(&app, bundle);
                }
                let _ = app.emit("clips:updated", ());
            }
        }
    });
}
