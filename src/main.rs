mod config;
mod decay;
mod defaults;
mod exclude;
mod logging;
mod rules;
mod stats;
mod walker;

fn main() {
    println!("tmtidy {}", env!("CARGO_PKG_VERSION"));
}
