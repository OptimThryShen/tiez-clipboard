//! Passive global listener for physical ⌘V — plays paste sound without intercepting the shortcut.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::permissions::{
    has_accessibility_permission, request_accessibility_permission, V_KEY_CODE,
};

type CFMachPortRef = *mut std::ffi::c_void;
type CFRunLoopRef = *mut std::ffi::c_void;
type CFRunLoopSourceRef = *mut std::ffi::c_void;
type CGEventRef = *mut std::ffi::c_void;
type CGEventTapProxy = *mut std::ffi::c_void;
type CGEventType = u32;
type CGEventMask = u64;
type CGEventFlags = u64;
type CGEventTapLocation = u32;
type CGEventTapPlacement = u32;
type CGEventTapOptions = u32;
type CGEventField = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

const K_CG_EVENT_LEFT_MOUSE_DOWN: CGEventType = 1;
const K_CG_EVENT_LEFT_MOUSE_UP: CGEventType = 2;
const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: CGEventType = 0xFFFF_FFFE;
const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: CGEventType = 0xFFFF_FFFF;
const K_CG_EVENT_TAP_OPTION_EVENT_EDIT: CGEventTapOptions = 0;
const K_CG_HEAD_INSERT_EVENT_TAP: CGEventTapPlacement = 0;
const K_CG_HID_EVENT_TAP: CGEventTapLocation = 0;
const K_CG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 1 << 20;
const K_CG_EVENT_FLAG_MASK_SHIFT: CGEventFlags = 1 << 17;
const K_CG_EVENT_FLAG_MASK_CONTROL: CGEventFlags = 1 << 18;
const K_CG_EVENT_FLAG_MASK_ALTERNATE: CGEventFlags = 1 << 19;
const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
const K_CG_KEYBOARD_EVENT_AUTOREPEAT: CGEventField = 11;

// macOS virtual key codes for clipboard-list navigation.
const K_VK_UP_ARROW: u16 = 0x7E;
const K_VK_DOWN_ARROW: u16 = 0x7D;
const K_VK_RETURN: u16 = 0x24;
const K_VK_ESCAPE: u16 = 0x35;

static SUPPRESS_CMDV_SOUND_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
static MONITOR_STARTED: OnceLock<()> = OnceLock::new();
static ACCESSIBILITY_PROMPTED: OnceLock<()> = OnceLock::new();
static ACTIVE_EVENT_TAP: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
static SWALLOW_NEXT_LEFT_MOUSE_UP: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Skip tap feedback briefly after TieZ posts a synthetic ⌘V (schedule_paste_sound handles that).
pub fn suppress_paste_sound_briefly() {
    SUPPRESS_CMDV_SOUND_UNTIL_MS.store(now_ms().saturating_add(250), Ordering::Release);
}

fn should_play_for_physical_cmd_v() -> bool {
    now_ms() >= SUPPRESS_CMDV_SOUND_UNTIL_MS.load(Ordering::Acquire)
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        events_of_interest: CGEventMask,
        callback: Option<
            unsafe extern "C" fn(
                CGEventTapProxy,
                CGEventType,
                CGEventRef,
                *mut std::ffi::c_void,
            ) -> CGEventRef,
        >,
        user_info: *mut std::ffi::c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
}

fn main_window_click_payload(point: CGPoint) -> Option<serde_json::Value> {
    use tauri::Manager;

    if crate::global_state::IS_HIDDEN.load(Ordering::Relaxed)
        || !crate::global_state::NAVIGATION_ENABLED.load(Ordering::SeqCst)
    {
        return None;
    }

    let Some(app) = crate::global_state::GLOBAL_APP_HANDLE.get() else {
        return None;
    };
    let Some(window) = app.get_webview_window("main") else {
        return None;
    };
    if !window.is_visible().unwrap_or(false) {
        return None;
    }

    let (Ok(pos), Ok(size)) = (
        window.inner_position().or_else(|_| window.outer_position()),
        window.inner_size().or_else(|_| window.outer_size()),
    ) else {
        return None;
    };
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);

    // CGEventGetLocation is in global logical points; Tauri positions are physical pixels.
    let left = pos.x as f64 / scale;
    let top = pos.y as f64 / scale;
    let right = left + size.width as f64 / scale;
    let bottom = top + size.height as f64 / scale;

    if point.x < left || point.x > right || point.y < top || point.y > bottom {
        return Some(serde_json::json!({ "outside": true }));
    }

    let client_x = point.x - left;
    let client_y = point.y - top;

    // Header chrome (drag region, traffic lights, search box) must keep the native
    // mouse stream so `startDragging()` and header buttons work on the first click.
    let header_pass_height =
        crate::global_state::MACOS_HEADER_PASS_HEIGHT.load(Ordering::Relaxed) as f64;
    if client_y <= header_pass_height {
        return Some(serde_json::json!({ "passThrough": true }));
    }

    Some(serde_json::json!({
        "clientX": client_x,
        "clientY": client_y,
        "screenX": point.x,
        "screenY": point.y
    }))
}

