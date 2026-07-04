//! CLI command definitions and dispatch.
//!
//! Doctor is a CLI-first tool. All diagnostic capabilities are
//! accessible through the command line. Uses clap for argument
//! parsing with derive macros.

pub mod diagnose;
pub mod explain;

use crate::error::DoctorResult;
use clap::{Parser, Subcommand};

/// Doctor — AI-native diagnostic engine for software systems.
#[derive(Parser)]
#[command(name = "doctor", version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a full diagnostic scan on a project
    Diagnose(diagnose::DiagnoseArgs),
    /// Generate AI explanations for a diagnostic report
    Explain(explain::ExplainArgs),
}

/// Dispatch CLI command to the appropriate handler.
pub fn run(args: Args) -> DoctorResult<()> {
    match args.command {
        Command::Diagnose(args) => diagnose::run(args),
        Command::Explain(args) => explain::run(args),
    }
}
