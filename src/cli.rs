use clap::Parser;
use crate::command::Command;

#[derive(Debug, Parser)]
#[command(name = "matma-indexer", version, about = "MATMA Rust testing architecture toolkit")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
