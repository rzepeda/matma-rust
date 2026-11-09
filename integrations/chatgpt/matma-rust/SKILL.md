---
name: matma-rust-chatgpt
description: Enforce the complete seven-rule MATMA Rust methodology, type-level impact, centralized mocks, and Cargo testing in ChatGPT/Codex.
---

# MATMA Rust for ChatGPT / Codex

# MATMA Rust — Mandatory agent instructions

Use this skill when modifying, testing, refactoring, or reviewing a Rust/Cargo project that adopts MATMA. **These rules are normative, not optional suggestions.** This file contains the entire testing methodology needed to act correctly after installation. The adjacent `references/commands.md` adds syntax and examples; neither access to the MATMA repository nor loading an unrelated README is required to know the rules.

**MATMA** means **Multi-Tiered Agentic Testing & Mock Architecture**. It optimizes agentic TDD, targeted verification, and bounded LLM context by combining strict real/mock testing tiers with deterministic, type-level source/test/mock mirrors and persisted dependency-impact manifests. MATMA selects tests; **Cargo executes tests**. Never represent a generated placeholder as passing coverage.

## Seven architectural rules (required)

### Rule 1 — Deterministic type identity and source mirroring

The original cross-language rule specifies one class per source/test/mock file, with matching filenames. **The approved Rust amendment supersedes that structural constraint:** each named `struct`, `enum`, `union`, or `trait` is a distinct atomic MATMA identity (`crate::module::Type`), even when multiple types share a `.rs` file. Associated `impl` blocks belong to that type, including resolvable implementations in other files; an `impl` is not a separate test target. Give each type its own unit, interaction, mock, and manifest companions. Sharing a physical file, module, or private visibility **does not create a dependency or expand impact**. Never split production types merely to satisfy the original one-class-per-file convention.

### Rule 2 — Isolated unit testing with mocks

For a unit test of **A**, instantiate and execute **real A**. Replace **all exercised external interactions and collaborators** with appropriate centralized mocks, including transitive external behavior reached by A. Unit B similarly uses **real B**, never real A when A is an exercised external collaborator. If MockA must construct a real B for B's unit test, MockA supplies controlled creation values or state without duplicating A's business logic. When Rust cannot substitute a concrete collaborator without a compatible trait, generic, constructor seam, or fixture, identify the isolation gap; do **not** silently run a second real collaborator and call it a unit test.

### Rule 3 — Centralized mirrored mock registries

Reusable mocks live at `tests/mocks/mirror/...`, one deterministic mock companion per named type. Test modules import these shared doubles instead of introducing equivalent inline mock implementations. Preserve authored mocks. Implement behavior through genuine Rust trait/generic or other valid injection seams; a scaffolded `MockA` struct alone is **not** a functioning replacement. Mock factories and mutable-state providers belong in the central mock registry when needed; mocks must not reimplement the production business decisions they replace.

### Rule 4 — Strict pairwise interaction isolation

An interaction test of **A → B** uses **real A and real B**, and **all other downstream collaborators needed by either are mocked**. If real A creates B, the test MUST exercise A's real production construction pathway and resulting real B behavior. If B depends on A's changing state, use two real objects in the A/B interaction tier; a unit test of B instead uses a stateful MockA and real B. A test with an additional real C is **not** a strict MATMA pairwise test. Cycles A ↔ B are valid and do not change the real/mocked instantiation rules.

### Rule 5 — Business workflow tests mock only infrastructure boundaries

Process-oriented tests in `tests/business/` execute **real internal domain types and workflow logic** across multiple steps. Mock **only external/infrastructure services at their boundaries** (databases, HTTP clients, LLMs, vector stores, filesystem/network gateways, etc.). Do not mock the internal business logic that the workflow is meant to verify. Business tests are NOT one-per-source mirrored files.

### Rule 6 — Full-system regression uses no mocks

Maintain `tests/regression/` with **real datasets and real/live/containerized participating services**, without mocks for those services. Run this regression gateway before merge or deployment where the consuming application requires it. MATMA's indexer only **selects/reports** registered paths; it does not execute the gateway or prove it passed. Do not run potentially external/live tests without the appropriate environment and authorization.

### Rule 7 — Mirrored tests versus process-oriented tests

Mirrored, type-specific artifacts live under `tests/unit/mirror/`, `tests/interaction/mirror/`, `tests/mocks/mirror/`, and generated `tests/mirror/**/*.auto.yaml`. The **source filename becomes a mirror directory**, allowing distinct types in one source file. Business and regression tests live in independent, process-oriented `tests/business/` and `tests/regression/`; register relevant paths in `tests/matma-verification.yaml` with a fully qualified type when a file contains multiple types.

