use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct Options {
    #[arg(long, short = 's', default_value = "src")]
    pub source_root: String,
    #[arg(long, short = 't', default_value = "tests")]
    pub tests_root: String,
    #[arg(long, short = 'o', default_value = "text", value_parser = ["text", "json", "yaml"])]
    pub format: String,
}
