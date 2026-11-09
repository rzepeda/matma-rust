use std::collections::{BTreeMap, BTreeSet};

/// One primary Rust struct/enum/trait and its source-owned static evidence.
#[derive(Debug, Clone)]
pub struct SourceRecord {
    pub source_file: String,
    pub module_name: String,
    pub symbol: String,
    pub kind: String,
    pub sha256: String,
    pub imports: BTreeMap<String, String>,
    pub references: BTreeSet<String>,
    pub diagnostics: Vec<String>,
}

impl SourceRecord {
    pub fn identity(&self) -> String {
        format!("{}::{}", self.module_name, self.symbol)
    }
}
