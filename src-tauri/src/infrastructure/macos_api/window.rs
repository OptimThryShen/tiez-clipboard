#[cfg(target_os = "macos")]
use objc2::rc::autoreleasepool;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSWorkspace;
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::Manager;

#[derive(Debug, Clone, Default)]
pub struct ActiveAppInfo {
    pub app_name: String,
    pub pid: u32,
    pub process_path: Option<String>,
}

pub fn start_window_tracking(_app_handle: AppHandle) {
    #[cfg(target_os = "macos")]
    std::thread::spawn(move || {
        use std::sync::atomic::Ordering;
        let self_pid = std::process::id();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            // Only update track if TieZ is NOT actually focused right now,
            // to avoid overwriting the "previous" app with "TieZ" itself.
            if !crate::global_state::IS_MAIN_WINDOW_FOCUSED.load(Ordering::Relaxed) {
                let info = get_active_app_snapshot();
                if info.pid != 0 {
                    let name = info.app_name;
                    let pid = info.pid;
                    if pid != 0 && pid != self_pid && !name.eq_ignore_ascii_case("TieZ") {
                        crate::global_state::LAST_ACTIVE_APP_PID.store(pid, Ordering::Relaxed);
                        crate::global_state::set_last_active_app_name(name);
                    }
                }
            }
        }
    });
}

pub fn get_active_app_snapshot() -> ActiveAppInfo {
    #[cfg(target_os = "macos")]
    return autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace.frontmostApplication();

        if let Some(app) = app {
            let name = app
                .localizedName()
                .map(|s: objc2::rc::Retained<NSString>| s.to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let pid = app.processIdentifier() as u32;
            let bundle_url = app.bundleURL();
            let process_path = if let Some(url) = bundle_url {
                url.path()
                    .map(|s: objc2::rc::Retained<NSString>| s.to_string())
            } else {
                None
            };

            ActiveAppInfo {
                app_name: name,
                pid,
                process_path,
            }
        } else {
            ActiveAppInfo {
                app_name: "macOS App".into(),
                pid: 0,
                process_path: None,
            }
        }
    });

    #[cfg(not(target_os = "macos"))]
    ActiveAppInfo {
        app_name: "Unknown App".into(),
        pid: 0,
        process_path: None,
    }
}

pub fn get_active_app_info() -> (String, String) {
    let info = get_active_app_snapshot();
    let pid = if info.pid == 0 {
        String::new()
    } else {
        info.pid.to_string()
    };
    (info.app_name, pid)
}

/// `WebviewWindow::is_visible` stays true when a window is visible on another
/// Space. Use AppKit's active-Space and occlusion state for hotkey toggling.
#[cfg(target_os = "macos")]
pub fn is_visible_on_active_space(window: &tauri::WebviewWindow) -> bool {
    use objc2_app_kit::{NSWindow, NSWindowOcclusionState};

    let Ok(ns_window_ptr) = window.ns_window() else {
        return window.is_visible().unwrap_or(false);
    };
    let ns_window = ns_window_ptr as *const NSWindow;
    if ns_window.is_null() {
        return false;
    }

    unsafe {
        (*ns_window).isVisible()
            && (*ns_window).isOnActiveSpace()
            && (*ns_window)
                .occlusionState()
                .contains(NSWindowOcclusionState::Visible)
    }
}

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(ClipboardPanel {
        config: {
            is_floating_panel: true,
            can_become_key_window: true,
            can_become_main_window: false
        }
    })
}

/// After `to_panel`, tao's `focusable` ivar no longer exists on the ObjC class.
/// Calling `WebviewWindow::set_focusable` then panics. Skip it once converted.
#[cfg(target_os = "macos")]
pub fn set_window_focusable(window: &tauri::WebviewWindow, focusable: bool) {
    use tauri_nspanel::ManagerExt;
    if window.app_handle().get_webview_panel("main").is_ok() {
        return;
    }
    let _ = window.set_focusable(focusable);
}

#[cfg(not(target_os = "macos"))]
pub fn set_window_focusable(window: &tauri::WebviewWindow, focusable: bool) {
    let _ = window.set_focusable(focusable);
}

/// Convert the main Tauri NSWindow into a non-activating NSPanel.
///
/// A plain NSWindow cannot appear on another app's native fullscreen Space,
/// even with FullScreenAuxiliary — this is a known AppKit/Tauri limitation.
/// See EcoPaste / tauri-nspanel fullscreen example.
#[cfg(target_os = "macos")]
pub fn setup_clipboard_panel(window: &tauri::WebviewWindow) -> Result<(), String> {
    use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

    let panel = window
        .to_panel::<ClipboardPanel>()
        .map_err(|e| format!("to_panel failed: {e:?}"))?;

    // Dock level sits above normal fullscreen content without using screen-saver.
    panel.set_level(PanelLevel::Dock.value());
    // Keep NonactivatingPanel for fullscreen overlay, but preserve Resizable —
    // StyleMask::empty().nonactivating_panel() alone wiped the resize bit and
    // made the undecorated window edges undraggable.
    panel.set_style_mask(
        StyleMask::empty()
            .nonactivating_panel()
            .resizable()
            .into(),
    );
    // Idle/hidden: move_to_active_space so the next show lands on the current Space
    // (including another app's fullscreen desktop).
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .move_to_active_space()
            .stationary()
            .into(),
    );

    eprintln!("[macos-overlay] main window converted to NSPanel");
    Ok(())
}

