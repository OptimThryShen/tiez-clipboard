use crate::global_state::TASKBAR_CREATED_MSG;
use std::sync::atomic::Ordering;
#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::DefSubclassProc;

pub const LEGACY_PLACEHOLDER_MACHINE_ID: &str = "MAC-DEVICE-000000";
pub const LEGACY_PLACEHOLDER_ANON_ID: &str = "MAC-DEVICE-000000-0000-0000-0000-000000000000";
pub const LEGACY_ZERO_SUFFIX: &str = "-0000-0000-0000-000000000000";

fn hash_to_short_id(seed: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex.chars().take(8).collect()
}

fn fallback_machine_id() -> String {
    let mut seed = String::new();

    for key in ["COMPUTERNAME", "HOSTNAME", "USER", "USERNAME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                seed.push_str(trimmed);
                seed.push('|');
            }
        }
    }

    if seed.is_empty() {
        seed.push_str("tiez-device");
    }

    hash_to_short_id(&seed)
}

pub fn get_machine_id() -> String {
    match machine_uid::get() {
        Ok(machine_uid) if !machine_uid.trim().is_empty() => hash_to_short_id(machine_uid.trim()),
        Ok(_) => fallback_machine_id(),
        Err(e) => {
            eprintln!("[WARN] Failed to get machine UID: {}. Using fallback.", e);
            fallback_machine_id()
        }
    }
}

pub fn build_anon_id(machine_id: &str) -> String {
    machine_id.trim().to_string()
}

pub fn is_legacy_placeholder_anon_id(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed == LEGACY_PLACEHOLDER_MACHINE_ID || trimmed == LEGACY_PLACEHOLDER_ANON_ID
}

pub fn normalize_anon_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_legacy_placeholder_anon_id(trimmed) {
        return None;
    }

    if let Some(prefix) = trimmed.strip_suffix(LEGACY_ZERO_SUFFIX) {
        let normalized = prefix.trim();
        if normalized.is_empty() || is_legacy_placeholder_anon_id(normalized) {
            return None;
        }
        return Some(normalized.to_string());
    }

    Some(trimmed.to_string())
}

pub fn is_same_device_id(id1: &str, id2: &str) -> bool {
    let n1 = normalize_anon_id(id1);
    let n2 = normalize_anon_id(id2);
    n1.is_some() && n1 == n2
}

pub fn same_anon_id(left: &str, right: &str) -> bool {
    match (normalize_anon_id(left), normalize_anon_id(right)) {
        (Some(l), Some(r)) => l == r,
        _ => is_same_device_id(left, right),
    }
}

/// Window subclass procedure to handle taskbar recreation (explorer restart)
#[cfg(target_os = "windows")]
pub unsafe extern "system" fn tray_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if msg != 0 && msg == taskbar_msg {
        if let Some(app_handle) = crate::GLOBAL_APP_HANDLE.get() {
            let handle = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                if let Some(settings) = handle.try_state::<crate::app_state::SettingsState>() {
                    if settings.hide_tray_icon.load(Ordering::Relaxed) {
                        if let Some(tray) = handle.tray_by_id("main_tray") {
                            let _ = tray.set_visible(false);
                            println!(">>> [TRAY] Explorer restart detected, re-hiding tray icon per user setting.");
                        }
                    }
                }
            });
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
