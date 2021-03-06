pub mod control_window;
pub mod hmd_window;

use std::ffi::CString;
use std::ptr::null_mut as NULL;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use winapi::um::winuser::{MessageBoxA, IDOK, MB_ICONERROR, MB_ICONINFORMATION};
use winapi::{
    shared::windef::HWND,
    um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
};

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

/// Returns the dimensions of the primary display in the format: (width, height)
pub fn get_screen_dimensions() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}
