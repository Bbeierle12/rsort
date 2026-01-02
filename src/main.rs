mod arena;
mod cli;
mod compare;
mod config;
mod debug;
mod error;
mod input;
mod key;
mod output;
mod sort;

use std::io::{self, BufReader, Write};

use clap::Parser;

use cli::Args;
use config::Config;
use error::Result;

/// Set up SIGPIPE handling for Unix systems
/// This prevents "broken pipe" errors when output is piped to commands like `head`
#[cfg(unix)]
fn setup_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn setup_sigpipe() {
    // Windows doesn't have SIGPIPE
}

fn main() {
    setup_sigpipe();

    if let Err(e) = run() {
        eprintln!("rsort: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let config = Config::from_args(&args)?;

    // Read all records from stdin
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    let mut records = input::read_all_records(reader, config.record_delimiter)?;

    // Debug output: show key spans before sorting
    if config.debug {
        let stderr = io::stderr();
        let mut stderr = stderr.lock();
        debug::debug_input(&mut stderr, &records, &config)?;
        stderr.flush()?;
    }

    // Sort records
    sort::sort_records(&mut records, &config);

    // Write output
    let mut out = output::open_output(&config)?;
    output::write_records(&mut out, &records, &config)?;

    Ok(())
}
