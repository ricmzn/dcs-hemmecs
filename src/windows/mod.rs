pub mod main_window;

use std::ffi::CString;
use std::ptr::null_mut as NULL;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    DispatchMessageA, GetFocus, MessageBoxA, PeekMessageA, TranslateMessage, IDOK, MB_ICONERROR,
    MB_ICONINFORMATION, MSG, PM_REMOVE,
};

/// Blocks execution of current thread while window is open and all worker threads are running
/// # Safety
/// `hwnd` must be a valid window handle, otherwise this results in undefined behavior
pub fn run_window_loop(hwnd: HWND, quit_signal: &AtomicBool) {
    // Run look while other threads are running
    while quit_signal.load(Relaxed) == false {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            // Process Windows event messages
            if PeekMessageA(&mut msg as *mut _, hwnd, 0, 0, PM_REMOVE) > 0 {
                TranslateMessage(&msg as *const _);
                DispatchMessageA(&msg as *const _);
            } else {
                // Notify other threads that the window has been closed
                quit_signal.store(true, Relaxed);
                break;
            }
        }
    }
}

pub enum MessageBoxType {
    Error(String),
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
