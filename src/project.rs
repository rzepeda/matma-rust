use std::collections::BTreeSet;
use std::env;
use std::path::{Component, Path, PathBuf};
use anyhow::{anyhow, bail, Context, Result};
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
            if let Some(record) = SourceParser::parse(entry.path(), &self.root, &self.source_root)? {
                if !names.insert(record.identity()) { return Err(anyhow!("duplicate identity: {}", record.identity())); }
                found.push(record);
            }
        }
        found.sort_by(|a,b| a.source_file.cmp(&b.source_file));
        Ok(found)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for item in WalkDir::new(&self.source_root).follow_links(false).sort_by_file_name() {
            match item {
                Ok(entry) => {
                    if entry.file_type().is_symlink() { errors.push(format!("symlink not supported: {}",entry.path().display())); }
                    else if entry.file_type().is_file() && entry.path().extension().and_then(|v| v.to_str())==Some("rs") {
                        if let Err(error) = SourceParser::parse(entry.path(), &self.root, &self.source_root) {
                            errors.push(format!("{error:#}"));
                        }
                    }
                }
                Err(error) => errors.push(format!("{error}")),
            }
        }
        errors
    }
}
