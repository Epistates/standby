//! CLI command definitions and execution handlers.
//!
//! This module defines the command-line interface for standby,
//! with subcommands for sleep, timeout, and wait operations.

pub mod sleep;
pub mod timeout;
pub mod wait;

use clap::{Parser, Subcommand};

/// Root CLI structure for the standby application.
#[derive(Parser)]
#[command(name = "standby")]
#[command(about = "A world-class cross-platform time management tool", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for standby.
#[derive(Subcommand)]
pub enum Commands {
    /// Suspend execution for a specified time
    Sleep(sleep::SleepArgs),

    /// Run a command with a time limit
    Timeout(timeout::TimeoutArgs),

    /// Wait for a process to complete (with optional timeout)
    Wait(wait::WaitArgs),
}

impl Cli {
    /// Execute the selected subcommand.
    pub fn execute(self) -> crate::Result<()> {
        match self.command {
            Commands::Sleep(args) => sleep::execute(args),
            Commands::Timeout(args) => timeout::execute(args),
            Commands::Wait(args) => wait::execute(args),
        }
    }
}