fn hide_main_window_for_outside_click() {
    use tauri::{Emitter, Manager};

    if crate::global_state::WINDOW_PINNED.load(Ordering::Relaxed)
        || crate::global_state::IS_HIDDEN.load(Ordering::Relaxed)
    {
        return;
    }

    let Some(app) = crate::global_state::GLOBAL_APP_HANDLE.get() else {
        return;
    };
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }

    let _ = app.emit("force-hide-compact-preview", ());
    let _ = window.set_always_on_top(false);
    let _ = window.set_focusable(false);
    crate::app::window_manager::clear_window_vibrancy(&window);
    let _ = window.hide();
    crate::global_state::NAVIGATION_ENABLED.store(false, Ordering::SeqCst);
    crate::global_state::NAVIGATION_MODE_ACTIVE.store(false, Ordering::SeqCst);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const std::ffi::c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(
        rl: CFRunLoopRef,
        source: CFRunLoopSourceRef,
        mode: *const std::ffi::c_void,
    );
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: *const std::ffi::c_void;
}

unsafe extern "C" fn cmd_v_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    _user_info: *mut std::ffi::c_void,
) -> CGEventRef {
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        let tap = ACTIVE_EVENT_TAP.load(Ordering::Acquire);
        if !tap.is_null() {
            CGEventTapEnable(tap as CFMachPortRef, true);
        }
        return event;
    }

    if (event_type == K_CG_EVENT_LEFT_MOUSE_DOWN || event_type == K_CG_EVENT_LEFT_MOUSE_UP)
        && !event.is_null()
    {
        if event_type == K_CG_EVENT_LEFT_MOUSE_UP
            && SWALLOW_NEXT_LEFT_MOUSE_UP.swap(false, Ordering::SeqCst)
        {
            return std::ptr::null_mut();
        }

        if event_type == K_CG_EVENT_LEFT_MOUSE_DOWN {
            let Some(payload) = main_window_click_payload(CGEventGetLocation(event)) else {
                return event;
            };
            if payload.get("outside").and_then(|v| v.as_bool()).unwrap_or(false) {
                hide_main_window_for_outside_click();
                return event;
            }
            if payload
                .get("passThrough")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return event;
            }
            SWALLOW_NEXT_LEFT_MOUSE_UP.store(true, Ordering::SeqCst);
            if let Some(app) = crate::global_state::GLOBAL_APP_HANDLE.get() {
                use tauri::Emitter;
                let _ = app.emit("macos-nonactivating-click", payload);
            }
            return std::ptr::null_mut();
        }
    }

    if event_type == K_CG_EVENT_KEY_DOWN && !event.is_null() {
        let flags = CGEventGetFlags(event);
        let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) as u16;
        let autorepeat = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_AUTOREPEAT);

        if autorepeat == 0
            && keycode == V_KEY_CODE
            && (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0
            && should_play_for_physical_cmd_v()
        {
            if let Some(app) = crate::global_state::GLOBAL_APP_HANDLE.get() {
                crate::services::ui_sound::schedule_paste_sound(app);
            }
        }

        // Clipboard-list navigation. Because `toggle_window` no longer calls
        // `set_focus()` on macOS (to avoid stealing focus from the previous
        // app's text input, e.g. a Finder rename), TieZ is often NOT the
        // frontmost app while the clipboard window is shown, so the webview
        // cannot receive arrow/Enter/Esc via DOM keydown. This HID-level tap
        // captures those keys system-wide and routes them to the same
        // `navigation-action` channel the Windows low-level hook uses.
        //
        // Swallowing at the HID layer also means the previous (frontmost) app
        // never sees the key (so e.g. a rename caret won't move), and the
        // webview's DOM keydown never fires either, avoiding double-stepping.
        let no_modifiers = (flags
            & (K_CG_EVENT_FLAG_MASK_COMMAND
                | K_CG_EVENT_FLAG_MASK_SHIFT
                | K_CG_EVENT_FLAG_MASK_CONTROL
                | K_CG_EVENT_FLAG_MASK_ALTERNATE))
            == 0;

        if autorepeat == 0
            && no_modifiers
            && crate::global_state::NAVIGATION_ENABLED.load(Ordering::SeqCst)
            && !crate::global_state::IS_HIDDEN.load(Ordering::Relaxed)
        {
            let nav_action: Option<&'static str> = match keycode {
                K_VK_UP_ARROW => Some("up"),
                K_VK_DOWN_ARROW => Some("down"),
                K_VK_RETURN => {
                    if crate::global_state::NAVIGATION_MODE_ACTIVE.load(Ordering::Relaxed) {
                        Some("enter")
                    } else {
                        None
                    }
                }
                K_VK_ESCAPE => Some("escape"),
                _ => None,
            };

            if let Some(action) = nav_action {
                if let Some(app) = crate::global_state::GLOBAL_APP_HANDLE.get() {
                    use tauri::Emitter;
                    match action {
                        "up" | "down" => {
                            crate::global_state::NAVIGATION_MODE_ACTIVE
                                .store(true, Ordering::Relaxed);
                        }
                        "escape" => {
                            crate::global_state::NAVIGATION_MODE_ACTIVE
                                .store(false, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    if action == "escape" {
                        // Mirror the Windows hook: emit escape, then hide.
                        let _ = app.emit("navigation-action", "escape");
                        crate::app::window_manager::toggle_window(app);
                    } else {
                        let _ = app.emit("navigation-action", action);
                    }
                }
                return std::ptr::null_mut();
            }
        }
    }
    event
}

/// Returns true if the monitor run loop was started successfully.
unsafe fn try_start_event_tap() -> bool {
    if !has_accessibility_permission() {
        return false;
    }

    let mask: CGEventMask = (1u64 << K_CG_EVENT_KEY_DOWN)
        | (1u64 << K_CG_EVENT_LEFT_MOUSE_DOWN)
        | (1u64 << K_CG_EVENT_LEFT_MOUSE_UP);
    let tap = CGEventTapCreate(
        K_CG_HID_EVENT_TAP,
        K_CG_HEAD_INSERT_EVENT_TAP,
        K_CG_EVENT_TAP_OPTION_EVENT_EDIT,
        mask,
        Some(cmd_v_callback),
        std::ptr::null_mut(),
    );
    if tap.is_null() {
        eprintln!("[paste-key-monitor] CGEventTapCreate returned null");
        return false;
    }

    ACTIVE_EVENT_TAP.store(tap as *mut std::ffi::c_void, Ordering::Release);

    let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
    if source.is_null() {
        eprintln!("[paste-key-monitor] Failed to create run loop source");
        ACTIVE_EVENT_TAP.store(std::ptr::null_mut(), Ordering::Release);
        return false;
    }

    let run_loop = CFRunLoopGetCurrent();
    CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

    CGEventTapEnable(tap, true);
    eprintln!("[paste-key-monitor] ⌘V listener active");
    CFRunLoopRun();
    true
}

pub fn start_paste_key_monitor() {
    if MONITOR_STARTED.set(()).is_err() {
        return;
    }

    std::thread::Builder::new()
        .name("tiez-paste-key-monitor".into())
        .spawn(|| {
            loop {
                if !has_accessibility_permission() {
                    if ACCESSIBILITY_PROMPTED.set(()).is_ok() {
                        let _ = request_accessibility_permission();
                    }
                } else if unsafe { try_start_event_tap() } {
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .ok();
}
