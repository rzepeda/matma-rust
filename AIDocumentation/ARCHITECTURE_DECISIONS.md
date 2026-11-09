# MATMA Rust — Architecture Decision Record (ADR-001)

Status: **proposed Rust-specific adaptation for review**. Seven MATMA rules remain normative. This file records the implementation choices applied in the initial branch, not an assertion that all choices were previously accepted by the owner.

## Inherited, non-negotiable MATMA decisions

1. **One class per file with matching name**, interpreted for Rust as one *named primary type* (`struct`, `enum`, `trait`, or `union`) per normal source file; snake_case basename matches PascalCase type name. Associated `impl` blocks are not additional primary types. Modules and type aliases without primary types do not become independent class-centric test targets.
2. **Isolated units:** the selected concrete type is real; all its injected collaborators must be mocked. Trait definitions themselves are not instantiable; their contracts must be verified through concrete implementors rather than pretending a trait object is a testable instance.
3. **Centralized mocks:** test doubles reside under `tests/mocks/mirror/`; no inline duplicate fake implementations. Rust mock structs must explicitly implement the required traits, including async and lifetime behavior where needed; generators do not invent functioning implementations.
4. **Strict pairwise interaction:** exactly two selected production collaborators are real; all other downstream collaborators are replaced by centralized injected trait doubles.
5. **Business workflow boundary mocking:** real internal Rust domain/services; external clients, DB, network, filesystem and other infrastructure boundaries are faked using real trait seams.
6. **Integrated regression gateway:** real datasets plus real or containerized services before merge/deploy; no mocks in regression.
7. **Bifurcated topology:** mirrored type-centric unit/interaction/mocks/manifests and nonmirrored process-centric business/regression suites.

These seven rules are from [`MATMA_Seven_Architectural_Rules.md`](sources/MATMA_Seven_Architectural_Rules.md) and [`MATMA_Architecture_Implementation_Blueprint_Updated.md`](sources/MATMA_Architecture_Implementation_Blueprint_Updated.md). Rust-specific mechanics below may not weaken their isolation boundaries.

## Language-specific engineering decisions

| ID | Decision | Rationale / remaining nuance |
| --- | --- | --- |
| RS-01 | Use Cargo standalone CLI binary named `matma-indexer`, installed as a development tool with `cargo install --path`, not a runtime dependency of consumer crates. | Preserve native Rust package management and keep production source untouched. |
| RS-02 | Index exactly one top-level named `struct`, `enum`, `trait`, or `union` per normal `.rs` file; reject mismatched names or two primary declarations. Empty helper files and `lib.rs`/`main.rs`/`mod.rs` without a primary type are allowed as glue. | Avoid pretending Rust has classes, but preserve deterministic atomic file mirroring. Named traits can be index targets, yet do not instantiate; their test placeholders require implementor-based contract tests. |
| RS-03 | A file's Rust module identity is derived from its path under the configured `src/` root; `impl` blocks belong to that file and do not create new identity. | First version supports a single ordinary Cargo crate/source root, not full workspaces or `#[path]` remapping. |
| RS-04 | Use `syn` v2 for AST parsing; direct resolved type/path uses are L1; unused `use` aliases and re-exports create conservative L2 edges. Do not fabricate internal edges for ambiguous/external/unexpanded macro paths. | Align Python's evidence-weighted graph with Rust's no-import-side-effects model; import-only L2 is conservative, not a runtime dependency claim. Static resolution intentionally covers `crate::`, `self::`, `super::` and explicit imports; no rustc name resolution. |
| RS-05 | `--level` is inclusive: 0 target unit; 1 target interaction plus direct used dependents; 2 includes import-only relationships and two usage hops, higher levels expand by minimum weighted distance. | Do not silently downgrade or omit a lower-level selection. Cycles are deduplicated. |
| RS-06 | Mirror `.rs` test modules under `tests/unit/mirror/` and `tests/interaction/mirror/`. Auto-generated Cargo entrypoints `tests/matma_unit.rs` and `tests/matma_interaction.rs` include those modules via `#[path]`. | Cargo only discovers integration test crates directly under `tests/`; nested mirrors are not directly discovered. External integration tests can test **public** APIs only; private API testing requires an explicit future design decision rather than invasive source rewriting. |
| RS-07 | Keep centralized mock templates as compilable but nonfunctional structs; require trait DI seam to author behavioral mocks. Generate explicitly failing `#[test]` placeholders, never passing assertions. | Concrete types without an injectable trait cannot be transparently replaced in Rust. Diagnose seams rather than clone real behavior. |
| RS-08 | Use deterministic `serde_yaml` manifests and `.index.auto.yaml` metadata under `tests/mirror/`, unversioned via targeted `.gitignore`. Index performs a full rebuild with owned-file safety checks. | Impact reads persisted metadata, rehashes known files and stats known directories; it is not O(1) for the whole operation, though no AST parse or source-tree traversal occurs during selection. |
| RS-09 | Business and regression tests are explicitly registered in user-owned `tests/matma-verification.yaml`. Impact only returns test paths; execution is always explicitly through Cargo or application commands. | Workflow/regression semantics differ from reverse impact radius. |
| RS-10 | Preserve UTF-8 JSON/text/YAML CLI output and cross-platform tests; never rewrite application code or user-authored tests and mocks. | Avoid false coverage, unsafe path traversal and symlink mutation. |
| RS-11 | Package three independent, installable instruction bundles: Claude Code, ChatGPT/Codex, Google Antigravity. | Store each under its own distribution folder, avoiding collisions where Codex and Antigravity share `.agents/skills`. |

