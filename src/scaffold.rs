use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use crate::project::Project;
use crate::source_record::SourceRecord;

pub struct Scaffold;

impl Scaffold {
    pub fn mirror(project: &Project, record: &SourceRecord) -> Value {
        let rel = Path::new(&record.source_file).strip_prefix(project.source_root.strip_prefix(&project.root).unwrap()).unwrap();
        let stem = rel.file_stem().unwrap().to_string_lossy();
        let parent = rel.parent().unwrap_or(Path::new(""));
        let tests = project.tests_root.strip_prefix(&project.root).unwrap();
        let path = |tier: &str, name: String| {
            tests.join(tier).join("mirror").join(parent).join(name).to_string_lossy().replace('\\', "/")
        };
        json!({
            "unit_test":path("unit",format!("test_{stem}.rs")),
            "interaction_test":path("interaction",format!("test_{stem}_interaction.rs")),
            "mock_file":path("mocks",format!("mock_{stem}.rs")),
            "mock_type":format!("Mock{}",record.symbol),
            "manifest_file":tests.join("mirror").join(parent).join(format!("{stem}.auto.yaml")).to_string_lossy().replace('\\', "/"),
        })
    }

    fn ensure_safe(project: &Project, relative: &str) -> Result<PathBuf> {
        let output = project.root.join(relative);
        if !output.starts_with(&project.tests_root) { bail!("unsafe scaffold output: {relative}"); }
        let mut dir = output.parent();
        while let Some(path) = dir {
            if path.is_symlink() { bail!("symlink output directory not allowed: {}",path.display()); }
            if path == project.root { break; }
            dir = path.parent();
        }
        if output.is_symlink() { bail!("refusing to overwrite symlink: {}",output.display()); }
        Ok(output)
    }

    fn create_missing(project: &Project, relative: &str, content: &str, created: &mut Vec<String>) -> Result<()> {
        let output = Self::ensure_safe(project, relative)?;
        if output.exists() { return Ok(()); }
        fs::create_dir_all(output.parent().unwrap())?;
        fs::write(&output, content).with_context(||format!("failed to create {relative}"))?;
        created.push(relative.into());
        Ok(())
    }

    pub fn create(project: &Project, records: &[SourceRecord], target: Option<&str>) -> Result<Vec<String>> {
        let matches: Vec<_> = records.iter().filter(|r| target.map(|t| {
            t == r.source_file || t == r.symbol || t == r.identity() || r.source_file.ends_with(t)
        }).unwrap_or(true)).collect();
        if target.is_some() && matches.len() != 1 { bail!("target must match exactly one indexed Rust type; found {}", matches.len()); }
        let mut created = Vec::new();
        for record in matches {
            let mirrors = Self::mirror(project, record);
            let unit = mirrors["unit_test"].as_str().unwrap();
            let interaction = mirrors["interaction_test"].as_str().unwrap();
            let mock_file = mirrors["mock_file"].as_str().unwrap();
            let header = "// MATMA starter: write meaningful assertions before using as verification.\n";
            Self::create_missing(project, unit, &format!("{header}#[test]\nfn matma_unit_unimplemented() {{\n    panic!(\"MATMA: implement isolated unit tests for {}\");\n}}\n",record.identity()), &mut created)?;
            Self::create_missing(project, interaction, &format!("{header}#[test]\nfn matma_pairwise_unimplemented() {{\n    panic!(\"MATMA: implement exactly-two-real-object interaction tests for {}\");\n}}\n",record.identity()), &mut created)?;
            Self::create_missing(project, mock_file, &format!("{header}// Implement the injected Rust trait boundary explicitly; do not duplicate real behavior.\npub struct Mock{};\n",record.symbol), &mut created)?;
        }
        for (tier, filename) in [("unit","matma_unit.rs"),("interaction","matma_interaction.rs")] {
            let mut body = String::from("// MATMA GENERATED TEST ENTRYPOINT -- safe to regenerate\n");
            for record in records {
                let relative = Self::mirror(project, record)[format!("{tier}_test")].as_str().unwrap().to_string();
                if !project.root.join(&relative).is_file() { continue; }
                let subpath = Path::new(&relative).strip_prefix(project.tests_root.strip_prefix(&project.root).unwrap())?;
                let id: String = subpath.to_string_lossy().chars().map(|c| if matches!(c, '/' | '\\' | '.' | '-') { '_' } else { c }).collect();
                body.push_str(&format!("#[path = \"{}\"]\nmod {id};\n",subpath.to_string_lossy().replace('\\', "/")));
            }
            let relative = project.relative(&project.tests_root.join(filename))?;
            let output = Self::ensure_safe(project, &relative)?;
            if output.exists() {
                let old = fs::read_to_string(&output)?;
                if !old.starts_with("// MATMA GENERATED TEST ENTRYPOINT") { bail!("refusing to overwrite authored entrypoint {relative}"); }
                if old != body { fs::write(&output, &body)?; created.push(relative); }
            } else {
                Self::create_missing(project, &relative, &body, &mut created)?;
            }
        }
        Ok(created)
    }
}
