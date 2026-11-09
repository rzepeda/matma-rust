use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
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
    /// One record per named type, even when several types share a source file.
    /// A type owns references within its declaration and associated impls.
    pub fn parse(path: &Path, project_root: &Path, source_root: &Path) -> Result<Vec<SourceRecord>> {
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
        let source_file = path.strip_prefix(project_root)?.to_string_lossy().replace('\\', "/");
        let relative = path.strip_prefix(source_root)?;
        let mut module_segments: Vec<_> = relative.components().map(|v| v.as_os_str().to_string_lossy().to_string()).collect();
        if let Some(last) = module_segments.last_mut() {
            *last = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").into();
        }
        let module_name = format!("crate::{}", module_segments.join("::"));
        let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let has_shared_imports = types.len() > 1 && !imports.is_empty();
        let has_shared_functions = file.items.iter().any(|item| matches!(item, Item::Fn(_)));
        let mut records = Vec::new();
        for (symbol, kind) in types {
            let mut visitor = Self { references: BTreeSet::new() };
            for item in &file.items {
                let owns_item = match item {
                    Item::Struct(v) => kind == "struct" && v.ident == symbol,
                    Item::Enum(v) => kind == "enum" && v.ident == symbol,
                    Item::Trait(v) => kind == "trait" && v.ident == symbol,
                    Item::Union(v) => kind == "union" && v.ident == symbol,
                    Item::Impl(v) => Self::impl_owner(v).as_deref() == Some(symbol.as_str()),
                    _ => false,
                };
                if owns_item { visitor.visit_item(item); }
            }
            let mut own_diagnostics = diagnostics.clone();
            if has_shared_imports {
                own_diagnostics.push("shared module imports: unused imports are not attributed to individual types".into());
            }
            if has_shared_functions {
                own_diagnostics.push("shared module-level free functions have no single type owner".into());
            }
            records.push(SourceRecord {
                source_file: source_file.clone(),
                module_name: module_name.clone(),
                symbol,
                kind: kind.into(),
                sha256: source_sha256.clone(),
                imports: imports.clone(),
                references: visitor.references,
                diagnostics: own_diagnostics,
            });
        }
        Ok(records)
    }

    fn impl_owner(item: &syn::ItemImpl) -> Option<String> {
        match item.self_ty.as_ref() {
            syn::Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 =>
                path.path.segments.first().map(|segment| segment.ident.to_string()),
            _ => None,
        }
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
                out.push((n.ident.to_string(), path.join("::")));
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
