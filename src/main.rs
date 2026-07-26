mod cli;
mod config;
mod decay;
mod defaults;
mod exclude;
mod logging;
mod paths;
mod rules;
mod schedule;
mod stats;
mod walker;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}
