use crossbeam::scope;
use font_kit::font::Font;
use font_kit::handle::Handle;
use raqote::{AntialiasMode, BlendMode, DrawOptions, DrawTarget, Point, SolidSource, Source};
use std::cell::Cell;
use std::cell::RefCell;
use std::ffi::CString;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader};
use std::mem;
use std::net::TcpStream;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::ptr::null_mut as NULL;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;
use winapi::shared::windef::*;
use winapi::um::errhandlingapi::*;
use winapi::um::libloaderapi::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

#[derive(Debug, Clone, Default)]
struct FlightData {
    time: f32,
    ias: f32,
    mach: f32,
    alt: f32,
    rad_alt: f32,
    pitch: f32,
    roll: f32,
    yaw: f32,
    aoa: f32,
    g: (f32, f32, f32),
}

impl FlightData {
    fn from_line(line: &str) -> FlightData {
        let mut data = FlightData::default();
        line.split(",").for_each(|kv| {
            let mut kv = kv.split("=");
            if let Some(key) = kv.next() {
                if let Some(value) = kv.next() {
                    let value = value.parse().unwrap_or(f32::NAN);
                    match key {
                        "t" => data.time = value,
                        "ias" => data.ias = value,
                        "mach" => data.mach = value,
                        "alt" => data.alt = value,
                        "radalt" => data.rad_alt = value,
                        "pitch" => data.pitch = value,
                        "roll" => data.roll = value,
                        "yaw" => data.yaw = value,
                        "aoa" => data.aoa = value,
                        "g.x" => data.g.0 = value,
                        "g.y" => data.g.1 = value,
                        "g.z" => data.g.2 = value,
                        _ => (),
                    }
                }
            }
        });
        data
    }
}

struct WindowData {
    flight_data: Arc<Mutex<FlightData>>,
    draw_target: RefCell<DrawTarget>,
    font: RefCell<Font>,
}

type WindowDataRef = Pin<Box<WindowData>>;

const FONT: &[u8] = include_bytes!("../fonts/Inconsolata-SemiBold.ttf");

const WIDTH: i32 = 1024;
const HEIGHT: i32 = 768;

const COLORS: RGBQUAD = RGBQUAD {
    rgbRed: 0xff,
    rgbGreen: 0xff,
    rgbBlue: 0xff,
    rgbReserved: 0x00,
};

