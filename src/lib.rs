mod cli;
mod command;
mod graph;
mod impact;
mod options;
mod parser;
mod project;
mod scaffold;
mod source_record;
mod store;

use anyhow::{bail, Result};
use clap::Parser;
use cli::Cli;
use command::Command;
use graph::Graph;
use project::Project;
use scaffold::Scaffold;
use serde_json::{json, Value};
use store::Store;

pub fn run() -> Result<()> {
    execute(Cli::parse())
}

fn execute(cli: Cli) -> Result<()> {
    let (options, output) = match &cli.command {
        Command::Index { options, .. }
        | Command::Scaffold { options, .. }
        | Command::Validate { options, .. }
        | Command::Impact { options, .. } => (options, options.format.as_str()),
    };
    let project = Project::new(&options.source_root, &options.tests_root)?;
    let result = match cli.command {
        Command::Index { scaffold, .. } => {
            let files = project.scan()?;
            let graph = Graph::new(&files);
            Store::write(&project, &files, &graph)?;
            let created = if scaffold { Scaffold::create(&project, &files, None)? } else { Vec::new() };
            json!({"status":"ok","command":"index","indexed":files.len(),"created":created})
        }
        Command::Validate { check, .. } => {
            let failures = project.validate();
            let result = json!({"status":if failures.is_empty(){"ok"}else{"invalid"},"command":"validate","diagnostics":failures});
            print_result(output, &result)?;
            if check && !failures.is_empty() { bail!("structure validation failed"); }
            return Ok(());
        }
        Command::Scaffold { target, .. } => {
            let files = project.scan()?;
            let graph = Graph::new(&files);
            Store::write(&project, &files, &graph)?;
            let created = Scaffold::create(&project, &files, target.as_deref())?;
            json!({"status":"ok","command":"scaffold","created":created})
        }
        Command::Impact { target, level, .. } => impact::Impact::select(&project, &target, level)?,
    };
    print_result(output, &result)
}

fn print_result(format: &str, value: &Value) -> Result<()> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(value)?),
        "yaml" => print!("{}", serde_yaml::to_string(value)?),
        "text" => println!("{}", serde_json::to_string_pretty(value)?),
        other => bail!("unsupported format: {other}; expected text, json or yaml"),
    }
    Ok(())
}
