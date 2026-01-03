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

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

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

    // Read records from files or stdin
    let (mut records, had_trailing) = if config.input_files.is_empty() {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        input::read_all_records(reader, config.record_delimiter)?
    } else {
        let mut all_records = Vec::new();
        let mut last_had_trailing = true;
        for path in &config.input_files {
            let reader: Box<dyn BufRead> = if path == "-" {
                Box::new(BufReader::new(io::stdin().lock()))
            } else {
                Box::new(BufReader::new(File::open(path)?))
            };
            let (mut file_records, had_trailing) = input::read_all_records(reader, config.record_delimiter)?;
            all_records.append(&mut file_records);
            last_had_trailing = had_trailing;
        }
        (all_records, last_had_trailing)
    };

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
    output::write_records(&mut out, &records, &config, had_trailing)?;

    Ok(())
}
