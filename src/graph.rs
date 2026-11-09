use std::collections::{BTreeMap, BTreeSet};
use serde_json::{json, Value};
use crate::source_record::SourceRecord;

/// Type-to-type dependencies, not file-to-file dependencies.
/// Usage has cost 1; import-only evidence has cost 2.
pub struct Graph {
    edges: BTreeMap<String, BTreeMap<String, usize>>,
}

impl Graph {
    pub fn new(records: &[SourceRecord]) -> Self {
        let identities: BTreeSet<String> = records.iter().map(SourceRecord::identity).collect();
        let mut edges = BTreeMap::new();
        for source in records {
            let identity = source.identity();
            let mut targets: BTreeMap<String, usize> = BTreeMap::new();
            for (alias, target) in &source.imports {
                let absolute = Self::absolute(&source.module_name, target);
                if let Some(resolved) = Self::resolve(&absolute, &identities) {
                    if resolved != identity {
                        let used = source.references.iter().any(|reference| {
                            reference == alias || reference.starts_with(&format!("{alias}::"))
                        });
                        let weight = if used { 1 } else { 2 };
                        let entry = targets.entry(resolved).or_insert(weight);
                        *entry = (*entry).min(weight);
                    }
                }
            }
            for reference in &source.references {
                let absolute = if reference.starts_with("crate::")
                    || reference.starts_with("self::") || reference.starts_with("super::") {
                    Self::absolute(&source.module_name, reference)
                } else {
                    // Bare names may refer to a sibling type in this very module.
                    // Resolve only against indexed type identities; never guess an
                    // arbitrary external crate dependency.
                    format!("{}::{reference}", source.module_name)
                };
                if let Some(resolved) = Self::resolve(&absolute, &identities) {
                    if resolved != identity {
                        targets.insert(resolved, 1);
                    }
                }
            }
            edges.insert(identity, targets);
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
            return name.into();
        }
        format!("{}::{rest}", path.join("::"))
    }

    fn resolve(path: &str, identities: &BTreeSet<String>) -> Option<String> {
        identities.iter()
            .filter(|identity| path == identity.as_str() || path.starts_with(&format!("{identity}::")))
            .max_by_key(|identity| identity.len()).cloned()
    }

    pub fn dependencies(&self, source_identity: &str, records: &[SourceRecord]) -> Vec<Value> {
        self.edges.get(source_identity).into_iter().flat_map(|targets| targets.iter()).filter_map(|(identity, weight)| {
            records.iter().find(|record| &record.identity() == identity).map(|record| json!({
                "symbol_name":record.symbol, "identity":identity, "source_file":record.source_file,
                "weight":weight, "evidence":if *weight == 1 {"usage"} else {"import_only"}
            }))
        }).collect()
    }

    /// Shortest reverse-dependency radius using a settled/visited set.
    /// The origin is visited first and is never returned as a dependent,
    /// even in A -> B -> A and longer cyclic dependency graphs.
    pub fn dependents(&self, target: &str) -> BTreeMap<String, usize> {
        let mut reverse: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
        for (dependent, dependencies) in &self.edges {
            for (dependency, cost) in dependencies {
                reverse.entry(dependency.clone()).or_default().push((dependent.clone(), *cost));
            }
        }
        let mut distances = BTreeMap::new();
        let mut pending = BTreeSet::new();
        let mut settled = BTreeSet::new();
        distances.insert(target.to_string(), 0usize);
        pending.insert((0usize, target.to_string()));
        while let Some((distance, identity)) = pending.iter().next().cloned() {
            pending.remove(&(distance, identity.clone()));
            if !settled.insert(identity.clone()) { continue; }
            if let Some(neighbors) = reverse.get(&identity) {
                for (next, weight) in neighbors {
                    if settled.contains(next) { continue; }
                    let candidate = distance.saturating_add(*weight);
                    if candidate < *distances.get(next).unwrap_or(&usize::MAX) {
                        if let Some(old) = distances.insert(next.clone(), candidate) {
                            pending.remove(&(old, next.clone()));
                        }
                        pending.insert((candidate, next.clone()));
                    }
                }
            }
        }
        distances.remove(target);
        distances
    }
}
