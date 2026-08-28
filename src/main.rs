use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use wuspotify_muter::{run, Config, ConfigError};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match Config::parse_from_args(args) {
        Ok(cfg) => cfg,
        Err(ConfigError::HelpRequested) => {
            println!("{}", wuspotify_muter::help_message());
            return;
        }
        Err(ConfigError::VersionRequested) => {
            println!("wuspotify-muter {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(1);
        }
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    if let Err(err) = ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    }) {
        eprintln!("Warning: Failed to set Ctrl+C handler ({err}).");
    }

    if let Err(err) = run(config, running) {
        eprintln!("Application error: {err}");
        process::exit(1);
    }
}
