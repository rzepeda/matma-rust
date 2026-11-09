use std::fs;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use crate::project::Project;
use crate::store::Store;

/// Return only the declaration and implementation spans belonging to one type.
pub struct Context;

impl Context {
    pub fn read(project: &Project, target: &str) -> Result<Value> {
        let index = Store::read(project)?;
        let symbols = index["symbols"].as_array().ok_or_else(||anyhow!("invalid symbol inventory"))?;
        let matches: Vec<_> = symbols.iter().filter(|row| {
            ["identity","symbol","source_file"].iter().any(|key| row[*key].as_str() == Some(target))
        }).collect();
        if matches.len() != 1 {
            bail!("context target must identify one type, found {}",matches.len());
        }
        let selected = matches[0];
        let manifest_path = selected["manifest"].as_str().ok_or_else(||anyhow!("missing manifest path"))?;
        let manifest = Store::read_yaml(&project.root.join(manifest_path))?;
        if manifest["identity"] != selected["identity"] { bail!("corrupt MATMA context manifest"); }
        let mut regions = Vec::new();
        for region in manifest["source_regions"].as_array().into_iter().flatten() {
            let source_file = region["source_file"].as_str().ok_or_else(||anyhow!("invalid region source"))?;
            let start = region["start_line"].as_u64().unwrap_or(0) as usize;
            let end = region["end_line"].as_u64().unwrap_or(0) as usize;
            if start == 0 || end < start { bail!("invalid source region {source_file}:{start}-{end}"); }
            let content = fs::read_to_string(project.root.join(source_file))?;
            let lines: Vec<_> = content.lines().collect();
            if end > lines.len() { bail!("stale source region; reindex before reading context"); }
            regions.push(json!({
                "source_file":source_file,"start_line":start,"end_line":end,
                "kind":region["kind"],"snippet":lines[start-1..end].join("\n")
            }));
        }
        Ok(json!({
            "status":"ok","command":"context","identity":selected["identity"],
            "fingerprint":manifest["fingerprint"],"regions":regions,
            "dependencies":manifest["dependencies"],"implementors":manifest["implementors"],
            "diagnostics":manifest["diagnostics"]
        }))
    }
}
