# MATMA Rust — Multi-Tiered Agentic Testing & Mock Architecture

MATMA Rust is the Rust/Cargo implementation of MATMA. It uses type-level AST indexing, mirrored tests and centralized mocks, and persisted dependency information to help coding agents select the verification relevant to a code change.

## The MATMA Methodology

### 1. What is MATMA?

**MATMA (Multi-Tiered Agentic Testing & Mock Architecture)** is a software testing methodology and architectural framework designed to optimize Test-Driven Development (TDD), refactoring, and verification workflows performed by autonomous AI coding agents.

MATMA establishes **seven architectural rules** governing source organization, test isolation, mock management, dependency relationships, and verification boundaries. It addresses constraints specific to LLM-based coding agents: finite context windows, token consumption, repeated filesystem exploration, and the need for focused, deterministic feedback when modifying software.

An agent working on a single class or Rust type may need to find its tests, inspect dependencies, locate appropriate mocks, and determine which consumers could be affected by a change. MATMA combines two mechanisms to make that work predictable:

- **A rule-based testing and mocking methodology:** precise definitions of which classes or types are real and which are mocked at each testing tier.
- **A deterministic indexing and discovery architecture:** mirrored tests and mock helpers, precomputed reverse dependencies, and persisted manifests for selecting affected verification.

MATMA complements existing frameworks such as pytest and PHPUnit; **it does not replace the host project's test runner**. In Rust, Cargo remains responsible for compiling and executing tests.

### 2. Five optimization objectives

1. **Context-window and token efficiency:** retrieve the relevant type's source regions, tests, mocks, and dependency metadata rather than repeatedly loading unrelated code.
2. **Deterministic file and symbol discovery:** derive test, mock, and manifest locations from a stable Rust type identity instead of repeated repository-wide searches.
3. **Disk-materialized reverse dependencies:** build dependency relationships ahead of time and persist generated `.auto.yaml` manifests for focused impact queries.
4. **Signal-to-noise isolation:** constrain each test tier to a known set of real objects and controlled mocks, narrowing the causes of failures.
5. **Tiered verification scaling:** start with the changed type, expand to impacted interactions and consumers, and keep business and regression verification as separate tiers.

### 3. The seven architectural rules

MATMA Rust preserves the accepted testing and mocking boundaries. Its [Rust-specific amendments](AIDocumentation/RUST_RULE_AMENDMENTS.md) explicitly adapt the original structural Rule 1 to native Rust type organization.

1. **Deterministic source-to-test identity.** The original MATMA rule specifies one class per file with matching names. The accepted **Rust amendment** makes each named `struct`, `enum`, `union`, or `trait` an independent MATMA identity, even when multiple types share a file. An associated `impl` belongs to its type. Sharing a file does not imply a dependency.
2. **Isolated unit testing with mocks.** When testing A, **A is real** and every exercised collaborator outside A's boundary is mocked.
3. **Centralized mirrored mock registries.** Reusable mock implementations live under `tests/mocks/mirror/`, not as duplicate inline fakes. In Rust, substituting a mock requires an appropriate trait, generic interface, or construction seam.
4. **Strict pairwise interaction isolation.** When testing A → B, **A and B are both real** and all other downstream collaborators are mocked. If real A constructs B, the test exercises that real creation path. Conversely, B's isolated unit test may use MockA to construct **real B** and provide controlled state.
5. **Service-level boundary mocking for workflows.** Business tests exercise real internal domain types and workflows, mocking only external or infrastructure service boundaries.
6. **Integrated regression gateways.** Regression tests use real datasets and real participating services, with no service mocks, before merge or deployment. The application and its CI pipeline execute this gateway; MATMA itself does not run it.
7. **Bifurcated directory topology.** Unit tests, pairwise interaction tests, centralized mocks, and manifests mirror the source type identity; business and regression tests follow workflows rather than individual types.

### 4. Four verification tiers

| Tier | Real participants | Mocked participants | Purpose |
| --- | --- | --- | --- |
| **Unit** | One type under test | All its external collaborators | Verify the type in isolation |
| **Interaction** | Exactly two selected collaborating types | Every other downstream collaborator | Verify the real A → B relationship |
| **Business** | Internal domain types and workflow logic | External/infrastructure boundaries | Verify business processes |
| **Regression** | Full application and participating services | None | Verify real-system behavior |

Rust uses **type-level** deterministic mirroring, rather than Python's module-level or PHP's class-file mapping. The source file `src/services/order_processing.rs` can contain A and B while their tests and mocks remain independent:

```text
src/services/order_processing.rs
tests/unit/mirror/services/order_processing.rs/test_a.rs
tests/unit/mirror/services/order_processing.rs/test_b.rs
tests/interaction/mirror/services/order_processing.rs/test_a_interaction.rs
tests/mocks/mirror/services/order_processing.rs/mock_a.rs
tests/mocks/mirror/services/order_processing.rs/mock_b.rs
tests/mirror/services/order_processing.rs/a.auto.yaml
tests/mirror/services/order_processing.rs/b.auto.yaml
```

