use std::ffi::CString;
use std::mem::zeroed;
use std::pin::Pin;
use std::ptr::null_mut as NULL;
use winapi::shared::windef::*;
use winapi::um::errhandlingapi::*;
use winapi::um::libloaderapi::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use crate::drawing::draw;
use crate::ApplicationState;

const REFRESH_TIMER: usize = 1;

static mut BMP_INFO: BITMAPINFO = BITMAPINFO {
    bmiColors: [RGBQUAD {
        rgbRed: 0xff,
        rgbGreen: 0xff,
        rgbBlue: 0xff,
        rgbReserved: 0x00,
    }],
    bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: -1,  // placeholder
        biHeight: -1, // placeholder
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
    let state = GetWindowLongPtrA(hwnd, GWL_USERDATA) as *const Pin<Box<ApplicationState>>;
    match msg {
        WM_NCCREATE => {
            // Save the passed Mutex<WindowData> pointer into the user data field of the window
            let data = *(lparam as *mut CREATESTRUCTA);
            SetWindowLongPtrA(hwnd, GWL_USERDATA, data.lpCreateParams as isize);
            1
        }
        WM_CREATE => window_proc(hwnd, WM_PAINT, 0, 0),
        WM_PAINT => {
            if let Some(state) = state.as_ref() {
                let mut ps: PAINTSTRUCT = zeroed();
                let window_hdc = BeginPaint(hwnd, &mut ps as *mut PAINTSTRUCT);

                // Unpack the data fields
                let mut draw_target = state.draw_target.borrow_mut();
                let flight_data = { state.flight_data.read().unwrap().clone() };
                let config = { state.config.lock().unwrap().clone() };
                let (width, height) = state.screen_dimensions;
                let font = state.font.borrow();

                // Set the image blit size
                // Note: the height is reversed because Raqote draws from the top left, but Windows draws from the bottom left
                BMP_INFO.bmiHeader.biWidth = width;
                BMP_INFO.bmiHeader.biHeight = -height;

                // Set up the drawing context for the transparent bitmap
                let target_hdc = CreateCompatibleDC(window_hdc);
                let target_bmp = CreateCompatibleBitmap(window_hdc, width, height);
                SelectObject(target_hdc, target_bmp as _);

                let mut point = POINT { x: 0, y: 0 };
                let mut size = SIZE {
                    cx: width,
                    cy: height,
                };
                let mut blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER,
                    BlendFlags: 0,
                    SourceConstantAlpha: config.appearance.brightness,
                    AlphaFormat: AC_SRC_ALPHA,
                };

                let pixels = draw(
                    &config,
                    &flight_data,
                    &mut state.radar_memory.write().unwrap(),
                    &mut draw_target,
                    state.screen_dimensions,
                    &font,
                );

                // Copy image data to the new bitmap
                SetDIBitsToDevice(
                    target_hdc,
                    0,
                    0,
                    width as u32,
                    height as u32,
                    0,
                    0,
                    0,
                    height as u32,
                    pixels.as_ptr() as *const _ as *mut _,
                    &mut BMP_INFO as *mut _,
                    DIB_RGB_COLORS,
                );

                // Set window to use the transparent image
                UpdateLayeredWindow(
                    hwnd,
                    NULL(),
                    NULL(),
                    &mut size as _,
                    target_hdc,
                    &mut point as _,
                    0,
                    &mut blend as _,
                    ULW_ALPHA,
                );

                // Restore old objects, clean up, and finish
                DeleteDC(target_hdc);
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
        WM_ACTIVATE => {
            if !GetParent(hwnd).is_null() {
                BringWindowToTop(GetParent(hwnd));
            }
            DefWindowProcA(hwnd, msg, wparam, lparam)
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

    let (screen_width, screen_height) = window_data.screen_dimensions;

    unsafe {
        RegisterClassA(&window_class);

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            screen_width,
            screen_height,
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
        hwnd
    }
}