const BMP_INFO: BITMAPINFO = BITMAPINFO {
    bmiColors: [COLORS],
    bmiHeader: BITMAPINFOHEADER {
        biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
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
    let data = GetWindowLongPtrA(hwnd, GWL_USERDATA) as *const WindowDataRef;
    match msg {
        WM_NCCREATE => {
            // Save the passed Mutex<WindowData> pointer into the user data field of the window
            let data = *(lparam as *mut CREATESTRUCTA);
            SetWindowLongPtrA(hwnd, GWL_USERDATA, data.lpCreateParams as isize);
            1
        }
        WM_PAINT => {
            if let Some(data) = data.as_ref() {
                let mut ps: PAINTSTRUCT = mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps as *mut _);

                let fd = { data.flight_data.lock().unwrap().clone() };
                let mut dt = data.draw_target.borrow_mut();
                let font = data.font.borrow();

                dt.clear(SolidSource::from_unpremultiplied_argb(
                    0xff, 0x00, 0x00, 0x00,
                ));

                // Format text information
                let text = format!(
                    "{}\n{}\n\n\n\n\n\n\n\n{}\n{}\n{}",
                    format!("              {:0>3.0}         ", fd.yaw.to_degrees()),
                    format!(
                        "[{:>3.0}]                   [{:>5.0}]",
                        fd.ias * 1.943844, // m/s -> kn
                        fd.alt * 3.28084   // m -> ft
                    ),
                    format!("M {:.2}", fd.mach),
                    format!("G {:.1}", fd.g.1),
                    format!("a {:.1}", fd.aoa)
                );

                // Draw text on the canvas
                dt.draw_glyphs(
                    &font,
                    64.0,
                    &text
                        .chars()
                        .map(|c| font.glyph_for_char(c).unwrap_or_default())
                        .collect::<Vec<_>>(),
                    &text
                        .chars()
                        .map({
                            let x = Cell::new(0.0);
                            let y = Cell::new(64.0);
                            move |c| {
                                let p = Point::new(x.get(), y.get());
                                if c == '\n' {
                                    // One line = 64px
                                    x.replace(32.0);
                                    y.replace(y.get() + 64.0);
                                } else {
                                    // One char = 32px
                                    x.replace(x.get() + 32.0);
                                }
                                p
                            }
                        })
                        .collect::<Vec<_>>(),
                    &Source::Solid(SolidSource::from_unpremultiplied_argb(
                        0xff, 0x00, 0xff, 0x00,
                    )),
                    &DrawOptions {
                        blend_mode: BlendMode::Add,
                        antialias: AntialiasMode::None,
                        alpha: 1.0,
                    },
                );

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
                    dt.get_data_mut() as *mut _ as *mut _,
                    &BMP_INFO as *const _,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );

                EndPaint(hwnd, &mut ps as *mut _);
            }

            // Force next redraw as soon as possible
            InvalidateRect(hwnd, NULL(), 0);
            0
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            PostMessageA(hwnd, WM_CLOSE, 0, 0);
            0
        }
        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

fn reader_thread(data_handle: Arc<Mutex<FlightData>>, quit: &AtomicBool) {
    // Watch for panics in the worker thread
    if let Err(_) = catch_unwind(|| {
        println!("Waiting for mission start");
        while quit.load(Ordering::Relaxed) == false {
            match TcpStream::connect("127.0.0.1:28561") {
                Ok(stream) => {
                    stream.set_nodelay(true).unwrap();
                    println!("Connected to DCS");
                    let mut lines = BufReader::new(stream).lines();
                    while quit.load(Ordering::Relaxed) == false {
                        let line = lines.next();
                        if let Some(line) = line {
                            *data_handle.lock().unwrap() = FlightData::from_line(&line.unwrap());
                        } else {
                            *data_handle.lock().unwrap() = FlightData::default();
                            println!("DCS disconnected, waiting for mission restart");
                            break;
                        }
                    }
                }
                Err(err) if err.kind() == ErrorKind::ConnectionReset => {
                    println!(
                        "Warning: DCS disconnected suddenly, did something happen? (Check dcs.log)"
                    );
                    sleep(Duration::from_millis(500))
                }
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    sleep(Duration::from_millis(500))
                }
                Err(err) => panic!(err),
            }
        }
    }) {
        // Send the quit signal on any error
        quit.store(true, Ordering::Relaxed);
    }
}

fn main() {
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

    let screen = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };

    let font = Handle::from_memory(Arc::new(FONT.into()), 0)
        .load()
        .unwrap();

    let quit = AtomicBool::new(false);

    let data: WindowDataRef = Box::pin(WindowData {
        flight_data: Arc::new(Mutex::new(FlightData::default())),
        draw_target: RefCell::new(DrawTarget::new(WIDTH, HEIGHT)),
        font: RefCell::new(font),
    });

    scope(|s| {
        let fd = Arc::clone(&data.flight_data);
        s.spawn(|_| reader_thread(fd, &quit));
        unsafe {
            RegisterClassA(&window_class);
            let hwnd = CreateWindowExA(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_VISIBLE | WS_POPUP,
                screen.0 / 2 - WIDTH / 2,
                screen.1 / 2 - HEIGHT / 2 - screen.1 / 10,
                WIDTH,
                HEIGHT,
                NULL(),
                NULL(),
                instance,
                // &T -> *const T -> *mut c_void
                &data as *const _ as *mut _,
            );
            if hwnd.is_null() {
                let err = GetLastError();
                panic!("Could not create window - Error code: 0x{:08x}", err);
            }
            SetLayeredWindowAttributes(hwnd, 0, 128, LWA_ALPHA | LWA_COLORKEY);
            loop {
                if quit.load(Ordering::Relaxed) {
                    panic!("Quit requested from worker thread");
                }
                let mut msg: MSG = mem::zeroed();
                if PeekMessageA(&mut msg as *mut _, hwnd, 0, 0, PM_REMOVE) > 0 {
                    TranslateMessage(&msg as *const _);
                    DispatchMessageA(&msg as *const _);
                } else {
                    break;
                }
            }
        }
        quit.store(true, Ordering::Relaxed);
    })
    .unwrap();
}
