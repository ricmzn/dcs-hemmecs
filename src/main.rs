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
use std::sync::{Arc, RwLock};

use config::load_or_create_config;
use config::Config;
use consts::{COULD_NOT_CREATE_CONFIG, DEFAULT_FONT, FIRST_TIME_MESSAGE, HEIGHT, WIDTH};
use data::ApplicationState;
use window::{create_window, run_window_loop, show_message_box, MessageBoxType};
use worker::{run_config_worker, run_data_worker};

fn main() {
    // Pre-load the font embedded in the program
    let default_font = Handle::from_memory(Arc::new(DEFAULT_FONT.into()), 0)
        .load()
        .unwrap();

    // Use an atomic boolean to syncronize the quit flag across threads
    let quit_signal = AtomicBool::new(false);

    // Get the application configuration and its watcher + notifier combo
    // Note: we have to keep the watcher around even if we don't use it, or else it will be dropped and stop working
    let (config, _config_watcher, config_notifier) = match load_or_create_config() {
        Ok((config, watcher, notifier, false)) => (config, Some(watcher), Some(notifier)),
        Ok((config, watcher, notifier, true)) => {
            show_message_box(MessageBoxType::Info(FIRST_TIME_MESSAGE.into()));
            (config, Some(watcher), Some(notifier))
        }
        Err(err) if err.downcast_ref::<toml::de::Error>().is_some() => {
            show_message_box(MessageBoxType::Error(format!(
                "Error while loading config file:\n\n{}",
                err
            )));
            (Config::default(), None, None)
        }
        Err(err) => {
            eprintln!("Internal error while loading/saving config file: {:?}", err);
            show_message_box(MessageBoxType::Error(COULD_NOT_CREATE_CONFIG.into()));
            (Config::default(), None, None)
        }
    };

    // Pin the data to make sure the pointer we use later (in window_proc) can't point to a dropped value
    let state = Box::pin(ApplicationState {
        flight_data: RwLock::new(None),
        draw_target: RefCell::new(DrawTarget::new(WIDTH, HEIGHT)),
        font: RefCell::new(default_font),
        config: RwLock::new(config),
    });

    // Use crossbeam's thread scope feature to keep lifetimes tidy as the worker threads don't need to run beyond the main thread
    let config_handle = &state.config;
    let data_handle = &state.flight_data;
    let thread_scope = scope(|scope| {
        // Create the worker thread
        scope.spawn(|_| run_config_worker(config_handle, config_notifier, &quit_signal));
        scope.spawn(|_| run_data_worker(data_handle, &quit_signal));

        // Create the window
        let window = create_window(&state);
        run_window_loop(window, &quit_signal);
    });

    thread_scope.expect("Error caught in worker thread");
}
