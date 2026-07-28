#[cfg(target_os = "macos")]
use clipboard_rs::{ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};
use std::sync::Arc;
#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, RegisterClassW, SetWindowLongPtrW, GWLP_USERDATA, HWND_MESSAGE, MSG,
    WM_CLIPBOARDUPDATE, WNDCLASSW,
};

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
        std::thread::spawn(move || unsafe {
            let instance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
                Ok(instance) => instance,
                Err(err) => {
                    eprintln!(">>> [CLIPBOARD] failed to get module handle: {err}");
                    return;
                }
            };
            let class_name: Vec<u16> = "TieZClipboardListener"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassW(&wnd_class);

            let hwnd = match CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                Default::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(HINSTANCE(instance.0)),
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(err) => {
                    eprintln!(">>> [CLIPBOARD] failed to create Windows listener window: {err}");
                    return;
                }
            };

            let callback_ptr = Box::into_raw(Box::new(callback));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, callback_ptr as isize);
            if let Err(err) = AddClipboardFormatListener(hwnd) {
                eprintln!(">>> [CLIPBOARD] failed to register Windows listener: {err}");
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(callback_ptr));
                let _ = DestroyWindow(hwnd);
                return;
            }

            println!(">>> [CLIPBOARD] Windows event-driven listener started");
            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, None, 0, 0);
                if result.0 <= 0 {
                    break;
                }
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(callback_ptr));
            let _ = DestroyWindow(hwnd);
        });
        return;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = callback;
        eprintln!(">>> [CLIPBOARD] listener unsupported on this platform");
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        let callback_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if callback_ptr != 0 {
            let callback = &*(callback_ptr as *const Arc<dyn Fn() + Send + Sync + 'static>);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
        }
        LRESULT(0)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
