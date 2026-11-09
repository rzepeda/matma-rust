# MATMA Rust CLI, decisions and verification guide

Install from a MATMA Rust checkout using `cargo install --path /path/to/matma-rust` (add `--locked` only when a lockfile is included). Use the executable in a **consumer Rust project root**.

```sh
matma-indexer validate --check -o json
matma-indexer index -o json
matma-indexer scaffold -f src/services/payment_gateway.rs -o json
matma-indexer impact -f PaymentGateway -l 2 -o json
cargo test --test matma_unit
cargo test --test matma_interaction
```

The CLI has subcommands `validate`, `index`, `scaffold`, `impact`; shared `-s/--source-root`, `-t/--tests-root`, `-o/--format=text|json|yaml`; `index -S/--scaffold`; `validate -c/--check`; `scaffold -f/--target`; and `impact -f/--target -l/--level`. Avoid imaginary `cargo matma` subcommands. The existing host Cargo test runner remains the execution interface.

The seven accepted MATMA rules: (1) one atomic type identity independent of source file; (2) real isolated unit and mock every dependency; (3) centralized mirrored mocks; (4) exactly two real collaborators in pairwise interaction, mock all others; (5) real internal business workflow, mock only infrastructure boundaries; (6) regression with real data and real/live/containerized participating services; (7) mirrored unit/interaction/mock/manifest vs nonmirrored process-tier business/regression. Do not let native-language adaptations weaken any testing boundary.

`tests/unit/mirror/`, `tests/interaction/mirror/`, `tests/mocks/mirror/` and `tests/mirror/` contain a separate companion for each named type under its containing source filename, such as `tests/unit/mirror/services/shared.rs/test_a.rs`. Scaffold also creates `tests/matma_unit.rs` and `tests/matma_interaction.rs` because Cargo does not otherwise discover nested Rust integration test modules. All generated test placeholders panic deliberately. Trait-contract mocks need a manually authored `impl Trait for MockType`, not a guessed implementation. External tests can see only public crate APIs.

Impact never executes tests. At level 0 it reports a target unit; level 1 adds the target's interaction plus usage-dependent tests; level 2 conservatively adds import-only or two-hop effects; greater levels expand inclusively. Missing test files are a separate list. Business/regression registration is optional, explicit, and not inferred from paths: a user-owned `tests/matma-verification.yaml` with `entries: [{source: src/path.rs, business: [tests/business/workflow.rs], regression: [tests/regression/live.rs]}]`. Invoke these suites and real services separately.

If impact says "missing MATMA index" or "stale MATMA index", rerun `matma-indexer index`; do not claim an unindexed selection. Index YAML is generated and ignored by Git, while test files and the user-owned registration remain versioned. Rust static analysis cannot guarantee resolution of macros, proc-macros, conditional features or full multi-crate workspace semantics; report such limits and supplement with business/regression evidence.

Type-level graph semantics: `impact` reports both `affected_symbols` and `affected_sources`. The original type is treated as visited, so A → B → A cannot recur; weighted shortest paths place every type at its minimum impact level. Specify a type name or qualified identity when a source file contains multiple types. The agreed `#[cfg(test)]` privacy bridge for private-member tests is not yet implemented; these generated external Cargo tests can access only public items.

**Mock and test mirroring per type:** src/services/order_processing.rs containing A and B produces tests/unit/mirror/services/order_processing.rs/test_a.rs and test_b.rs, and tests/mocks/mirror/services/order_processing.rs/mock_a.rs and mock_b.rs. Scaffolding or selecting A must not select B by file co-location. Shared unused imports in multi-type files are not type dependencies; an actual reverse usage edge may select B at higher levels. Use affected_symbols for test selection, not the deduplicated affected_sources list. For multi-type files, tests/matma-verification.yaml must register fully qualified type identities; ambiguous file-wide registrations fail.
