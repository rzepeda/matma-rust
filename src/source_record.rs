use std::collections::{BTreeMap, BTreeSet};
use serde::{Serialize, Deserialize};

/// Source regions owned by one named Rust type. Do not treat the whole
/// containing source file as context for that type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRegion {
    pub source_file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct SourceRecord {
    pub source_file: String,
    pub module_name: String,
    pub symbol: String,
    pub kind: String,
    /// File hash is for freshness only, never a type-change fingerprint.
    pub sha256: String,
    /// SHA-256 of normalized declaration and associated impl AST tokens.
    pub fingerprint: String,
    pub regions: Vec<SourceRegion>,
    pub implemented_traits: BTreeSet<String>,
    pub imports: BTreeMap<String, String>,
    pub references: BTreeSet<String>,
    pub diagnostics: Vec<String>,
}

impl SourceRecord {
    pub fn identity(&self) -> String {
        format!("{}::{}", self.module_name, self.symbol)
    }
}

/// One implementation block, which may reside outside its type declaration.
#[derive(Debug, Clone)]
pub struct ExternalImplementation {
    pub owner: String,
    pub source_file: String,
    pub module_name: String,
    pub imports: BTreeMap<String, String>,
    pub references: BTreeSet<String>,
    pub normalized: String,
    pub region: SourceRegion,
    pub contract: Option<String>,
}
