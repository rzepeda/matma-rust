use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use anyhow::{bail, Context, Result};
use heck::ToSnakeCase;
use serde_json::{json, Value};
use syn::Item;
use crate::project::Project;
use crate::source_record::SourceRecord;

pub struct Scaffold;
const BEGIN: &str = "// MATMA TEST BRIDGES BEGIN -- managed by matma-indexer";
const END: &str = "// MATMA TEST BRIDGES END";
const OLD_ENTRYPOINT: &str = "// MATMA GENERATED TEST ENTRYPOINT";

impl Scaffold {
    pub fn mirror(project: &Project, record: &SourceRecord) -> Value {
        let rel = Path::new(&record.source_file)
            .strip_prefix(project.source_root.strip_prefix(&project.root).unwrap()).unwrap();
        let stem = record.symbol.to_snake_case();
        let tests = project.tests_root.strip_prefix(&project.root).unwrap();
        let path = |tier: &str, name: String| {
            tests.join(tier).join("mirror").join(rel).join(name)
                .to_string_lossy().replace('\\', "/")
        };
        json!({
            "unit_test":path("unit",format!("test_{stem}.rs")),
            "interaction_test":path("interaction",format!("test_{stem}_interaction.rs")),
            "mock_file":path("mocks",format!("mock_{stem}.rs")),
            "mock_type":format!("Mock{}",record.symbol),
            "manifest_file":tests.join("mirror").join(rel).join(format!("{stem}.auto.yaml"))
                .to_string_lossy().replace('\\', "/"),
        })
    }

    fn ensure_safe(project: &Project, relative: &str) -> Result<PathBuf> {
        let output = project.root.join(relative);
        if !output.starts_with(&project.tests_root) {
            bail!("unsafe scaffold output: {relative}");
        }
        let mut dir = output.parent();
        while let Some(path) = dir {
            if path.is_symlink() {
                bail!("symlink output directory not allowed: {}",path.display());
            }
            if path == project.root { break; }
            dir = path.parent();
        }
        if output.is_symlink() {
            bail!("refusing to overwrite symlink: {}",output.display());
        }
        Ok(output)
    }

    fn create_missing(project: &Project, relative: &str, body: &str, created: &mut Vec<String>) -> Result<()> {
        let output = Self::ensure_safe(project, relative)?;
        if output.exists() { return Ok(()); }
        fs::create_dir_all(output.parent().unwrap())?;
        fs::write(&output, body).with_context(||format!("failed to create {relative}"))?;
        created.push(relative.into());
        Ok(())
    }

    /// Compute a path relative to a Rust source file's parent directory.
    /// #[path] is interpreted relative to the owning Rust module file, not
    /// relative to the package's current working directory.
    fn relative_path(from_dir: &Path, dest: &Path) -> Result<String> {
        let lhs: Vec<_> = from_dir.components().collect();
        let rhs: Vec<_> = dest.components().collect();
        let common = lhs.iter().zip(&rhs).take_while(|(a, b)| a == b).count();
        if common == 0 || lhs[common..].iter().any(|c| !matches!(c, Component::Normal(_))) {
            bail!("cannot create portable test bridge between {} and {}",from_dir.display(),dest.display());
        }
        let mut path = PathBuf::new();
        for _ in &lhs[common..] { path.push(".."); }
        for component in &rhs[common..] { path.push(component.as_os_str()); }
        Ok(path.to_string_lossy().replace('\\', "/"))
    }

    fn bridge_block(project: &Project, source: &str, records: &[&SourceRecord]) -> Result<String> {
        let source_path = project.root.join(source);
        let from_dir = source_path.parent().context("source file has no parent")?;
        let mut block = format!("{BEGIN}\n");
        for record in records {
            let stem = record.symbol.to_snake_case();
            let mirrors = Self::mirror(project, record);
            for (key, prefix, visibility) in [
                ("unit_test", "matma_unit", ""),
                ("interaction_test", "matma_interaction", ""),
                ("mock_file", "matma_mock", "pub(crate) "),
            ] {
                let target = mirrors[key].as_str().context("missing mirror path")?;
                if !project.root.join(target).is_file() { continue; }
                let relative = Self::relative_path(from_dir, &project.root.join(target))?;
                block.push_str(&format!("#[cfg(test)]\n#[path = \"{relative}\"]\n{visibility}mod {prefix}_{stem};\n"));
            }
        }
        block.push_str(&format!("{END}\n"));
        Ok(block)
    }

