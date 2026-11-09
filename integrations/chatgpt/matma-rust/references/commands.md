# MATMA Rust CLI, decisions and verification guide

Install from a MATMA Rust checkout using `cargo install --path /path/to/matma-rust --locked` when lockfile is available; without a lockfile, omit `--locked`. Use the executable in a **consumer Rust project root**.

```sh
matma-indexer validate --check -o json
matma-indexer index -o json
matma-indexer scaffold -f src/services/payment_gateway.rs -o json
matma-indexer impact -f PaymentGateway -l 2 -o json
cargo test --test matma_unit
cargo test --test matma_interaction
```

The CLI has subcommands `validate`, `index`, `scaffold`, `impact`; shared `-s/--source-root`, `-t/--tests-root`, `-o/--format=text|json|yaml`; `index -S/--scaffold`; `validate -c/--check`; `scaffold -f/--target`; and `impact -f/--target -l/--level`. Avoid imaginary `cargo matma` subcommands. The existing host Cargo test runner remains the execution interface.

The seven accepted MATMA rules: (1) one atomic type/file identity; (2) real isolated unit and mock every dependency; (3) centralized mirrored mocks; (4) exactly two real collaborators in pairwise interaction, mock all others; (5) real internal business workflow, mock only infrastructure boundaries; (6) regression with real data and real/live/containerized participating services; (7) mirrored unit/interaction/mock/manifest vs nonmirrored process-tier business/regression. Do not let native-language adaptations weaken any testing boundary.

`tests/unit/mirror/`, `tests/interaction/mirror/`, `tests/mocks/mirror/` and `tests/mirror/` follow source-relative paths. Scaffold also creates `tests/matma_unit.rs` and `tests/matma_interaction.rs` because Cargo does not otherwise discover nested Rust integration test modules. All generated test placeholders panic deliberately. Trait-contract mocks need a manually authored `impl Trait for MockType`, not a guessed implementation. External tests can see only public crate APIs.

Impact never executes tests. At level 0 it reports a target unit; level 1 adds the target's interaction plus usage-dependent tests; level 2 conservatively adds import-only or two-hop effects; greater levels expand inclusively. Missing test files are a separate list. Business/regression registration is optional, explicit, and not inferred from paths: a user-owned `tests/matma-verification.yaml` with `entries: [{source: src/path.rs, business: [tests/business/workflow.rs], regression: [tests/regression/live.rs]}]`. Invoke these suites and real services separately.

If impact says "missing MATMA index" or "stale MATMA index", rerun `matma-indexer index`; do not claim an unindexed selection. Index YAML is generated and ignored by Git, while test files and the user-owned registration remain versioned. Rust static analysis cannot guarantee resolution of macros, proc-macros, conditional features or full multi-crate workspace semantics; report such limits and supplement with business/regression evidence.