MATMA's goal is **verification proportional to the actual change**: a change to A should select A, plus other types genuinely dependent on A at the requested impact level—not every other type declared in the same source file. Type-level AST fingerprints, bounded context extraction, persisted reverse dependencies, and cycle-safe impact selection support that goal.

These are architectural objectives and verification rules, not claims of guaranteed token savings or proof that a selected test passed. MATMA selects tests; developers, agents, Cargo, and CI execute and evaluate them.

**Status:** Development implementation. The agreed Rust-specific decisions are documented in [Architecture Decisions](AIDocumentation/ARCHITECTURE_DECISIONS.md); source-of-truth methodology and explicit Rust amendments are maintained separately. Scaffolding may insert managed **test-only** `#[cfg(test)]` bridge declarations into source files without changing production API visibility; it does not generate passing tests or execute them.

## Install in a Rust application

Requires Rust/Cargo with toolchain 1.81 or newer. Until a release is published, use a checked-out local repository:

```sh
cargo install --path /path/to/matma-rust
```

Or use the executable from this repository:

```sh
cargo run -- index -o json
```

For a consuming project, run commands from **that project's root**, not the MATMA repository:

```sh
matma-indexer index -o json
matma-indexer scaffold --target PaymentGateway -o json
# Author actual tests and trait implementations in the mirrored files.
matma-indexer validate --check -o json
matma-indexer impact --target PaymentGateway --level 2 -o json
matma-indexer context --target PaymentGateway -o json
cargo test --lib matma_unit_payment_gateway
cargo test --lib matma_interaction_payment_gateway
# After source changes, explicitly reindex:
matma-indexer index -o json
matma-indexer changes --level 2 -o json
# Separately run registered business/regression suites against the required services.
```

Generated tests use `panic!("MATMA: implement ...")`; this is intentional red until replaced with real assertions and correctly injected centralized doubles. The `impact` command reports *existing* unit/interaction paths, *missing* paths, and optional registered business/regression paths; **it does not execute tests.**

## Commands

| Command | Role | Key flags |
| --- | --- | --- |
| `index` | Parse Rust AST, build graph, persist `.auto.yaml` mirrors and freshness metadata | `-s/--source-root`, `-t/--tests-root`, `-S/--scaffold`, `-o/--format` |
| `validate` | Audit type identities, test bridges and unfinished scaffolds | `-c/--check` |
| `scaffold` | Create missing mirrored unit, interaction, and mock files plus native test-only module bridges | `-f/--target` |
| `impact` | Read saved manifests, check freshness, select tests by inclusive reverse dependency depth | `-f/--target`, `-l/--level` |
| `changes` | Compare current and prior indexed type revisions, traverse union of old/new dependencies | `-l/--level` |
| `context` | Return only type-owned declaration/impl source spans with dependency contracts | `-f/--target` |

All commands support `-s/--source-root` (default `src`), `-t/--tests-root` (default `tests`) and `-o/--format text|json|yaml`. JSON is recommended for agents.

**Impact levels**: 0 = selected type unit; 1 = its interaction plus direct proven-usage dependent units/interactions; 2 = expands to import-only relationships and two-hop actual usage; higher levels continue inclusively. Results only refer to test files that exist; expected missing files are separate. Changes to source after indexing trigger a stale-index error requiring `matma-indexer index`; no automatic reindex during `impact`.

## Rust-native boundaries

Rust structs/enums/unions are primary concrete types; traits are primary contract types but cannot be instantiated. A normal Rust file may have one or more named primary types, each with its associated `impl` blocks. Glue-only `lib.rs`, `main.rs`, and `mod.rs` are not forced to contain a primary type. The AST analyzer supports explicit module/type references and `use` aliases, but it does not emulate `rustc` resolution of macros, workspace crates, or cfg-specific expansions. It reports some unknowns and does not claim whole-program precision.

The mirror creates one test/mock/manifest per type under the containing source filename, e.g. `tests/unit/mirror/services/order_processing.rs/test_a.rs` and `tests/mirror/services/order_processing.rs/a.auto.yaml`. The scaffolder now adds marked `#[cfg(test)]` native child-module bridges to the containing source file, making private members accessible without changing production API visibility. Generated external test entrypoints are deprecated; standalone public-API integration tests may continue separately.

To register non-mirrored business/regression tests, copy [`tests/matma-verification.example.yaml`](tests/matma-verification.example.yaml) to `tests/matma-verification.yaml` and update its source/test paths. MATMA reports these separately; invoke Cargo and live-service gateways explicitly.

## Agent integrations — final implementation phase

Independent skills with complete instructions and installation notes are under:

- [`integrations/claude/matma-rust/`](integrations/claude/matma-rust/) for Claude Code at `.claude/skills/matma-rust/`.
- [`integrations/chatgpt/matma-rust/`](integrations/chatgpt/matma-rust/) for ChatGPT/Codex at `.agents/skills/matma-rust-chatgpt/`.
- [`integrations/antigravity/matma-rust/`](integrations/antigravity/matma-rust/) for Antigravity at `.agents/skills/matma-rust-antigravity/`.

