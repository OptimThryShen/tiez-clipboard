#[cfg(target_os = "macos")]
use clipboard_rs::{ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};
use std::sync::Arc;

#[cfg(target_os = "macos")]
struct MacClipboardHandler {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
}

#[cfg(target_os = "macos")]
impl ClipboardHandler for MacClipboardHandler {
    fn on_clipboard_change(&mut self) {
        // Do not debounce change notifications here. A later clipboard value
        // cannot be reconstructed after its event is dropped. The monitor uses
        // a payload hash to discard genuine duplicate notifications instead.
        (self.callback)();
    }
}

pub fn listen_clipboard(callback: Arc<dyn Fn() + Send + Sync + 'static>) {
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(move || {
            let mut watcher = match ClipboardWatcherContext::new() {
                Ok(w) => w,
                Err(err) => {
                    eprintln!(">>> [CLIPBOARD] failed to create mac watcher: {}", err);
                    return;
                }
            };

            watcher.add_handler(MacClipboardHandler { callback });
            watcher.start_watch();
        });
        return;
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || {
            let mut last_seq =
                crate::infrastructure::windows_api::win_clipboard::get_clipboard_sequence_number();
            loop {
                let current_seq =
                    crate::infrastructure::windows_api::win_clipboard::get_clipboard_sequence_number();
                if current_seq != last_seq {
                    println!(
                        ">>> [CLIPBOARD] Windows change detected (seq: {})",
                        current_seq
                    );
                    last_seq = current_seq;
                    callback();
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });
        return;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = callback;
        eprintln!(">>> [CLIPBOARD] listener unsupported on this platform");
    }
}
