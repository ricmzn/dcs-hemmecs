pub mod control_window;
pub mod hmd_window;
pub mod terrain_window;

use std::ffi::CString;
use std::ptr::null_mut as NULL;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{GetFocus, MessageBoxA, IDOK, MB_ICONERROR, MB_ICONINFORMATION};

/// Blocks execution of current thread while window is open and all worker threads are running
/// # Safety
/// `hwnd` must be a valid window handle, otherwise this results in undefined behavior
pub fn run_window_loop(_: HWND, quit_signal: &AtomicBool) {
    // Run look while other threads are running
    while quit_signal.load(Relaxed) == false {
        nwg::dispatch_thread_events();
        quit_signal.store(true, Relaxed);
    }
}

pub enum MessageBoxType {
    Error(String),
    #[allow(dead_code)]
    Info(String),
}

/// Shows a native Windows message box
pub fn show_message_box(msg_type: MessageBoxType) -> bool {
    unsafe {
        let (title, message, flags) = match msg_type {
            MessageBoxType::Error(msg) => (
                b"Error\0" as *const u8 as *const i8,
                CString::new(msg).unwrap(),
                MB_ICONERROR,
            ),
            MessageBoxType::Info(msg) => (
                b"Information\0" as *const u8 as *const i8,
                CString::new(msg).unwrap(),
                MB_ICONINFORMATION,
            ),
        };
        MessageBoxA(NULL(), message.as_ptr(), title, flags) == IDOK
    }
}

pub fn is_focused(hwnd: HWND) -> bool {
    unsafe { GetFocus() == hwnd }
}
