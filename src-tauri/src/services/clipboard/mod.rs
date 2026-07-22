mod pipeline;
mod utils;

use crate::app_state::SettingsState;
pub use crate::database::DbState;
use arboard::Clipboard;
use base64::Engine;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
fn get_platform_clipboard_html() -> Option<String> {
    crate::infrastructure::macos_api::clipboard::get_clipboard_html()
}

#[cfg(target_os = "windows")]
fn get_platform_clipboard_html() -> Option<String> {
    unsafe {
        crate::infrastructure::windows_api::win_clipboard::get_clipboard_raw_format("HTML Format")
    }
    .and_then(|raw| utils::parse_cf_html(&raw))
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn get_platform_clipboard_html() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
const MAX_MACOS_TEXT_BYTES: usize = 128 * 1024;
fn build_rich_image_fallback_data_url(
    width: usize,
    height: usize,
    rgba_bytes: &[u8],
) -> Option<String> {
    let png_bytes = encode_clipboard_image_to_png(width, height, rgba_bytes)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Some(format!("data:image/png;base64,{}", b64))
}

fn encode_clipboard_image_to_png(
    width: usize,
    height: usize,
    rgba_bytes: &[u8],
) -> Option<Vec<u8>> {
    let img_buf = image::RgbaImage::from_raw(width as u32, height as u32, rgba_bytes.to_vec())?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;
    Some(bytes)
}

pub fn start_clipboard_monitor(app_handle: AppHandle) {
    use std::sync::{Arc, Mutex};

    // Initial state for deduplication and self-copy detection.
    let mut last_image_hash = 0u64;

    if let Ok(mut cb) = Clipboard::new() {
        if let Ok(img) = cb.get_image() {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            img.bytes.hash(&mut hasher);
            last_image_hash = hasher.finish();
        }
    }

    struct MonitorState {
        last_image_hash: u64,
        last_content_hash: u64,
    }

    let state = Arc::new(Mutex::new(MonitorState {
        last_image_hash,
        last_content_hash: 0,
    }));

    let app_clone = app_handle.clone();
    let state_lock = state.clone();

    // Start the native clipboard listener
    crate::services::clipboard_listener::listen_clipboard(Arc::new(move || {
        let app = app_clone.clone();
        let mut monitor_state = state_lock.lock().unwrap();

        // 1. Check for pause
        if crate::CLIPBOARD_MONITOR_PAUSED.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        // 2. We don't use sequence check on macOS as we rely on polling/listener triggering.

        // Snapshot file URLs immediately. Finder can replace the pasteboard
        // again while we wait for richer text formats, making an earlier file
        // event otherwise observe a later file and lose the intermediate one.
        #[cfg(target_os = "macos")]
        let clipboard_files_snapshot =
            crate::infrastructure::macos_api::clipboard::get_clipboard_files();

        // Give source apps (especially Excel) a brief moment to finish writing
        // non-file payloads. File URLs are already complete in the event snapshot.
        #[cfg(target_os = "macos")]
        if clipboard_files_snapshot.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        #[cfg(not(target_os = "macos"))]
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Initialize clipboard for this thread
        #[cfg(not(target_os = "macos"))]
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(_) => return,
        };
        #[cfg(target_os = "macos")]
        let mut clipboard = Clipboard::new().ok();
        #[cfg(target_os = "macos")]
        let text_from_clipboard = crate::infrastructure::macos_api::clipboard::get_clipboard_text();

        #[cfg(target_os = "macos")]
        let clipboard_image = clipboard
            .as_mut()
            .and_then(|cb| cb.get_image().ok());
        #[cfg(not(target_os = "macos"))]
        let clipboard_image = clipboard.get_image().ok();

        // Calculate hash of current clipboard content
        let current_content_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();

            // Hash text content if available
            #[cfg(target_os = "macos")]
            {
                if let Some(file_paths) = clipboard_files_snapshot.as_ref() {
                    file_paths.hash(&mut hasher);
                }
                if let Some(text) = &text_from_clipboard {
                    if text.len() > MAX_MACOS_TEXT_BYTES {
                        "__TEXT_TOO_LARGE__".hash(&mut hasher);
                    } else {
                        text.hash(&mut hasher);
                    }
                }
                if let Some(html) = get_platform_clipboard_html() {
                    if html.len() > MAX_MACOS_TEXT_BYTES {
                        "__HTML_TOO_LARGE__".hash(&mut hasher);
                    } else {
                        html.hash(&mut hasher);
                    }
                } else if crate::infrastructure::macos_api::clipboard::get_clipboard_rtf().is_some() {
                    "__RTF__".hash(&mut hasher);
                }
            }
            #[cfg(not(target_os = "macos"))]
            if let Ok(text) = clipboard.get_text() {
                text.hash(&mut hasher);
            }

            if let Some(image) = &clipboard_image {
                image.bytes.hash(&mut hasher);
            }

            hasher.finish()
        };

        // Strict duplicate guard: identical payload should never be processed twice.
        if current_content_hash == monitor_state.last_content_hash && current_content_hash != 0 {
            return;
        }

        monitor_state.last_content_hash = current_content_hash;

        let mut handled = false;

        // --- Core processing logic (same as before) ---

        // 1. Check Files (macOS)
        #[cfg(target_os = "macos")]
        {
            if let Some(file_paths) = clipboard_files_snapshot {
                let settings = app.state::<SettingsState>();
                if settings.capture_files.load(Ordering::Relaxed) {
                    process_new_entry(&app, ClipboardData::Files(file_paths), None);
                }
                // Whether we captured or skipped, we MUST mark as handled so the file
                // icon (image) and filename (text) are NOT captured.
                return;
            }
        }

        #[cfg(target_os = "macos")]
        let clipboard_html_snapshot = {
            let settings = app.state::<SettingsState>();
            if settings.capture_rich_text.load(Ordering::Relaxed) {
                let mut html = get_platform_clipboard_html();
                if html.as_ref().map(|value| value.trim().is_empty()).unwrap_or(true) {
                    if let Some(rtf) =
                        crate::infrastructure::macos_api::clipboard::get_clipboard_rtf()
                    {
                        html =
                            crate::infrastructure::macos_api::clipboard::convert_rtf_to_html(&rtf);
                    }
                }
                html
            } else {
                None
            }
        };

        #[cfg(target_os = "macos")]
        let clipboard_rtf_snapshot =
            crate::infrastructure::macos_api::clipboard::get_clipboard_rtf();

        // 2. Check Image
        if !handled {
            let settings = app.state::<SettingsState>();
            let rich_text_enabled = settings.capture_rich_text.load(Ordering::Relaxed);
            #[cfg(not(target_os = "macos"))]
            let has_text = clipboard
                .get_text()
                .map(|t| !t.is_empty())
                .unwrap_or(false);
            #[cfg(target_os = "macos")]
            let has_rich_html = rich_text_enabled
                && clipboard_html_snapshot
                    .as_ref()
                    .map(|html| !html.trim().is_empty())
                    .unwrap_or(false);
            #[cfg(not(target_os = "macos"))]
            let has_rich_html = if rich_text_enabled && has_text {
                get_platform_clipboard_html()
                    .map(|html| !html.trim().is_empty())
                    .unwrap_or(false)
            } else {
                false
            };

            // Rich text wins over image when rich HTML exists; image remains fallback for pure image content.
            if !has_rich_html {
                if !handled {
                    if let Some(image) = clipboard_image.as_ref() {
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        use std::hash::{Hash, Hasher};
                        image.bytes.hash(&mut hasher);
                        let hash = hasher.finish();

                        if hash != monitor_state.last_image_hash {
                            let last_app_hash = crate::LAST_APP_SET_HASH.load(Ordering::SeqCst);
                            let last_app_time =
                                crate::LAST_APP_SET_TIMESTAMP.load(Ordering::SeqCst);
                            let now_secs = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let last_app_hash_alt =
                                crate::LAST_APP_SET_HASH_ALT.load(Ordering::SeqCst);

                            if last_app_hash != 0
                                && (last_app_hash == hash || last_app_hash_alt == hash)
                                && (now_secs - last_app_time) < 10
                            {
                                crate::LAST_APP_SET_HASH.store(0, Ordering::SeqCst);
                                crate::LAST_APP_SET_HASH_ALT.store(0, Ordering::SeqCst);
                            } else if let Some(png_bytes) = encode_clipboard_image_to_png(
                                image.width,
                                image.height,
                                &image.bytes,
                            ) {
                                process_new_entry(
                                    &app,
                                    ClipboardData::Image { png_bytes },
                                    None,
                                );
                                handled = true;
                            }
                            monitor_state.last_image_hash = hash;
                        }
                    }
                }
            }
        }

        if !handled {
            #[cfg(target_os = "macos")]
            {
                let settings = app.state::<SettingsState>();
                let rich_text_enabled = settings.capture_rich_text.load(Ordering::Relaxed);
                let active_app =
                    crate::infrastructure::macos_api::window::get_active_app_snapshot();

                let resolved = utils::resolve_clipboard_text_capture(
                    text_from_clipboard.as_deref(),
                    clipboard_html_snapshot.as_deref(),
                    None,
                    rich_text_enabled,
                    &active_app.app_name,
                    active_app.process_path.as_deref(),
                );

                if let Some(payload) = resolved {
                    let text = payload.text;
                    if text.len() > MAX_MACOS_TEXT_BYTES {
                        eprintln!(
                            ">>> [CLIPBOARD] Skip capture: text exceeds {} bytes",
                            MAX_MACOS_TEXT_BYTES
                        );
                        return;
                    }

                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher};
                    utils::normalize_clipboard_line_endings(&text).hash(&mut hasher);
                    let current_hash = hasher.finish();

                    let last_app_hash = crate::LAST_APP_SET_HASH.load(Ordering::SeqCst);
                    let last_app_time = crate::LAST_APP_SET_TIMESTAMP.load(Ordering::SeqCst);
                    let now_secs = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    if (last_app_hash != 0
                        && (current_hash == last_app_hash
                            || current_hash == crate::LAST_APP_SET_HASH_ALT.load(Ordering::SeqCst)))
                        && (now_secs - last_app_time) < 10
                    {
                        crate::LAST_APP_SET_HASH.store(0, Ordering::SeqCst);
                        crate::LAST_APP_SET_HASH_ALT.store(0, Ordering::SeqCst);
                        return;
                    }

                    if rich_text_enabled {
                        if let Some(mut html) = payload.html {
                            if !html.trim().is_empty() {
                                let image_opt = clipboard_image.as_ref();
                                if let Some(image) = image_opt {
                                    if utils::should_attach_rich_image_fallback_on_capture(
                                        &active_app.app_name,
                                        active_app.process_path.as_deref(),
                                    ) {
                                        if let Some(data_url) = build_rich_image_fallback_data_url(
                                            image.width,
                                            image.height,
                                            &image.bytes,
                                        ) {
                                            html = utils::attach_rich_image_fallback(&html, &data_url);
                                        }
                                    }
                                }

                                if let Some(rtf_bytes) = clipboard_rtf_snapshot.clone() {
                                    html = utils::attach_rich_named_formats(
                                        &html,
                                        &[crate::infrastructure::windows_api::win_clipboard::NamedClipboardFormat {
                                            name: "Rich Text Format".to_string(),
                                            data: rtf_bytes,
                                        }],
                                    );
                                }

                                process_new_entry(
                                    &app,
                                    ClipboardData::RichText {
                                        text: text.clone(),
                                        html,
                                    },
                                    None,
                                );
                                handled = true;
                            }
                        }
                    }

                    if !handled {
                        if last_app_hash != 0 {
                            crate::LAST_APP_SET_HASH.store(0, Ordering::SeqCst);
                        }
                        process_new_entry(&app, ClipboardData::Text(text), None);
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    let settings = app.state::<SettingsState>();

                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher};
                    utils::normalize_clipboard_line_endings(&text).hash(&mut hasher);
                    let current_hash = hasher.finish();

                    let last_app_hash = crate::LAST_APP_SET_HASH.load(Ordering::SeqCst);
                    let last_app_time = crate::LAST_APP_SET_TIMESTAMP.load(Ordering::SeqCst);
                    let now_secs = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    if (last_app_hash != 0
                        && (current_hash == last_app_hash
                            || current_hash == crate::LAST_APP_SET_HASH_ALT.load(Ordering::SeqCst)))
                        && (now_secs - last_app_time) < 10
                    {
                        crate::LAST_APP_SET_HASH.store(0, Ordering::SeqCst);
                        crate::LAST_APP_SET_HASH_ALT.store(0, Ordering::SeqCst);
                        return;
                    }

                    if settings.capture_rich_text.load(Ordering::Relaxed) {
                        if let Some(mut html) =
                            get_platform_clipboard_html()
                        {
                            if !html.trim().is_empty() {
                                if let Some(image) = clipboard_image.as_ref() {
                                    #[cfg(target_os = "windows")]
                                    let capture_source = {
                                        let info = crate::infrastructure::windows_api::window_tracker::get_clipboard_source_app_info();
                                        (info.app_name, info.process_path)
                                    };
                                    #[cfg(not(target_os = "windows"))]
                                    let capture_source = (
                                        crate::global_state::get_last_active_app_name(),
                                        None::<String>,
                                    );
                                    if utils::should_attach_rich_image_fallback_on_capture(
                                        &capture_source.0,
                                        capture_source.1.as_deref(),
                                    ) {
                                        if let Some(data_url) = build_rich_image_fallback_data_url(
                                            image.width,
                                            image.height,
                                            &image.bytes,
                                        ) {
                                            html = utils::attach_rich_image_fallback(&html, &data_url);
                                        }
                                    }
                                }

                                process_new_entry(
                                    &app,
                                    ClipboardData::RichText {
                                        text: text.clone(),
                                        html,
                                    },
                                    None,
                                );
                                handled = true;
                            }
                        }
                    }

                    if !handled {
                        if last_app_hash != 0 {
                            crate::LAST_APP_SET_HASH.store(0, Ordering::SeqCst);
                        }
                        process_new_entry(&app, ClipboardData::Text(text), None);
                    }
                }
            }
        }
    }));
}

