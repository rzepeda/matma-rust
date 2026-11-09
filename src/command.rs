use clap::Subcommand;
use crate::options::Options;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build deterministic source dependency manifests.
    Index {
        #[command(flatten)]
        options: Options,
        #[arg(long, short = 'S')]
        scaffold: bool,
    },
    /// Generate missing tests and centralized mocks, without overwriting authored files.
    Scaffold {
        #[command(flatten)]
        options: Options,
        #[arg(long, short = 'f')]
        target: Option<String>,
    },
    /// Audit one-primary-type-per-file and native naming conventions.
    Validate {
        #[command(flatten)]
        options: Options,
        #[arg(long, short = 'c')]
        check: bool,
    },
    /// Read persisted manifests and select tests; never execute tests.
    Impact {
        #[command(flatten)]
        options: Options,
        #[arg(long, short = 'f')]
        target: String,
        #[arg(long, short = 'l', default_value_t = 0)]
        level: usize,
    },
}