## Decisions needing special attention before treating Rust as stable

- **Strict naming versus idiomatic Rust:** macro-heavy projects, multiple related helper types per source file, proc-macro crates and codegen can violate the one-primary-type rule. The current rule validates ordinary hand-written sources; any general relaxation of the atomic rule needs a separate architecture decision.
- **Trait-only targets:** a `trait` cannot itself be instantiated as Rule 2's one real object. Contract tests must use an actual implementation. Whether trait-only files should receive standalone mirrored test artifacts, or just dependency metadata, is subject to owner acceptance.
- **Private APIs:** external Cargo test crates cannot access nonpublic items. Current scaffold deliberately does not inject `#[cfg(test)]` into production files. A hybrid private-unit strategy would need an explicit future decision.
- **Static completeness:** nested modules, macro-generated identifiers, associated types, blanket impls, full re-exports, conditional compilation features, cross-crate and proc-macro dependencies are not claimed to be exhaustively resolved in v0.1. Affected business/regression evidence remains necessary.
- **Rust toolchain:** minimum 1.81 is the package declaration, subject to actual compilation and CI verification.

## Mirroring example

```text
src/services/payment_gateway.rs                 # struct PaymentGateway (and its impls)
tests/unit/mirror/services/test_payment_gateway.rs
tests/interaction/mirror/services/test_payment_gateway_interaction.rs
tests/mocks/mirror/services/mock_payment_gateway.rs
tests/mirror/services/payment_gateway.auto.yaml
tests/matma_unit.rs                             # generated Cargo integration entrypoint
tests/matma_interaction.rs                      # generated Cargo integration entrypoint
tests/business/                                # manual process tests; NOT mirrored
tests/regression/                              # manual E2E tests; NOT mirrored
```

## Acceptance requirements

`cargo test --all-targets` passes in Rust/Linux and Windows CI. Test fixtures prove naming violations, type/reference resolution, Level 0/1/2, stale-index failures, no overwrites, deliberate failing placeholders, and no implicit test execution. Full runtime/cross-crate integration and measured performance require future independent acceptance. A passing CI run for this branch does not authorize release publication, crates.io packaging, a version tag, or merge into `main`.