    fn update_bridge(project: &Project, source: &str, block: &str, changes: &mut Vec<String>) -> Result<()> {
        let path = project.root.join(source);
        if path.is_symlink() { bail!("refusing to change linked production file {}",path.display()); }
        let original = fs::read_to_string(&path)?;
        let result = if let Some(start) = original.find(BEGIN) {
            let Some(end_rel) = original[start..].find(END) else {
                bail!("unterminated managed MATMA bridge in {source}");
            };
            let after = start + end_rel + END.len();
            let trailer = &original[after..];
            if trailer.contains(BEGIN) || trailer.contains(END) || original[..start].contains(END) {
                bail!("conflicting MATMA bridge markers in {source}");
            }
            let mut result = original[..start].to_string();
            result.push_str(block);
            result.push_str(trailer.strip_prefix('\n').unwrap_or(trailer));
            result
        } else {
            if original.contains(END) { bail!("orphaned MATMA bridge marker in {source}"); }
            let mut result = original.clone();
            if !result.ends_with('\n') { result.push('\n'); }
            result.push('\n');
            result.push_str(block);
            result
        };
        if result != original {
            fs::write(&path, result).with_context(||format!("cannot update bridge for {source}"))?;
            changes.push(source.to_string());
        }
        Ok(())
    }

    /// Compare the actual managed bridge against the deterministic mirror
    /// inventory; report omissions and drift without rewriting source.
    pub fn bridge_diagnostics(project: &Project, records: &[SourceRecord]) -> Result<Vec<String>> {
        let mut groups: BTreeMap<String, Vec<&SourceRecord>> = BTreeMap::new();
        let mut diagnostics = Vec::new();
        for record in records {
            let mirrors = Self::mirror(project,record);
            if ["unit_test","interaction_test","mock_file"].iter().any(|key|
                mirrors[*key].as_str().is_some_and(|file| project.root.join(file).is_file())
            ) {
                groups.entry(record.source_file.clone()).or_default().push(record);
            }
        }
        for (source,mut group) in groups {
            group.sort_by_key(|r|r.identity());
            let expected = Self::bridge_block(project,&source,&group)?;
            let body = fs::read_to_string(project.root.join(&source))?;
            let ast = syn::parse_file(&body).with_context(||format!("cannot parse {source} during bridge validation"))?;
            for item in &ast.items {
                let Item::Mod(module) = item else { continue; };
                let Some((_,children)) = &module.content else { continue; };
                if children.iter().any(|item| matches!(item,Item::Fn(function)
                    if function.attrs.iter().any(|attribute|attribute.path().is_ident("test")))) {
                    diagnostics.push(format!("inline test module {} in {source} must be migrated to MATMA mirrored files",module.ident));
                }
            }
            let Some(start) = body.find(BEGIN) else {
                diagnostics.push(format!("missing MATMA test bridge in {source}"));
                continue;
            };
            let Some(end_rel) = body[start..].find(END) else {
                diagnostics.push(format!("unterminated MATMA test bridge in {source}"));
                continue;
            };
            let end = start + end_rel + END.len();
            if body[start..end] != *expected.trim_end() {
                diagnostics.push(format!("incorrect or stale MATMA test bridge in {source}"));
            }
            if body[end..].contains(BEGIN) {
                diagnostics.push(format!("duplicate MATMA bridge block in {source}"));
            }
            for record in group {
                let mirrors = Self::mirror(project,record);
                for key in ["unit_test","interaction_test"] {
                    let Some(path) = mirrors[key].as_str() else { continue; };
                    if !project.root.join(path).is_file() { continue; }
                    let content = fs::read_to_string(project.root.join(path))?;
                    if content.contains("MATMA: implement isolated unit tests") ||
                        content.contains("MATMA: implement exactly-two-real-object interaction tests") ||
                        content.contains("MATMA: author real-implementor contract tests") {
                        diagnostics.push(format!("unimplemented MATMA test placeholder: {path}"));
                    }
                }
            }
        }
        Ok(diagnostics)
    }

