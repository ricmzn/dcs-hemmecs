use anyhow::{anyhow, Result};
use notify::DebouncedEvent;
use std::io::{self, BufRead, BufReader, ErrorKind};
use std::net::TcpStream;
use std::panic::{catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::RwLock;
use std::thread::sleep;
use std::time::Duration;

use crate::config::{read_existing_config, Config};
use crate::data::FlightData;

fn handle_data_connection(
    stream: TcpStream,
    data_handle: &RwLock<Option<FlightData>>,
    quit_signal: &AtomicBool,
) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut lines = BufReader::new(stream).lines();
    while quit_signal.load(Relaxed) == false {
        let line = lines.next();
        let mut data = data_handle.write().unwrap();
        if let Some(line) = line {
            *data = Some(serde_json::from_str(&line?).unwrap());
        } else {
            *data = None;
            break;
        }
    }
    // Connection closed normally
    println!("DCS disconnected, waiting for mission restart");
    Ok(())
}

pub fn run_data_worker(data_handle: &RwLock<Option<FlightData>>, quit_signal: &AtomicBool) {
    // Run thread while looking for possible panics
    if let Err(err) = catch_unwind(|| {
        println!("Waiting for mission start");
        while quit_signal.load(Relaxed) == false {
            match TcpStream::connect("127.0.0.1:28561") {
                // Connected to DCS
                Ok(stream) => {
                    println!("Connected to DCS");
                    match handle_data_connection(stream, data_handle, quit_signal) {
                        Err(_) => println!("Warning: DCS disconnected suddenly"),
                        _ => (),
                    }
                }
                // DCS closed connection
                Err(err)
                    if err.kind() == ErrorKind::ConnectionAborted
                        || err.kind() == ErrorKind::ConnectionAborted =>
                {
                    println!(
                        "Warning: DCS disconnected suddenly, did something happen? (Check dcs.log)"
                    );
                    sleep(Duration::from_millis(500))
                }
                // The export script is not running yet
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => (),
                // Unexpected error
                Err(err) => panic!(err),
            }
            // Wait a bit before trying to connect again
            sleep(Duration::from_millis(500));
        }
    }) {
        // Send the quit signal to the main thread
        quit_signal.store(true, Relaxed);
        // Finish unwinding the worker thread
        resume_unwind(err);
    }
}

fn try_config_reload(
    config: &RwLock<Config>,
    notifier: &Receiver<notify::DebouncedEvent>,
) -> Result<()> {
    match notifier.recv_timeout(Duration::from_millis(500))? {
        DebouncedEvent::Write(_) => {
            println!("Updating config");
            let mut write_lock = config
                .write()
                .map_err(|err| anyhow!("Unable to update config: {:?}", err))?;
            *write_lock = read_existing_config()?;
        }
        _ => (),
    };
    Ok(())
}

pub fn run_config_worker(
    config: &RwLock<Config>,
    notifier: Option<Receiver<notify::DebouncedEvent>>,
    quit_signal: &AtomicBool,
) {
    if let Some(notifier) = notifier {
        while quit_signal.load(Relaxed) == false {
            if let Err(err) = try_config_reload(&config, &notifier) {
                if err.downcast_ref::<RecvTimeoutError>().is_none() {
                    eprintln!(
                        "Error while receiving config change notification: {:?}",
                        err
                    );
                    eprintln!("Configuration will no longer be reloaded automatically!");
                    break;
                }
            }
        }
    }
}
