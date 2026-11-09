# MATMA Rust

A standalone Rust implementation of the **Multi-Tiered Agentic Testing & Mock Architecture** (MATMA). MATMA helps coding agents index Rust source dependencies ahead of time, scaffold predictable test and mock files, and **select** the relevant tests after a change. It does not replace Cargo, generate passing tests, execute selected tests, or modify production code.

**Status:** Development implementation; Rust-specific decisions in [ADR-001](AIDocumentation/ARCHITECTURE_DECISIONS.md) are proposed for review. Source-of-truth testing boundaries are the MATMA seven rules.

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
matma-indexer validate --check -o json
matma-indexer index -o json
matma-indexer scaffold --target src/services/payment_gateway.rs -o json
matma-indexer impact --target PaymentGateway --level 2 -o json
cargo test --test matma_unit
cargo test --test matma_interaction
# Separately run registered business/regression suites against the required services.
```

Generated tests use `panic!("MATMA: implement ...")`; this is intentional red until replaced with real assertions and correctly injected centralized doubles. The `impact` command reports *existing* unit/interaction paths, *missing* paths, and optional registered business/regression paths; **it does not execute tests.**

## Commands

| Command | Role | Key flags |
| --- | --- | --- |
| `index` | Parse Rust AST, build graph, persist `.auto.yaml` mirrors and freshness metadata | `-s/--source-root`, `-t/--tests-root`, `-S/--scaffold`, `-o/--format` |
| `validate` | Audit one-primary-type-per-file + naming | `-c/--check` |
| `scaffold` | Create only missing mirror test and mock files plus Cargo test entrypoints | `-f/--target` |
| `impact` | Read saved manifests, check freshness, select tests by inclusive reverse dependency depth | `-f/--target`, `-l/--level` |

All commands support `-s/--source-root` (default `src`), `-t/--tests-root` (default `tests`) and `-o/--format text|json|yaml`. JSON is recommended for agents.

**Impact levels**: 0 = selected type unit; 1 = its interaction plus direct proven-usage dependent units/interactions; 2 = expands to import-only relationships and two-hop actual usage; higher levels continue inclusively. Results only refer to test files that exist; expected missing files are separate. Changes to source after indexing trigger a stale-index error requiring `matma-indexer index`; no automatic reindex during `impact`.

## Rust-native boundaries

Rust structs/enums/unions are primary concrete types; traits are primary contract types but cannot be instantiated. A normal Rust file may have one or more named primary types, each with its associated `impl` blocks. Glue-only `lib.rs`, `main.rs`, and `mod.rs` are not forced to contain a primary type. The AST analyzer supports explicit module/type references and `use` aliases, but it does not emulate `rustc` resolution of macros, workspace crates, or cfg-specific expansions. It reports some unknowns and does not claim whole-program precision.

The mirror creates one test/mock/manifest per type under the containing source filename, e.g. `tests/unit/mirror/services/order_processing.rs/test_a.rs` and `tests/mirror/services/order_processing.rs/a.auto.yaml`. Cargo only discovers top-level integration test files, so scaffold also generates `tests/matma_unit.rs` and `tests/matma_interaction.rs`. These are managed generated entrypoints; handwritten test files are never overwritten. Tests run as external integration crates and require public host APIs; private tests need a future explicitly accepted approach.

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

Shared free functions, macros, reexports and unresolved generated code can affect multiple types. The current AST analysis does not fully attribute all such references and must not be represented as complete impact detection. The separately agreed Rust private-test child-module bridge is **not yet generated** by this version; current Cargo test entrypoints still see only public items.
