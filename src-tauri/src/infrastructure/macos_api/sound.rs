//! Play audible UI sounds using cached NSSound instances for low latency.
//!
//! macOS renames legacy files in System Settings → Sound → Sound Effects:
//! - Boop  → Tink.aiff
//! - Jump  → Frog.aiff
//! (see AlertSounds.loctable in Sound.appex)

use std::cell::RefCell;
use std::thread::LocalKey;

use objc2::rc::{autoreleasepool, Retained};
use objc2_app_kit::NSSound;
use objc2_foundation::NSString;

thread_local! {
    static COPY_SOUND: RefCell<Option<Retained<NSSound>>> = const { RefCell::new(None) };
    static PASTE_SOUND: RefCell<Option<Retained<NSSound>>> = const { RefCell::new(None) };
}

pub fn play_clipboard_sound(kind: &str, volume: f64) {
    let vol = volume.clamp(0.0, 1.0);
    if vol <= 0.0 {
        return;
    }

    let sound_name = match kind {
        "paste" => "Frog", // Jump
        _ => "Tink",       // Boop
    };

    if play_cached_nssound(kind, sound_name, vol) {
        return;
    }

    // Fallback only if AppKit cannot resolve the system sound name.
    let path = format!("/System/Library/Sounds/{}.aiff", sound_name);
    let vol_arg = format!("{:.3}", vol);

    std::thread::spawn(move || {
        let _ = std::process::Command::new("/usr/bin/afplay")
            .arg("-v")
            .arg(vol_arg)
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    });
}

fn play_cached_nssound(kind: &str, sound_name: &str, volume: f64) -> bool {
    let cache = match kind {
        "paste" => &PASTE_SOUND,
        _ => &COPY_SOUND,
    };

    play_from_cache(cache, sound_name, volume as f32)
}

fn play_from_cache(
    cache: &'static LocalKey<RefCell<Option<Retained<NSSound>>>>,
    sound_name: &str,
    volume: f32,
) -> bool {
    autoreleasepool(|_| {
        cache.with(|slot| {
            let mut cached_sound = slot.borrow_mut();
            if cached_sound.is_none() {
                let ns_name = NSString::from_str(sound_name);
                *cached_sound = NSSound::soundNamed(&ns_name);
            }

            let Some(sound) = cached_sound.as_ref() else {
                return false;
            };

            sound.setVolume(volume);
            let _ = sound.stop();
            sound.setCurrentTime(0.0);
            sound.play()
        })
    })
}