The folders may be copied to consumer repositories independently. Skill instructions are not a replacement for installing the MATMA binary, granting filesystem/terminal access, writing tests, or verifying them. See [`integrations/README.md`](integrations/README.md).

## Development

```sh
cargo fmt --all
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets
```

The CI workflow runs these on Linux and Windows. No benchmark or compatibility claim is made without measured evidence. See the ADR for currently unsupported Rust syntax and the release boundary.

## Type-level mirrors and circular dependencies

MATMA targets **types**, not physical files. Multiple types may coexist in `src/services/order_processing.rs` with independent mirrors, mocks, manifests and impact identities (`crate::services::order_processing::A`, `crate::services::order_processing::B`). Specify a type or full identity when a file contains multiple types; selecting the ambiguous source path is an error.

Directed type dependencies may be cyclic (A → B → A). MATMA starts reverse traversal with the original type in a settled set, processes each additional type once and selects minimum weighted path distances (usage=1, import-only=2). Impact returns `affected_symbols` as well as deduplicated `affected_sources`, and a high level neither repeats the origin nor traverses the cycle indefinitely.

Shared free functions, macros, reexports and unresolved generated code can affect multiple types. The current AST analysis does not fully attribute all such references and must not be represented as complete impact detection. The Rust child-module bridge is generated by scaffold; cfg-specific and macro-expanded source attribution is still limited.

### No impact by file co-location

When A and B share a source file, their unit tests and mocks are independent. Scaffolding A creates only test_a.rs and mock_a.rs under their respective order_processing.rs mirror directories. Impact A selects only A unless another type actually depends on A at the requested level. The affected_sources field lists physical files for context, not extra test targets; use affected_symbols and the returned test paths.

Rust imports are module-scoped. In multi-type files, unused imports are not attributed to every type as L2 edges; real per-type usages are retained. Existing conservative unused-import behavior remains for single-type files. Unresolved macros and shared free functions still require review.

Use fully qualified type identities (for example crate::services::order_processing::A) in the verification registry for multi-type files. A file-scoped registry entry is allowed only when that file defines one indexed type. Ambiguous entries fail explicitly.

## Native Rust private test bridges

`scaffold` creates mirrored tests and centralized mock modules, then inserts a **marked, test-only** `#[cfg(test)] #[path="..."] mod matma_unit_<type>;` and interaction/mock bridges into the owning source module. These are compiled as **child modules of the source file** and may access its private items, without publishing them. The source is modified only in its MATMA-owned bridge region; developer-authored tests and mocks are not overwritten. Repeated scaffolds are idempotent, and indices are refreshed after bridge insertion. The external `tests/matma_unit.rs` and `tests/matma_interaction.rs` entrypoints are superseded and removed only if MATMA-generated. Run `cargo test --lib matma_unit_a` (or the owning binary target) rather than `cargo test --test matma_unit`. The mirror is the canonical home for tests; agents must migrate any temporary inline tests into it.

## Type-owned source context and changed-type selection

Run `matma-indexer context -f A -o json` for only the AST-owned source declaration/impl regions, dependency contracts, and diagnostics; siblings' implementation bodies are not included just because they share a file. `index` now stores normalized AST token fingerprints per type, separately from the full-file SHA used to check index freshness. Run `index` before editing, then `index` after editing: meaningful differences preserve the preceding type inventory at `tests/mirror/.previous.auto.yaml`. `matma-indexer changes -l 1 -o json` returns modified symbols and the associated test files using a shortest-path traversal over **both old and new dependency graphs**. Formatting-only changes are ignored by token fingerprints. The preceding snapshot survives repeated identical reindexing. New projects need two indexed revisions before `changes` can report a baseline. This is source-index comparison; a direct Git-ref/commit comparison is not yet supported.

## Actual MATMA consumer integration

The self-contained [consumer acceptance crate](fixtures/consumer/README.md) is compiled in CI on both Ubuntu and Windows. It tests private state through mirrored tests, real A creating real B through the normal constructor, a real B using a centralized stateful MockA (including changes after construction), and contract tests against two real implementors. The second implementation is in a different source file. It provides practical verification of the instantiation rules, but does not claim that a generic scaffold can invent valid Rust trait mock behavior automatically.

The indexer records implementing types in each trait's `implementors` manifest field. A change to a trait is a real reverse dependency of implementing types. The generated trait-test stub is deliberately failing until the developer/agent writes actual reusable contract assertions over concrete real implementors. Rust's object-safety and constructor constraints still apply: a mock struct is not a drop-in replacement for a hard-coded concrete Rust type.

The `validate --check` command now reports missing/incorrect generated child-module bridges and test stubs still containing the MATMA placeholder panic. This is intentional; red scaffolds are not evidence of completed TDD.

After scaffolding, `validate --check` also flags inline `#[test]` functions inside a source module's inline test module when MATMA mirrors are present. Those development-time tests should be migrated by the agent into the corresponding permanent mirrored test file rather than being left as duplicate sources of truth.
