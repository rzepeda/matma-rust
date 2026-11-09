use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use heck::ToSnakeCase;
use sha2::{Digest, Sha256};
use syn::visit::{self, Visit};
use syn::{Item, UseTree};

use crate::source_record::SourceRecord;

pub struct SourceParser {
    references: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for SourceParser {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<_> = path.segments.iter().map(|p| p.ident.to_string()).collect();
        if !segments.is_empty() {
            self.references.insert(segments.join("::"));
        }
        visit::visit_path(self, path);
    }
}

impl SourceParser {
    pub fn parse(path: &Path, project_root: &Path, source_root: &Path) -> Result<Option<SourceRecord>> {
        let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        let content = std::str::from_utf8(&bytes).context("Rust source is not UTF-8")?;
        let file = syn::parse_file(content).with_context(|| format!("Rust syntax error in {}", path.display()))?;
        let mut types = Vec::new();
        let mut imports = BTreeMap::new();
        let mut diagnostics = Vec::new();
        for item in &file.items {
            match item {
                Item::Struct(x) => types.push((x.ident.to_string(), "struct")),
                Item::Enum(x) => types.push((x.ident.to_string(), "enum")),
                Item::Trait(x) => types.push((x.ident.to_string(), "trait")),
                Item::Union(x) => types.push((x.ident.to_string(), "union")),
                Item::Use(u) => {
                    let mut parts = Vec::new();
                    Self::imports(&u.tree, Vec::new(), &mut parts, &mut diagnostics);
                    for (alias, target) in parts { imports.insert(alias, target); }
                }
                Item::Macro(_) => diagnostics.push("macro expansion cannot be statically resolved".into()),
                Item::Mod(m) if m.content.is_some() => diagnostics.push(format!("inline module {} is not indexed as a separate target", m.ident)),
                _ => {}
            }
        }
        if types.len() > 1 {
            bail!("{} declares {} primary types; MATMA requires one", path.display(), types.len());
        }
        let Some((symbol, kind)) = types.into_iter().next() else { return Ok(None) };
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem != symbol.to_snake_case() {
            bail!("{} defines {symbol}, expected filename {}.rs", path.display(), symbol.to_snake_case());
        }
        let source_file = path.strip_prefix(project_root)?.to_string_lossy().replace('\\', "/");
        let relative = path.strip_prefix(source_root)?;
        let mut module_segments: Vec<_> = relative.components().map(|v| v.as_os_str().to_string_lossy().to_string()).collect();
        if let Some(last) = module_segments.last_mut() {
            *last = stem.into();
        }
        // src/thing/mod.rs is module glue, but if it contains a named type its
        // basename cannot coincide with that type, so it was rejected above.
        let module_name = format!("crate::{}", module_segments.join("::"));
        let mut visitor = Self { references: BTreeSet::new() };
        visitor.visit_file(&file);
        Ok(Some(SourceRecord {
            source_file,
            module_name,
            symbol,
            kind: kind.into(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            imports,
            references: visitor.references,
            diagnostics,
        }))
    }

    fn imports(tree: &UseTree, prefix: Vec<String>, out: &mut Vec<(String, String)>, diagnostics: &mut Vec<String>) {
        match tree {
            UseTree::Path(p) => {
                let mut next = prefix;
                next.push(p.ident.to_string());
                Self::imports(&p.tree, next, out, diagnostics);
            }
            UseTree::Name(n) => {
                let mut path = prefix;
                path.push(n.ident.to_string());
                let alias = n.ident.to_string();
                out.push((alias, path.join("::")));
            }
            UseTree::Rename(n) => {
                let mut path = prefix;
                path.push(n.ident.to_string());
                out.push((n.rename.to_string(), path.join("::")));
            }
            UseTree::Group(g) => {
                for item in &g.items { Self::imports(item, prefix.clone(), out, diagnostics); }
            }
            UseTree::Glob(_) => diagnostics.push("glob import has unresolved target symbols".into()),
        }
    }
}
