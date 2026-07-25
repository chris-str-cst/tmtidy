mod config;
mod defaults;
mod exclude;
mod rules;
mod stats;
mod walker;

fn main() {
    println!("tmtidy {}", env!("CARGO_PKG_VERSION"));
}
