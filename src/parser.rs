use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use quote::ToTokens;
use syn::spanned::Spanned;
use crate::source_record::{SourceRegion,ExternalImplementation};
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
        let module_name = Self::module_path(path,source_root)?;
        let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let has_shared_imports = types.len() > 1 && !imports.is_empty();
        let has_shared_functions = file.items.iter().any(|item| matches!(item, Item::Fn(_)));
        let mut records = Vec::new();
        for (symbol, kind) in types {
            let mut visitor = Self { references: BTreeSet::new() };
            let mut fragments = Vec::new();
            let mut regions = Vec::new();
            let mut implemented_traits = BTreeSet::new();
            for item in &file.items {
                let owns_item = match item {
                    Item::Struct(v) => kind == "struct" && v.ident == symbol,
                    Item::Enum(v) => kind == "enum" && v.ident == symbol,
                    Item::Trait(v) => kind == "trait" && v.ident == symbol,
                    Item::Union(v) => kind == "union" && v.ident == symbol,
                    Item::Impl(v) => Self::impl_owner(v).as_deref() == Some(symbol.as_str()),
                    _ => false,
                };
                if owns_item {
                    visitor.visit_item(item);
                    fragments.push(item.to_token_stream().to_string());
                    let span = item.span();
                    regions.push(SourceRegion {
                        source_file: source_file.clone(),
                        start_line: span.start().line,
                        start_column: span.start().column,
                        end_line: span.end().line,
                        end_column: span.end().column,
                        kind: if matches!(item, Item::Impl(_)) {"impl"} else {"declaration"}.into(),
                    });
                    if let Item::Impl(implementation) = item {
                        if let Some((_, trait_path, _)) = &implementation.trait_ {
                            implemented_traits.insert(Self::path_text(trait_path));
                        }
                    }
                }
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
                fingerprint: format!("{:x}", Sha256::digest(fragments.join("\n").as_bytes())),
                regions,
                implemented_traits,
                imports: imports.clone(),
                references: visitor.references,
                diagnostics: own_diagnostics,
            });
        }
        Ok(records)
    }


    /// Handle ordinary module files, as well as crate roots and mod.rs.
    fn module_path(path: &Path, source_root: &Path) -> Result<String> {
        let rel = path.strip_prefix(source_root)?;
        let mut parts: Vec<_> = rel.components()
            .map(|v|v.as_os_str().to_string_lossy().to_string()).collect();
        let filename = path.file_name().and_then(|f|f.to_str()).unwrap_or("");
        if filename == "lib.rs" || filename == "main.rs" || filename == "mod.rs" {
            parts.pop();
        } else if let Some(last) = parts.last_mut() {
            *last = path.file_stem().and_then(|s|s.to_str()).unwrap_or("").to_string();
        }
        Ok(if parts.is_empty() { "crate".into() } else { format!("crate::{}",parts.join("::")) })
    }

    /// Extract implementation blocks from every file, including files that
    /// declare no named primary types. Ownership is resolved in Project.scan.
    pub fn external_implementations(path: &Path, root: &Path, source_root: &Path) -> Result<Vec<ExternalImplementation>> {
        let input = fs::read_to_string(path)?;
        let parsed = syn::parse_file(&input)?;
        let source_file = path.strip_prefix(root)?.to_string_lossy().replace('\\',"/");
        let module_name = Self::module_path(path,source_root)?;
        let mut imports = BTreeMap::new();
        for item in &parsed.items {
            if let Item::Use(using) = item {
                let mut pairs = Vec::new();
                Self::imports(&using.tree, Vec::new(), &mut pairs, &mut Vec::new());
                for (alias, target) in pairs { imports.insert(alias,target); }
            }
        }
        let mut found = Vec::new();
        for item in &parsed.items {
            let Item::Impl(implementation) = item else { continue; };
            let syn::Type::Path(path) = implementation.self_ty.as_ref() else { continue; };
            let mut visitor = Self { references:BTreeSet::new() };
            visitor.visit_item(item);
            let span = item.span();
            found.push(ExternalImplementation {
                owner: Self::path_text(&path.path),
                source_file:source_file.clone(),
                module_name:module_name.clone(),
                imports:imports.clone(),
                references:visitor.references,
                normalized:item.to_token_stream().to_string(),
                region: SourceRegion {
                    source_file:source_file.clone(),
                    start_line:span.start().line,
                    start_column:span.start().column,
                    end_line:span.end().line,
                    end_column:span.end().column,
                    kind:"impl".into(),
                },
                contract:implementation.trait_.as_ref().map(|(_,name,_)|Self::path_text(name)),
            });
        }
        Ok(found)
    }

    fn path_text(path: &syn::Path) -> String {
        path.segments.iter().map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>().join("::")
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
