---
name: matma-rust-chatgpt
description: Apply MATMA testing, centralized Rust trait mocks, and read-only indexed impact selection while editing or reviewing Cargo projects in ChatGPT/Codex.
---

# MATMA Rust for ChatGPT/Codex

In a connected Rust coding workspace with terminal access, read `references/commands.md` and use MATMA's seven architecture rules. Do not claim this file itself grants repository access, CLI execution, or persistent ChatGPT web skill installation.

- Keep one primary Rust named type per file, mirrored paths, and user-authored code intact; `impl` blocks belong to the named type. Explain any Rust-native conflict instead of silently weakening the seven rules.
- Validate, explicitly rebuild the AST index after source edits, scaffold only missing failing tests/mocks, then request an impact selection. Prefer `-o json` and do not scan the entire repository when a persisted manifest resolves the target.
- MATMA `impact` is a read-only **selector**: it does not run Cargo, unit tests, business tests, or regression services. Stale index and missing tests are diagnostics, not successes.
- Unit: exactly one real concrete collaborator, with all other behavior injected via centralized mocks. Interaction: exactly two real types. Business: real domain, mocked infrastructure boundaries. Regression: real datasets and service dependencies.
- Author concrete traits/mock implementations for Rust DI; never pretend a generated bare `MockType` replaces an arbitrary struct or trait. Write real assertions before executing selected Cargo entrypoints.
- Report tested commands/results and any unresolved/cfg/macro/trait/private-API limitation. Require explicit live-service approval before regression actions involving external systems.

For repeatable CLI and installation instructions see `references/commands.md`.