    pub fn create(project: &Project, records: &[SourceRecord], target: Option<&str>) -> Result<Vec<String>> {
        let chosen: Vec<_> = records.iter().filter(|r| target.map(|t| {
            t == r.source_file || t == r.symbol || t == r.identity() || r.source_file.ends_with(t)
        }).unwrap_or(true)).collect();
        if target.is_some() && chosen.len() != 1 {
            bail!("target must match exactly one indexed Rust type; found {}", chosen.len());
        }
        let mut changes = Vec::new();
        for record in &chosen {
            let mirrors = Self::mirror(project, record);
            let unit = mirrors["unit_test"].as_str().unwrap();
            let interaction = mirrors["interaction_test"].as_str().unwrap();
            let mock_file = mirrors["mock_file"].as_str().unwrap();
            let header = "// MATMA starter: replace these intentional failures with meaningful tests.\n";
            let is_trait = record.kind == "trait";
            let unit_body = if is_trait {
                format!("{header}// Contract test: verify the trait against real implementors.\n// Never instantiate a trait or substitute a mock for a real implementor.\n#[test]\nfn matma_contract_unimplemented() {{\n    panic!(\"MATMA: author real-implementor contract tests for {}\");\n}}\n",record.identity())
            } else {
                format!("{header}#[test]\nfn matma_unit_unimplemented() {{\n    panic!(\"MATMA: implement isolated unit tests for {}\");\n}}\n",record.identity())
            };
            Self::create_missing(project, unit, &unit_body, &mut changes)?;
            Self::create_missing(project, interaction,
                &format!("{header}#[test]\nfn matma_pairwise_unimplemented() {{\n    panic!(\"MATMA: implement exactly-two-real-object interaction tests for {}\");\n}}\n",record.identity()),
                &mut changes)?;
            // Trait contracts are not mock instances. A concrete type may be
            // substituted by a named double only behind an actual Rust seam.
            Self::create_missing(project, mock_file,
                &format!("{header}// Implement the required injected Rust trait(s) here, without cloning production logic.\npub struct Mock{};\n",record.symbol),
                &mut changes)?;
        }

        // Refresh the entire managed bridge for each affected source file,
        // preserving previously scaffolded types and developer-authored code.
        let mut groups: BTreeMap<String, Vec<&SourceRecord>> = BTreeMap::new();
        for record in records {
            let mirrors = Self::mirror(project, record);
            if ["unit_test","interaction_test","mock_file"].iter().any(|key| {
                mirrors[*key].as_str().is_some_and(|path| project.root.join(path).is_file())
            }) {
                groups.entry(record.source_file.clone()).or_default().push(record);
            }
        }
        for (source, mut group) in groups {
            group.sort_by_key(|record| record.identity());
            let block = Self::bridge_block(project,&source,&group)?;
            Self::update_bridge(project,&source,&block,&mut changes)?;
        }

        // The old external entrypoints would compile the same mirrored test
        // a second time, without private access. Remove only MATMA-owned ones.
        for name in ["matma_unit.rs","matma_interaction.rs"] {
            let path = project.tests_root.join(name);
            if !path.exists() { continue; }
            if fs::read_to_string(&path)?.starts_with(OLD_ENTRYPOINT) {
                fs::remove_file(&path)?;
                changes.push(format!("removed:{}",project.relative(&path)?));
            }
        }
        Ok(changes)
    }
}
