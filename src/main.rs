mod config;
mod consts;
mod data;
mod drawing;
mod window;
mod worker;

use crossbeam::scope;
use font_kit::handle::Handle;
use raqote::DrawTarget;
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;

use config::load_or_create_config;
use config::Config;
use consts::{COULD_NOT_CREATE_CONFIG, DEFAULT_FONT, FIRST_TIME_MESSAGE, HEIGHT, WIDTH};
use data::{FlightData, WindowData};
use window::{create_window, run_window_loop, show_message_box, MessageBoxType};
use worker::run_worker;

fn main() {
    // Pre-load the font embedded in the program
    let default_font = Handle::from_memory(Arc::new(DEFAULT_FONT.into()), 0)
        .load()
        .unwrap();

    // Use an atomic boolean to syncronize the quit flag across threads
    let quit_signal = AtomicBool::new(false);

    // Get the application configuration
    let config = match load_or_create_config() {
        Ok((config, false)) => config,
        Ok((config, true)) => {
            show_message_box(MessageBoxType::Info(FIRST_TIME_MESSAGE.into()));
            config
        }
        Err(err) if err.downcast_ref::<toml::de::Error>().is_some() => {
            show_message_box(MessageBoxType::Error(format!(
                "Found an error while loading config file:\n\n{}",
                err
            )));
            Config::default()
        }
        Err(err) => {
            eprintln!("Error details: {}", err);
            show_message_box(MessageBoxType::Error(COULD_NOT_CREATE_CONFIG.into()));
            Config::default()
        }
    };

    // Pin the data to make sure the pointer we use later (in window_proc) can't point to a dropped value
    let data = Box::pin(WindowData {
        flight_data: Arc::new(Mutex::new(FlightData::default())),
        draw_target: RefCell::new(DrawTarget::new(WIDTH, HEIGHT)),
        font: RefCell::new(default_font),
        config,
    });

    // Use crossbeam's thread scope feature to keep lifetimes tidy as the worker thread doesn't need to run beyond the main thread
    let thread_scope = scope(|scope| {
        // Create the worker thread
        let data_handle = Arc::clone(&data.flight_data);
        scope.spawn(|_| run_worker(data_handle, &quit_signal));

        // Create the window
        let window = create_window(&data);
        run_window_loop(window, &quit_signal);
    });

    thread_scope.expect("Error caught in worker thread");
}
