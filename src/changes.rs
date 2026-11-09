use std::collections::{BTreeMap, BTreeSet};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use crate::project::Project;
use crate::store::Store;

/// Compare two persisted type inventories, then traverse the union of their
/// dependency graphs. Removed edges matter just as much as newly added edges.
pub struct Changes;

impl Changes {
    fn symbols(index: &Value) -> Result<BTreeMap<String, Value>> {
        let list = index["symbols"].as_array().ok_or_else(||anyhow!("invalid symbol index"))?;
        let mut out = BTreeMap::new();
        for row in list {
            let identity = row["identity"].as_str().ok_or_else(||anyhow!("missing symbol identity"))?;
            out.insert(identity.to_string(),row.clone());
        }
        Ok(out)
    }

    fn reverse(rows: &BTreeMap<String,Value>, edges: &mut BTreeMap<String,BTreeMap<String,usize>>) {
        for (owner, record) in rows {
            for edge in record["dependencies"].as_array().into_iter().flatten() {
                let Some(dependency) = edge["identity"].as_str() else { continue; };
                let weight = edge["weight"].as_u64().unwrap_or(1) as usize;
                let entry = edges.entry(dependency.to_string()).or_default()
                    .entry(owner.clone()).or_insert(weight);
                *entry = (*entry).min(weight);
            }
        }
    }

    pub fn select(project: &Project, level: usize) -> Result<Value> {
        let current = Store::read(project)?;
        let previous_path = project.tests_root.join("mirror/.previous.auto.yaml");
        if !previous_path.is_file() {
            bail!("no preceding type index available; index once before editing source, then index again after changes");
        }
        let previous = Store::read_yaml(&previous_path)?;
        if previous["schema_version"] != 3 { bail!("unsupported preceding MATMA index schema"); }
        let old = Self::symbols(&previous)?;
        let now = Self::symbols(&current)?;
        let all: BTreeSet<_> = old.keys().chain(now.keys()).cloned().collect();
        let mut changed = BTreeSet::new();
        let mut removed = BTreeSet::new();
        for identity in all {
            match (old.get(&identity),now.get(&identity)) {
                (Some(before),Some(after)) => {
                    if before["fingerprint"] != after["fingerprint"]
                        || before["dependencies"] != after["dependencies"] {
                        changed.insert(identity);
                    }
                }
                (Some(_),None) => { removed.insert(identity.clone()); changed.insert(identity); }
                (None,Some(_)) => { changed.insert(identity); }
                (None,None) => {}
            }
        }
        let mut reverse = BTreeMap::new();
        Self::reverse(&old,&mut reverse);
        Self::reverse(&now,&mut reverse);
        let mut distances = BTreeMap::new();
        let mut pending = BTreeSet::new();
        let mut settled = BTreeSet::new();
        for type_id in &changed {
            distances.insert(type_id.clone(),0usize);
            pending.insert((0usize,type_id.clone()));
        }
        while let Some((distance,identity)) = pending.iter().next().cloned() {
            pending.remove(&(distance,identity.clone()));
            if !settled.insert(identity.clone()) { continue; }
            if let Some(dependents) = reverse.get(&identity) {
                for (dependent,cost) in dependents {
                    if settled.contains(dependent) { continue; }
                    let next = distance.saturating_add(*cost);
                    if next < *distances.get(dependent).unwrap_or(&usize::MAX) {
                        if let Some(old_distance) = distances.insert(dependent.clone(),next) {
                            pending.remove(&(old_distance,dependent.clone()));
                        }
                        pending.insert((next,dependent.clone()));
                    }
                }
            }
        }
        let mut affected = BTreeSet::new();
        let mut units = BTreeSet::new();
        let mut interactions = BTreeSet::new();
        let mut missing = BTreeSet::new();
        for (identity,distance) in distances {
            if distance > level { continue; }
            affected.insert(identity.clone());
            let Some(entry) = now.get(&identity) else { continue; };
            let Some(path) = entry["manifest"].as_str() else { continue; };
            let manifest = Store::read_yaml(&project.root.join(path))?;
            if let Some(unit) = manifest["mirrors"]["unit_test"].as_str() {
                if project.root.join(unit).is_file() { units.insert(unit.to_string()); }
                else { missing.insert(unit.to_string()); }
            }
            if level >= 1 {
                if let Some(interaction) = manifest["mirrors"]["interaction_test"].as_str() {
                    if project.root.join(interaction).is_file() { interactions.insert(interaction.to_string()); }
                    else { missing.insert(interaction.to_string()); }
                }
            }
        }
        Ok(json!({
            "status":"ok","command":"changes","level":level,
            "changed_symbols":changed,"removed_symbols":removed,
            "affected_symbols":affected,"unit_tests":units,
            "interaction_tests":interactions,"missing_tests":missing,
            "note":"Minimum weighted paths across old and new graphs. MATMA selects tests, it does not execute them."
        }))
    }
}