Example when `src/services/order_processing.rs` declares both A and B:

```text
src/services/order_processing.rs
tests/unit/mirror/services/order_processing.rs/test_a.rs
tests/unit/mirror/services/order_processing.rs/test_b.rs
tests/interaction/mirror/services/order_processing.rs/test_a_interaction.rs
tests/interaction/mirror/services/order_processing.rs/test_b_interaction.rs
tests/mocks/mirror/services/order_processing.rs/mock_a.rs
tests/mocks/mirror/services/order_processing.rs/mock_b.rs
tests/mirror/services/order_processing.rs/a.auto.yaml
tests/mirror/services/order_processing.rs/b.auto.yaml
tests/business/          # workflow-centric, not mirrored
tests/regression/        # live-system-centric, not mirrored
```

## Rust-specific contracts and hard boundaries

- **Trait contracts:** a trait has its own type identity, manifest, and mirrored contract tests. A trait cannot be instantiated. Write assertions that exercise **real concrete implementors**, including default trait methods; each implementor also has its own unit tests. A trait change may affect every relevant implementor.
- **Private APIs:** `scaffold` creates marked `#[cfg(test)] #[path = "..."]` child-module bridges in the owning source module. Mirrored tests compile as children of that module, gaining native Rust private access without exposing public production APIs. Only MATMA-owned bridge declarations may be managed; preserve authored code. Move temporary inline tests into mirrors before completion. The old top-level `tests/matma_unit.rs`/`tests/matma_interaction.rs` Cargo entrypoint scheme is obsolete.
- **Precise change detection:** normalized AST fingerprints, source-region ownership, and `context` are **per type**, not per file. If only A changes, do not select B merely because both share a file. `changes` compares previous/current indexed revisions and old/new dependency edges; `impact` selects from a specified type. Cyclic graphs use a visited/settled set with minimum weighted reverse-impact distances; the original type is not rediscovered as a dependent. `affected_sources` is physical-file metadata, **not a test selection key**.
- **Limits:** macros, dynamic/ambiguous resolution, `cfg` variants, and workspaces may need review. No unsupported pattern may be silently treated as comprehensive evidence. Report missing injection seams, unindexed constructs, and scope uncertainty.

## Mandatory development workflow

1. **Locate the target** by its named Rust type, not just its file. Confirm the consumer project's Cargo context and whether `matma-indexer` is actually available. Read `references/commands.md` for exact syntax; if unavailable, apply the seven rules manually and explicitly report the missing tool.
2. **Establish an index baseline** from the consumer project root with `matma-indexer index -o json`. Use `matma-indexer context -f A -o json` to load only A's declaration, associated `impl` regions, dependencies, and diagnostics, rather than unrelated types' code.
3. **Scaffold as needed** with `matma-indexer scaffold -f A -o json`. Scaffold creates missing test/mock mirrors and native private-test bridges without overwriting existing authored tests. For trait targets, ensure authored contract assertions run against real implementors. Temporary inline tests must be migrated into mirrored files.
4. **TDD with real tier boundaries:** write meaningful unit/pairwise assertions, create centralized mock implementations through valid seams, and observe the expected red tests first. Fix production behavior, then run the relevant Cargo tests to green. Scaffolded placeholders intentionally panic: **replace them**, never skip or disguise them.
5. **Refresh source impact** by running `matma-indexer index -o json` after source changes, then `matma-indexer changes -l 1 -o json` for changed types, or `matma-indexer impact -f A -l 1 -o json` for an explicit target. Choose level 0 for the target unit only; level 1 includes interaction and directly affected dependents; higher levels expand by weighted reverse dependencies. Select tests using `affected_symbols` and returned test paths, not entire files.
6. **Validate and execute:** run `matma-indexer validate --check -o json`, then `cargo test --lib order_processing::matma_unit_a` or the appropriate owning library/binary test filter, plus interaction, contract, and broader affected tests as required. Add registered business tests, and execute real regression suites before merge/deploy where authorized. MATMA does not execute tests itself.
7. **Report evidence:** state which files/types changed, real versus mocked participants, tests actually run and their results, remaining failing placeholders/missing coverage, and unsupported graph analysis. Do not claim completion when validation fails or required tests were not executed.

**Reference for CLI flags and recovery:** `references/commands.md` (shipped in this same skill package). The optional repository ADR offers implementation history; it is **not required** to learn these rules.
