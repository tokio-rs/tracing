use std::error;

use clap::{Parser, Subcommand};

mod macro_tests;

use macro_tests::gen_macro_tests;

#[derive(Debug, Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate tests for `tracing` macros.
    ///
    /// This will be placed in the dedicated project `tracing/test-macros`.
    GenMacroTests,
}

impl Command {
    fn run(&self) -> Result<(), Box<dyn error::Error>> {
        match self {
            Self::GenMacroTests => gen_macro_tests(),
        }
    }
}

fn main() -> Result<(), Box<dyn error::Error>> {
    Args::parse().cmd.run()
}
