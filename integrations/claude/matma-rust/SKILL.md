---
name: matma-rust
description: Apply MATMA's seven-tier-boundary rules to Rust code changes, TDD, mirrored test scaffolding, and indexed impact queries in Cargo projects.
---

# MATMA Rust for Claude Code

Use this skill when developing, refactoring, or reviewing Rust code whose project uses MATMA. Read `references/commands.md` for CLI syntax and recovery before applying changes.

1. Preserve existing Rust source and one-primary-type-per-file naming; do not silently refactor application structure to satisfy tool convenience. When unsupported patterns arise, explain the conflict with Rule 1 rather than deleting helpers.
2. Confirm that `matma-indexer` is installed; do not claim to run it without terminal access. From the consumer Rust project root, run `matma-indexer validate --check -o json` then `matma-indexer index -o json` after source/dependency changes.
3. Use `matma-indexer scaffold --target <source-or-type> -o json` only when mirrored tests/mocks are missing. Generated test placeholders must remain **red**, not skipped or passed, until meaningful assertions are written. Never overwrite developer-authored tests.
4. Use `matma-indexer impact --target <source-or-type> --level 1 -o json` (or other needed level) to select the tests. Treat missing paths as unfinished coverage; impact does not run tests.
5. Unit: one real type, all collaborator trait seams mocked via centralized `tests/mocks/mirror/`. Interaction: exactly two real types; all others mocked. Business: real internal domain, mock only external services. Regression: real participating services and datasets.
6. Run explicitly selected Cargo tests (`cargo test --test matma_unit`, `cargo test --test matma_interaction`) after replacing failing placeholders; separately invoke business/regression suites and live service gateways. Report commands, actual pass/fail, unfinished tests, and unsupported analysis honestly.
7. If the index is stale, explicitly rerun `index`; never conceal or automatically bypass the error by executing full AST analysis during `impact`.

Rust-specific limits: ordinary Cargo single-crate module resolution first; traits cannot be instantiated directly; external integration tests cannot inspect private items; macro and procedural expansion may require manual review. The skill never modifies live infrastructure or authorizes deployment without approval.
