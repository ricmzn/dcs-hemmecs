use crate::data::FlightData;
use std::io::{BufRead, BufReader, ErrorKind};
use std::net::TcpStream;
use std::panic::{catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub fn run_worker(data_handle: Arc<Mutex<FlightData>>, quit_signal: &AtomicBool) {
    // Run thread while looking for possible panics
    if let Err(err) = catch_unwind(|| {
        println!("Waiting for mission start");
        while quit_signal.load(Ordering::Relaxed) == false {
            match TcpStream::connect("127.0.0.1:28561") {
                // Connected to DCS
                Ok(stream) => {
                    stream.set_nodelay(true).unwrap();
                    println!("Connected to DCS");
                    let mut lines = BufReader::new(stream).lines();
                    while quit_signal.load(Ordering::Relaxed) == false {
                        let line = lines.next();
                        let mut data = data_handle.lock().unwrap();
                        if let Some(line) = line {
                            *data = serde_json::from_str(&line.unwrap()).unwrap();
                        } else {
                            *data = FlightData::default();
                            println!("DCS disconnected, waiting for mission restart");
                            break;
                        }
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
                // DCS is not running yet
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    sleep(Duration::from_millis(500))
                }
                // Unexpected error
                Err(err) => panic!(err),
            }
        }
    }) {
        // Send the quit signal to the main thread
        quit_signal.store(true, Ordering::Relaxed);
        // Finish unwinding the worker thread
        resume_unwind(err);
    }
}
