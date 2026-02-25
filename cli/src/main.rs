#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
//! Parsec CLI main entry point

mod args;
mod commands;
mod output;

use clap::Parser;
use anyhow::Result;
use tokio::runtime::Runtime;

use crate::args::Cli;
use crate::commands::handle_command;

fn main() -> Result<()> {
    #[cfg(all(windows, feature = "win-preload"))]
    {
        // Preload system comctl32 from System32 to ensure TaskDialogIndirect is available
        use std::path::PathBuf;
        if let Ok(system_root) = std::env::var("SystemRoot") {
            let path: PathBuf = [system_root, String::from("System32"), String::from("comctl32.dll")].iter().collect();
            if let Ok(lib) = unsafe { libloading::Library::new(path) } {
                Box::leak(Box::new(lib));
            }
        }
    }

    let cli = Cli::parse();

    // Setup runtime
    let rt = Runtime::new()?;

    // Handle command
    rt.block_on(async {
        match handle_command(cli.command).await {
            Ok(exit_code) => std::process::exit(exit_code),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    })
}