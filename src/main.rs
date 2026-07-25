mod config;
mod defaults;
mod rules;
mod walker;

fn main() {
    println!("tmtidy {}", env!("CARGO_PKG_VERSION"));
}
