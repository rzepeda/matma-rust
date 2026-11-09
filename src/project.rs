use std::collections::BTreeSet;
use std::env;
use std::path::{Component, Path, PathBuf};
use anyhow::{anyhow, bail, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use crate::parser::SourceParser;
use crate::source_record::SourceRecord;

pub struct Project {
    pub root: PathBuf,
    pub source_root: PathBuf,
    pub tests_root: PathBuf,
}

impl Project {
    pub fn new(source_root: &str, tests_root: &str) -> Result<Self> {
        let root = env::current_dir()?;
        let source_root = Self::safe_path(&root, source_root)?;
        let tests_root = Self::safe_path(&root, tests_root)?;
        if !source_root.is_dir() { bail!("source root does not exist: {}", source_root.display()); }
        if source_root.starts_with(&tests_root) || tests_root.starts_with(&source_root) {
            bail!("source and tests roots must not overlap");
        }
        Ok(Self { root, source_root, tests_root })
    }

    fn safe_path(root: &Path, path: &str) -> Result<PathBuf> {
        let relative = Path::new(path);
        if relative.as_os_str().is_empty() || relative.is_absolute() || relative.components().any(|c| !matches!(c, Component::Normal(_))) {
            bail!("unsafe path {path:?}: use a relative path without dot or parent components");
        }
        let mut current = root.to_path_buf();
        for part in relative.components() {
            current.push(part.as_os_str());
            if current.is_symlink() { bail!("symlink directory not permitted: {}", current.display()); }
        }
        Ok(current)
    }

    pub fn relative(&self, path: &Path) -> Result<String> {
        Ok(path.strip_prefix(&self.root)?.to_string_lossy().replace('\\', "/"))
    }

    pub fn scan(&self) -> Result<Vec<SourceRecord>> {
        let mut found = Vec::new();
        let mut names = BTreeSet::new();
        for item in WalkDir::new(&self.source_root).follow_links(false).sort_by_file_name() {
            let entry = item?;
            if entry.file_type().is_symlink() { bail!("symlink in source tree: {}", entry.path().display()); }
            if !entry.file_type().is_file() || entry.path().extension().and_then(|s| s.to_str()) != Some("rs") { continue; }
            for record in SourceParser::parse(entry.path(), &self.root, &self.source_root)? {
                if !names.insert(record.identity()) { return Err(anyhow!("duplicate identity: {}", record.identity())); }
                found.push(record);
            }
        }
        found.sort_by_key(SourceRecord::identity);
        // A second AST pass binds out-of-file impls to declared types. This
        // includes files with only impl blocks and no primary declaration.
        for entry in WalkDir::new(&self.source_root).follow_links(false).sort_by_file_name() {
            let entry = entry?;
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|e|e.to_str()) != Some("rs") { continue; }
            for implementation in SourceParser::external_implementations(
                entry.path(), &self.root, &self.source_root
            )? {
                let target = if implementation.owner.starts_with("crate::") {
                    implementation.owner.clone()
                } else if let Some(imported) = implementation.imports.get(&implementation.owner) {
                    imported.clone()
                } else {
                    format!("{}::{}",implementation.module_name,implementation.owner)
                };
                let Some(record) = found.iter_mut().find(|r|r.identity() == target) else {
                    continue;
                };
                if record.regions.iter().any(|r|
                    r.source_file == implementation.source_file &&
                    r.start_line == implementation.region.start_line &&
                    r.kind == "impl") { continue; }
                record.fingerprint = format!("{:x}",Sha256::digest(
                    format!("{}\n{}",record.fingerprint,implementation.normalized).as_bytes()));
                record.regions.push(implementation.region);
                if let Some(contract) = implementation.contract {
                    record.implemented_traits.insert(
                        implementation.imports.get(&contract).cloned().unwrap_or(contract));
                }
                for reference in implementation.references {
                    // Expand explicit use aliases before transferring references
                    // to the type's canonical module identity.
                    let head = reference.split("::").next().unwrap_or("");
                    if let Some(imported) = implementation.imports.get(head) {
                        record.references.insert(reference.replacen(head,imported,1));
                    } else {
                        record.references.insert(reference);
                    }
                }
            }
        }
        Ok(found)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut identities = BTreeSet::new();
        for item in WalkDir::new(&self.source_root).follow_links(false).sort_by_file_name() {
            match item {
                Ok(entry) => {
                    if entry.file_type().is_symlink() { errors.push(format!("symlink not supported: {}",entry.path().display())); }
                    else if entry.file_type().is_file() && entry.path().extension().and_then(|v| v.to_str())==Some("rs") {
                        match SourceParser::parse(entry.path(), &self.root, &self.source_root) {
                            Ok(records) => for record in records {
                                if !identities.insert(record.identity()) {
                                    errors.push(format!("duplicate type identity: {}", record.identity()));
                                }
                            },
                            Err(error) => errors.push(format!("{error:#}")),
                        }
                    }
                }
                Err(error) => errors.push(format!("{error}")),
            }
        }
        errors
    }
}
