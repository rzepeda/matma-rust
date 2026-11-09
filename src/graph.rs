use std::collections::BTreeMap;
use serde_json::{json, Value};
use crate::source_record::SourceRecord;

/// Static directed dependency graph with weighted (usage=1, import-only=2) edges.
pub struct Graph {
    edges: BTreeMap<String, BTreeMap<String, usize>>,
}

impl Graph {
    pub fn new(files: &[SourceRecord]) -> Self {
        let identities: BTreeMap<_, _> = files.iter().map(|r| (r.identity(), r.source_file.clone())).collect();
        let mut edges = BTreeMap::new();
        for source in files {
            let mut targets: BTreeMap<String, usize> = BTreeMap::new();
            for (alias, target) in &source.imports {
                let absolute = Self::absolute(&source.module_name, target);
                if let Some(file) = Self::resolve(&absolute, &identities) {
                    if file != source.source_file {
                        let used = source.references.iter().any(|r| r == alias || r.starts_with(&format!("{alias}::")));
                        let weight = if used { 1 } else { 2 };
                        let edge = targets.entry(file).or_insert(weight);
                        *edge = (*edge).min(weight);
                    }
                }
            }
            for reference in &source.references {
                if !(reference.starts_with("crate::") || reference.starts_with("self::") || reference.starts_with("super::")) { continue; }
                let absolute = Self::absolute(&source.module_name, reference);
                if let Some(file) = Self::resolve(&absolute, &identities) {
                    if file != source.source_file { targets.insert(file, 1); }
                }
            }
            edges.insert(source.source_file.clone(), targets);
        }
        Self { edges }
    }

    fn absolute(module: &str, name: &str) -> String {
        if name.starts_with("crate::") { return name.into(); }
        let mut path: Vec<&str> = module.split("::").collect();
        let mut rest = name;
        if rest.starts_with("self::") {
            rest = &rest[6..];
        } else if rest.starts_with("super::") {
            while rest.starts_with("super::") {
                if path.len() > 1 { path.pop(); }
                rest = &rest[7..];
            }
        } else {
            // An unqualified `use foo::Bar` is not necessarily crate-local in
            // Rust 2018; leave it unresolved rather than inventing a match.
            return name.into();
        }
        format!("{}::{rest}", path.join("::"))
    }

    fn resolve(path: &str, identities: &BTreeMap<String, String>) -> Option<String> {
        identities.iter().filter(|(identity, _)| path == *identity || path.starts_with(&format!("{identity}::")))
            .max_by_key(|(identity, _)| identity.len()).map(|(_, file)| file.clone())
    }

    pub fn dependencies(&self, source: &str, records: &[SourceRecord]) -> Vec<Value> {
        self.edges.get(source).into_iter().flat_map(|v| v.iter()).filter_map(|(file, weight)| {
            records.iter().find(|r| &r.source_file == file).map(|record| json!({
                "symbol_name":record.symbol,"source_file":file,
                "weight":weight,"evidence":if *weight == 1 {"usage"} else {"import_only"}
            }))
        }).collect()
    }

    /// Minimum cumulative reverse-edge weight from changed source to dependents.
    pub fn dependents(&self, target: &str) -> BTreeMap<String, usize> {
        let mut distance = BTreeMap::new();
        distance.insert(target.to_string(), 0usize);
        loop {
            let mut changed = false;
            for (dependent, downstream) in &self.edges {
                for (dependency, weight) in downstream {
                    if let Some(parent) = distance.get(dependency).copied() {
                        let next = parent.saturating_add(*weight);
                        if next < *distance.get(dependent).unwrap_or(&usize::MAX) {
                            distance.insert(dependent.clone(), next);
                            changed = true;
                        }
                    }
                }
            }
            if !changed { break; }
        }
        distance.remove(target);
        distance
    }
}
