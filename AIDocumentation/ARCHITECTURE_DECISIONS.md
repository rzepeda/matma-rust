# MATMA Rust — Architecture Decision Record (ADR-001)

Status: **accepted Rust-specific amendments RS-12 through RS-17, with implementation limitations noted below**. The original seven MATMA rules remain normative except for the owner-approved Rust type-atomicity amendment. This document distinguishes implemented mechanisms from source-analysis and execution limitations.

## Inherited, non-negotiable MATMA decisions

1. **Rust-specific amendment to Rule 1 (owner-approved symbol atomicity):** a named `struct`, `enum`, `trait` or `union` is an atomic MATMA identity independent of its containing file. Multiple types may share a file. Each has its own mirror/manifest; associated `impl` blocks belong to the matching type. This explicitly revises the original one-class-per-file normative requirement rather than silently reinterpreting it.
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
| RS-02 | Index each named top-level `struct`, `enum`, `trait`, or `union` as a distinct identity, including multiple types in one source file; reject duplicate fully qualified identities. | Each type maps to a separate mirror under its `<source-relative-file.rs>/` directory; traits still require implementor-based contract tests. |
| RS-03 | A file's Rust module identity is derived from its path under the configured `src/` root; `impl` blocks belong to that file and do not create new identity. | First version supports a single ordinary Cargo crate/source root, not full workspaces or `#[path]` remapping. |
| RS-04 | Use `syn` v2 for AST parsing; direct resolved type/path uses are L1; unused `use` aliases and re-exports create conservative L2 edges. Do not fabricate internal edges for ambiguous/external/unexpanded macro paths. | Align Python's evidence-weighted graph with Rust's no-import-side-effects model; import-only L2 is conservative, not a runtime dependency claim. Static resolution intentionally covers `crate::`, `self::`, `super::` and explicit imports; no rustc name resolution. |
| RS-05 | `--level` is inclusive: 0 target unit; 1 target interaction plus direct used dependents; 2 includes import-only relationships and two usage hops, higher levels expand by minimum weighted distance. | Do not silently downgrade or omit a lower-level selection. Cycles use a visited set seeded with the original type and weighted shortest paths; the origin never reappears as a dependent. |
| RS-06 | Mirrored unit, interaction and mock files are linked into their owning source module using managed `#[cfg(test)] #[path = "..."]` declarations. | Child modules inherit Rust privacy; Cargo runs them through the parent library/binary test target. Existing MATMA-owned external entrypoints are removed on scaffolding. |
| RS-07 | Keep centralized mock templates as compilable but nonfunctional structs; require trait DI seam to author behavioral mocks. Generate explicitly failing `#[test]` placeholders, never passing assertions. | Concrete types without an injectable trait cannot be transparently replaced in Rust. Diagnose seams rather than clone real behavior. |
| RS-08 | Use deterministic `serde_yaml` manifests and `.index.auto.yaml` metadata under `tests/mirror/`, unversioned via targeted `.gitignore`. Index performs a full rebuild with owned-file safety checks. | Impact reads persisted metadata, rehashes known files and stats known directories; it is not O(1) for the whole operation, though no AST parse or source-tree traversal occurs during selection. |
| RS-09 | Business and regression tests are explicitly registered in user-owned `tests/matma-verification.yaml`. Impact only returns test paths; execution is always explicitly through Cargo or application commands. | Workflow/regression semantics differ from reverse impact radius. |
| RS-10 | Preserve UTF-8 JSON/text/YAML CLI output and cross-platform tests; never overwrite developer-authored tests or mocks. Production changes are restricted to explicitly managed, test-only bridge declarations. | Avoid false coverage, unsafe path traversal and symlink mutation. |
| RS-11 | Package three independent, installable instruction bundles: Claude Code, ChatGPT/Codex, Google Antigravity. | Store each under its own distribution folder, avoiding collisions where Codex and Antigravity share `.agents/skills`. |

## Decisions needing special attention before treating Rust as stable

- **Type granularity and shared module code:** Multiple hand-written types in one file are supported, but shared free functions, macro expansions, generated types and complex import resolution are not yet fully attributed to individual types.
- **Trait-only targets:** a `trait` cannot itself be instantiated as Rule 2's one real object. Contract tests must use an actual implementation. Whether trait-only files should receive standalone mirrored test artifacts, or just dependency metadata, is subject to owner acceptance.
- **Private APIs:** the approved scaffolder inserts managed `#[cfg(test)]` child-module bridges in source files, allowing mirrored tests to use Rust privacy naturally. This is an explicit test-only exception to the original no-source-edit requirement.
- **Static completeness:** nested modules, macro-generated identifiers, associated types, blanket impls, full re-exports, conditional compilation features, cross-crate and proc-macro dependencies are not claimed to be exhaustively resolved in v0.1. Affected business/regression evidence remains necessary.
- **Rust toolchain:** minimum 1.81 is the package declaration, subject to actual compilation and CI verification.

## Mirroring example

