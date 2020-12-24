use std::ffi::CString;
use std::pin::Pin;
use std::ptr::null_mut as NULL;
use winapi::shared::windef::*;
use winapi::um::errhandlingapi::*;
use winapi::um::libloaderapi::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use crate::consts::{HEIGHT, WIDTH};
use crate::drawing::draw;
use crate::ApplicationState;

const REFRESH_TIMER: usize = 1;

const BMP_INFO: BITMAPINFO = BITMAPINFO {
    bmiColors: [RGBQUAD {
        rgbRed: 0xff,
        rgbGreen: 0xff,
        rgbBlue: 0xff,
        rgbReserved: 0x00,
    }],
    bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: WIDTH,
        biHeight: -HEIGHT,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    },
};

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    let data = GetWindowLongPtrA(hwnd, GWL_USERDATA) as *const Pin<Box<ApplicationState>>;
    match msg {
        WM_NCCREATE => {
            // Save the passed Mutex<WindowData> pointer into the user data field of the window
            let data = *(lparam as *mut CREATESTRUCTA);
            SetWindowLongPtrA(hwnd, GWL_USERDATA, data.lpCreateParams as isize);
            1
        }
        WM_PAINT => {
            if let Some(data) = data.as_ref() {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps as *mut PAINTSTRUCT);

                // Unpack the data fields
                let mut draw_target = data.draw_target.borrow_mut();
                let flight_data = { data.flight_data.read().unwrap().clone() };
                let config = { data.config.lock().unwrap().clone() };
                let font = data.font.borrow();

                // Copy image data to window
                StretchDIBits(
                    hdc,
                    0,
                    0,
                    WIDTH,
                    HEIGHT,
                    0,
                    0,
                    WIDTH,
                    HEIGHT,
                    draw(hwnd, &config, &flight_data, &mut draw_target, &font) as *const [u32]
                        as *mut _,
                    &BMP_INFO as *const BITMAPINFO,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );

                SetLayeredWindowAttributes(
                    hwnd,
                    0,
                    config.appearance.brightness,
                    LWA_ALPHA | LWA_COLORKEY,
                );
                EndPaint(hwnd, &mut ps as *mut PAINTSTRUCT);
            }

            // Force next redraw as soon as possible
            unsafe extern "system" fn refresh(hwnd: HWND, _: u32, _: usize, _: u32) {
                InvalidateRect(hwnd, NULL(), 0);
            }
            SetTimer(hwnd, REFRESH_TIMER, 10, Some(refresh));

            // Return zero to signal the message was handled
            0
        }
        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

pub fn create(window_data: &Pin<Box<ApplicationState>>, parent: HWND) -> HWND {
    let instance = unsafe { GetModuleHandleA(NULL()) };
    let class_name = CString::new("HMDWindow").unwrap();
    let title = CString::new("HMD").unwrap();

    let window_class = WNDCLASSA {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class_name.as_ptr(),
        cbClsExtra: 0,
        hbrBackground: NULL(),
        cbWndExtra: 0,
        hCursor: NULL(),
        hIcon: NULL(),
        lpszMenuName: NULL(),
        style: CS_HREDRAW | CS_VREDRAW,
    };

    unsafe {
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        RegisterClassA(&window_class);

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_VISIBLE | WS_CHILD | WS_POPUP,
            screen_width / 2 - WIDTH / 2,
            screen_height / 2 - HEIGHT / 2 - screen_height / 10,
            WIDTH,
            HEIGHT,
            parent as *const _ as *mut _,
            NULL(),
            instance,
            // &T -> *const T -> *mut c_void
            window_data as *const _ as *mut _,
        );
        if hwnd.is_null() {
            let err = GetLastError();
            panic!("Could not create window - Error code: 0x{:08x}", err);
        }
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA | LWA_COLORKEY);
        hwnd
    }
}