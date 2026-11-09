use std::collections::BTreeSet;
use std::fs;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use crate::project::Project;
use crate::store::Store;

pub struct Impact;

impl Impact {
    pub fn select(project: &Project, target: &str, level: usize) -> Result<Value> {
        let index = Store::read(project)?;
        let symbols = index["symbols"].as_array().ok_or_else(||anyhow!("invalid symbol inventory"))?;
        let matches: Vec<_> = symbols.iter().filter(|r| {
            ["source_file", "symbol", "identity"].iter().any(|key| r[key].as_str() == Some(target))
                || r["source_file"].as_str().map(|f|f.ends_with(target)).unwrap_or(false)
        }).collect();
        if matches.len() != 1 { bail!("impact target must match exactly one indexed type (use a type or fully qualified identity when multiple share a file); found {}", matches.len()); }
        let selected = matches[0];
        let manifest_file = selected["manifest"].as_str().ok_or_else(||anyhow!("missing manifest pointer"))?;
        let manifest = Store::read_yaml(&project.root.join(manifest_file))?;
        if manifest["source_file"] != selected["source_file"] || manifest["identity"] != selected["identity"] { bail!("corrupt type manifest; run index"); }
        let mut unit = BTreeSet::new();
        let mut interaction = BTreeSet::new();
        let mut missing = BTreeSet::new();
        let mut affected = BTreeSet::new();
        let mut affected_symbols = BTreeSet::new();
        let origin = selected["source_file"].as_str().unwrap();
        affected.insert(origin.to_string());
        let origin_identity = selected["identity"].as_str().unwrap();
        affected_symbols.insert(origin_identity.to_string());
        Self::register(project,&manifest["mirrors"]["unit_test"], &mut unit,&mut missing);
        if level >= 1 { Self::register(project,&manifest["mirrors"]["interaction_test"],&mut interaction,&mut missing); }
        // Only visit levels saved in the manifest. Graph generation has
        // already settled every identity once, even when dependencies cycle.
        if let Some(groups) = manifest["dependents"].as_object() {
            for (key, dependents) in groups {
                let Some(distance) = key.strip_prefix("level_").and_then(|n| n.parse::<usize>().ok()) else { continue; };
                if distance > level { continue; }
                if let Some(dependents) = dependents.as_array() {
                    for dependent in dependents {
                        let Some(identity) = dependent["identity"].as_str() else { continue; };
                        if identity == origin_identity || !affected_symbols.insert(identity.into()) { continue; }
                        if let Some(path) = dependent["source_file"].as_str() { affected.insert(path.into()); }
                        Self::register(project,&dependent["unit_test"],&mut unit,&mut missing);
                        Self::register(project,&dependent["interaction_test"],&mut interaction,&mut missing);
                    }
                }
            }
        }
        let (business,regression,registry_missing) = Self::registrations(project,&affected,&affected_symbols)?;
        missing.extend(registry_missing);
        Ok(json!({"status":"ok","command":"impact","target":origin_identity,"source_file":origin,"level":level,
            "affected_sources":affected,"affected_symbols":affected_symbols,"unit_tests":unit,"interaction_tests":interaction,
            "business_tests":business,"regression_tests":regression,"missing_tests":missing,
            "note":"MATMA selects tests only; invoke cargo test explicitly. Registry tiers are not automatically executed."}))
    }

    fn register(project: &Project, path: &Value, selected: &mut BTreeSet<String>, missing: &mut BTreeSet<String>) {
        if let Some(path) = path.as_str() {
            if project.root.join(path).is_file() { selected.insert(path.into()); }
            else { missing.insert(path.into()); }
        }
    }

    fn registrations(project: &Project, affected: &BTreeSet<String>, affected_symbols: &BTreeSet<String>) -> Result<(BTreeSet<String>, BTreeSet<String>, BTreeSet<String>)> {
        let path = project.tests_root.join("matma-verification.yaml");
        let mut business = BTreeSet::new();
        let mut regression = BTreeSet::new();
        let mut missing = BTreeSet::new();
        if !path.exists() { return Ok((business,regression,missing)); }
        let root: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
        for row in root["entries"].as_array().into_iter().flatten() {
            if !row["source"].as_str().map(|s| affected.contains(s) || affected_symbols.contains(s)).unwrap_or(false) { continue; }
            for (name, paths) in [("business",&mut business),("regression",&mut regression)] {
                if let Some(group) = row[name].as_array() {
                    for value in group {
                        if let Some(file) = value.as_str() {
                            if project.root.join(file).is_file() { paths.insert(file.into()); }
                            else { missing.insert(file.into()); }
                        }
                    }
                }
            }
        }
        Ok((business,regression,missing))
    }
}