```text
src/services/payment_gateway.rs                 # struct PaymentGateway (and its impls)
tests/unit/mirror/services/payment_gateway.rs/test_payment_gateway.rs
tests/interaction/mirror/services/payment_gateway.rs/test_payment_gateway_interaction.rs
tests/mocks/mirror/services/payment_gateway.rs/mock_payment_gateway.rs
tests/mirror/services/payment_gateway.rs/payment_gateway.auto.yaml
tests/matma_unit.rs                             # generated Cargo integration entrypoint
tests/matma_interaction.rs                      # generated Cargo integration entrypoint
tests/business/                                # manual process tests; NOT mirrored
tests/regression/                              # manual E2E tests; NOT mirrored
```

## Acceptance requirements

`cargo test --all-targets` passes in Rust/Linux and Windows CI. Test fixtures prove naming violations, type/reference resolution, Level 0/1/2, stale-index failures, no overwrites, deliberate failing placeholders, and no implicit test execution. Full runtime/cross-crate integration and measured performance require future independent acceptance. A passing CI run for this branch does not authorize release publication, crates.io packaging, a version tag, or merge into `main`.

## RS-12 — Cycle-safe type-level reverse impact (approved 2026-11-09)

The original type is inserted into a settled/visited set before reverse traversal. Each subsequent type is settled at most once. Edges may be cyclic, including A → B → A in a shared Rust source file. The persisted graph preserves each type's individual outbound dependencies and computes reverse impact using **minimum weighted distances** (usage = 1, import-only = 2). A shorter path supersedes a longer one before a type settles; the origin never appears among its own dependents. Tests and contexts deduplicate by type identity, and physical sources remain independently listed.

The previously agreed native `#[cfg(test)]` child-module privacy bridge is **not implemented in this cycle-safe indexing change**. Existing top-level Cargo integration test scaffolds cannot inspect private members. This bridge requires its own safe implementation.

## RS-13 — Per-type mirrors and no impact by file co-location (approved 2026-11-09)

Each named type has independent mirrored unit, interaction, mock and YAML manifest paths, even when A and B share a source file. Selecting A at level zero selects only A's unit test. Higher levels select other types only from real reverse type dependency edges, never shared source filename or module visibility. Shared unused Rust imports in multi-type files are not assigned as dependencies of every type; actual type references remain indexed. The affected_sources collection is metadata, not a type-level test selector. Multi-type files require fully qualified type identities for business and regression registrations; ambiguous file-wide registrations fail. File SHA-256 freshness does not establish which type changed, and automatic per-symbol diff detection is not yet implemented.

## RS-14 — Native private tests and safe scaffolding (approved and implemented)

Scaffold creates source-path-mirrored unit, interaction, and centralized mock files, then installs deterministic, marked `#[cfg(test)]` child-module declarations in the original Rust source module. Private fields and methods remain private to production consumers; the child tests gain exactly the visibility Rust grants. Repeated scaffolds are idempotent. An authorized bridge is the sole exception to the previously preferred non-editing of production source. Legacy generated external test entrypoints are removed only when MATMA-owned. Validate checks bridge drift, duplications, and deliberately unimplemented test placeholders.

## RS-15 — Creator-aware testing (approved; concrete fixture verified)

Unit A uses real A and mocks any downstream behavior it exercises. Interaction A → B uses real A and real B, including B's actual production construction by A. Unit B may be created by centralized MockA when B depends on a creator/state provider; B remains real. A stateful MockA may provide changing state throughout B's lifetime. Other downstream collaborators remain mocked for the unit and pairwise tiers. No generic replacement of Rust concrete types is implied: the Rust code must expose a suitable trait/generic/seam, or a test fixture must prepare B without copying business logic. The consumer fixture compiles and runs these boundaries; the CLI does not synthesize arbitrary working trait mock implementations.

## RS-16 — Trait contracts and real implementors (approved and indexed)

Traits receive individual type identities, manifests, and mirrored contract-test files. A trait is never instantiated. Its contract tests are authored using **real** implementations; the indexer stores concrete implementor identities and reverse impact flows to each implementor. Same-file and explicitly resolved cross-file `impl Trait for Type` are attributed to the type owner. Generic contract-test assertions cannot be inferred reliably from a trait signature and remain developer/agent authored.

## RS-17 — Per-type AST fingerprints, revisions and bounded context (approved and implemented)

Each type's normalized AST token fingerprint includes its declaration and attributed impl blocks, independent of its source file's SHA-256 freshness hash. `index` saves the prior meaningful type inventory in a managed previous-index snapshot. `changes` reports changed/removed symbols and selects affected test files using **both previous and current dependency graphs**, including cycles and removed edges. `context` reads only the indexed source regions owned by the requested type, including external impl regions. A source-file change alone cannot mark its unrelated sibling types changed. Macros, shared helpers, inferred aliases, feature-specific expansions, and broad workspaces remain areas of incomplete static attribution; surface uncertainty rather than claim exhaustive results.

## Phase acceptance

An independent consumer Cargo crate in `fixtures/consumer/` is exercised in Linux and Windows CI: developer-authored mirrored private tests, trait-contract tests with two real implementors, a real A → real B creation interaction, and a real B created by stateful centralized MockA. CLI contracts additionally verify cycle handling, per-type impact, AST changes and cross-file impl ownership. This is not a regression gateway with external/live services; those require a separately approved application environment.
