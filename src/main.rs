//! Doctor — AI-native diagnostic engine for software systems.

use clap::Parser;

fn main() {
    let args = doctor::cli::Args::parse();
    if let Err(e) = doctor::cli::run(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