#[cfg(target_os = "windows")]
pub fn capture_preserved_named_formats_from_clipboard(
    _source_snapshot: Option<&str>,
) -> Vec<crate::infrastructure::windows_api::win_clipboard::NamedClipboardFormat> {
    vec![]
}

#[cfg(not(target_os = "windows"))]
pub fn capture_preserved_named_formats_from_clipboard(
    _source_snapshot: Option<&str>,
) -> Vec<crate::infrastructure::windows_api::win_clipboard::NamedClipboardFormat> {
    vec![]
}

#[cfg(target_os = "windows")]
pub fn clipboard_image_fallback_data_url() -> Option<String> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn clipboard_image_fallback_data_url() -> Option<String> {
    None
}

pub use pipeline::{ClipboardData, ClipboardPipeline, PipelineContext};
pub use utils::{
    attach_rich_image_fallback, attach_rich_named_formats, build_clipboard_text_fingerprint,
    build_entry_preview, build_exact_paste_html, derive_rich_text_content, embed_local_images,
    encode_cf_html, entry_matches_search, entry_searchable_text,
    extract_animated_image_data_url_from_html, extract_animated_image_data_url_from_text,
    extract_first_image_data_url_from_html, html_has_renderable_rich_body,
    normalize_clipboard_line_endings,
    normalize_plain_text_for_clipboard_paste, plain_text_requires_exact_paste, parse_cf_html,
    plain_text_from_tabular_html,
    repair_html_fragment, rtf_bytes_from_named_formats, sanitize_tabular_html_for_paste,
    should_attach_rich_image_fallback_on_capture, should_use_rich_image_clipboard_fallback,
    split_rich_html_and_image_fallback, split_rich_html_and_named_formats,
    truncate_html_for_preview, app_likely_word_processor,
};

const MAX_PIPELINE_TEXT_BYTES: usize = 128 * 1024;
const MAX_PIPELINE_HTML_BYTES: usize = 512 * 1024;

pub fn process_new_entry(
    app_handle: &AppHandle,
    data: ClipboardData,
    source_override: Option<String>,
) {
    let payload_too_large = match &data {
        ClipboardData::Text(text) => text.len() > MAX_PIPELINE_TEXT_BYTES,
        ClipboardData::RichText { text, html } => {
            text.len() > MAX_PIPELINE_TEXT_BYTES || html.len() > MAX_PIPELINE_HTML_BYTES
        }
        _ => false,
    };

    if payload_too_large {
        eprintln!(
            ">>> [CLIPBOARD] Skip pipeline: payload exceeds text/html limits (text:{} bytes, html:{} bytes)",
            MAX_PIPELINE_TEXT_BYTES,
            MAX_PIPELINE_HTML_BYTES
        );
        return;
    }

    let mut ctx = PipelineContext::new(app_handle.clone(), data);
    if let Some(source) = source_override {
        ctx.source_app = source;
        ctx.source_app_path = None;
    }

    let pipeline = ClipboardPipeline::new();
    pipeline.execute(&mut ctx);
}
