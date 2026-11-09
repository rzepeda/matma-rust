# MATMA Rust — CLI reference and verification contracts

Use from a **consumer Rust project root**, with the `matma-indexer` executable installed from the MATMA Rust checkout.

```sh
matma-indexer index -o json
matma-indexer scaffold -f A -o json
# Replace deliberately failing mirror test placeholders with real tests.
matma-indexer validate --check -o json
matma-indexer context -f A -o json
matma-indexer impact -f A -l 1 -o json
cargo test --lib order_processing::matma_unit_a
cargo test --lib order_processing::matma_interaction_a
# Following a source edit:
matma-indexer index -o json
matma-indexer changes -l 2 -o json
```

Commands: `index` (AST inventory, reverse dependency manifests and prior-version snapshots); `scaffold` (per-type unit/interaction/mock/manifest mirrors plus marked native child-module bridges); `validate --check` (syntax, bridge drift, unfinished intentionally failing tests); `impact -f Type -l N` (type-scoped read-only selection); `context -f Type` (only AST-owned declaration/implementation source regions and dependencies); `changes -l N` (compare last two meaningful indexed revisions using old/new union reverse edges). Common `-s/--source-root`, `-t/--tests-root`, `-o/--format`; `index -S/--scaffold`. `changes` requires two indexed versions and does not infer changed symbols from a Git ref.

Mapping for source file `src/services/order_processing.rs` with A and B: `tests/unit/mirror/services/order_processing.rs/test_a.rs`, `.../test_b.rs`; `tests/interaction/mirror/services/order_processing.rs/test_a_interaction.rs`, `.../test_b_interaction.rs`; `tests/mocks/mirror/services/order_processing.rs/mock_a.rs`, `.../mock_b.rs`; `tests/mirror/services/order_processing.rs/a.auto.yaml`, `.../b.auto.yaml`. Never expand impact to another type merely because of file co-location. Type identities are `crate::module::Type`. For business/regression registrations, use a fully qualified type for multi-type files; file paths are ambiguous.

The scaffolded source bridge uses `#[cfg(test)] #[path = "..."] mod matma_unit_a;` and corresponding interaction/mock modules; Cargo compiles them inside the declaring source module, preserving normal private visibility. This is a test-only exception to MATMA's general no-source-modification preference. Existing MATMA-owned external integration entrypoints are retired; public-API external integration tests may still be used separately.

Unit A: only A is real, with centralized mocks for collaborators; interaction A→B: both selected production types real, including B's actual creation by real A; unit B: MockA can create real B when a compatible Rust abstraction exists and provide changing state throughout B's lifetime. Every other dependency at those levels remains mocked. A bare generated MockA is *not* a working fake or a license to copy A's business behavior. Traits are contracts, never instances: author shared real-implementor contract assertions, consult trait manifest `implementors`. Business workflows use real domain with mocked infrastructure; regression uses real services and datasets, with explicit authorization.

Missing tests, unresolved macros, difficult proc macros, `cfg` features, imports, and multi-crate workspaces require conservative human review. `impact` and `changes` only select test files; they do not run Cargo or external services. `validate --check` fails until intentionally red scaffolds are replaced.