#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// Bumps on every show/hide so deferred reinforce work can cancel itself.
#[cfg(target_os = "macos")]
fn overlay_show_generation() -> &'static AtomicU64 {
    static SHOW_GEN: AtomicU64 = AtomicU64::new(0);
    &SHOW_GEN
}

/// Show the clipboard panel on the current Space / fullscreen desktop.
#[cfg(target_os = "macos")]
pub fn show_clipboard_panel(app: &AppHandle) -> bool {
    use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelLevel};

    let Ok(panel) = app.get_webview_panel("main") else {
        eprintln!("[macos-overlay] get_webview_panel(main) failed");
        return false;
    };

    let gen = overlay_show_generation().fetch_add(1, AtomicOrdering::SeqCst) + 1;

    panel.set_level(PanelLevel::Dock.value());
    // can_join_all_spaces + full_screen_auxiliary: appear on the active fullscreen Space.
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .stationary()
            .into(),
    );
    // make_key so the first click reaches the webview (HID no longer synthesizes
    // inside-panel clicks). nonactivating_panel still avoids activating the app.
    panel.show_and_make_key();
    eprintln!("[macos-overlay] panel.show_and_make_key() gen={gen}");
    true
}

/// Hide the clipboard panel and re-arm move_to_active_space for the next show.
#[cfg(target_os = "macos")]
pub fn hide_clipboard_panel(app: &AppHandle) -> bool {
    use tauri_nspanel::{CollectionBehavior, ManagerExt};

    // Cancel reinforce retries from the previous show.
    overlay_show_generation().fetch_add(1, AtomicOrdering::SeqCst);

    let Ok(panel) = app.get_webview_panel("main") else {
        return false;
    };

    panel.hide();
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .move_to_active_space()
            .stationary()
            .into(),
    );
    eprintln!("[macos-overlay] panel.hide()");
    true
}

/// Kept for startup re-apply paths that still hold a WebviewWindow handle.
#[cfg(target_os = "macos")]
pub fn configure_overlay_space_behavior(window: &tauri::WebviewWindow) {
    use tauri_nspanel::{CollectionBehavior, ManagerExt};

    let app = window.app_handle();
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .full_screen_auxiliary()
                .can_join_all_spaces()
                .stationary()
                .into(),
        );
        return;
    }

    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    if let Ok(ns_window_ptr) = window.ns_window() {
        let ns_window = ns_window_ptr as *const NSWindow;
        if ns_window.is_null() {
            return;
        }
        unsafe {
            let current = (*ns_window).collectionBehavior()
                & !NSWindowCollectionBehavior::MoveToActiveSpace
                & !NSWindowCollectionBehavior::FullScreenPrimary
                & !NSWindowCollectionBehavior::FullScreenNone
                & !NSWindowCollectionBehavior::Stationary;
            (*ns_window).setCollectionBehavior(
                current
                    | NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::IgnoresCycle,
            );
            (*ns_window).setHidesOnDeactivate(false);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn configure_overlay_space_behavior_async(window: &tauri::WebviewWindow) {
    let window_for_main = window.clone();
    let _ = window.run_on_main_thread(move || {
        configure_overlay_space_behavior(&window_for_main);
    });
}

/// After panel conversion, show/hide go through [`show_clipboard_panel`].
#[cfg(target_os = "macos")]
pub fn present_overlay_above_fullscreen_async(window: &tauri::WebviewWindow) {
    let app = window.app_handle().clone();
    let _ = window.run_on_main_thread(move || {
        let _ = show_clipboard_panel(&app);
    });
}

/// Re-apply Space flags after tao/window-state settles — without calling show again
/// (repeated show_and_make_key steals the first click / fights hide).
#[cfg(target_os = "macos")]
pub fn reinforce_overlay_above_fullscreen(window: &tauri::WebviewWindow) {
    use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelLevel};

    let gen = overlay_show_generation().load(AtomicOrdering::SeqCst);
    let app_for_retry = window.app_handle().clone();
    std::thread::spawn(move || {
        for delay_ms in [50_u64, 200] {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if overlay_show_generation().load(AtomicOrdering::SeqCst) != gen {
                return;
            }
            let handle = app_for_retry.clone();
            let _ = handle.clone().run_on_main_thread(move || {
                if overlay_show_generation().load(AtomicOrdering::SeqCst) != gen {
                    return;
                }
                if let Ok(panel) = handle.get_webview_panel("main") {
                    panel.set_level(PanelLevel::Dock.value());
                    panel.set_collection_behavior(
                        CollectionBehavior::new()
                            .full_screen_auxiliary()
                            .can_join_all_spaces()
                            .stationary()
                            .into(),
                    );
                }
            });
        }
    });
}
