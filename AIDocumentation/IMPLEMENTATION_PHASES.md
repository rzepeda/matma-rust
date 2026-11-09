# MATMA Rust implementation phases

The normative sources are in [`sources/`](sources/). The Rust adaptation is documented in [ADR-001](ARCHITECTURE_DECISIONS.md), which explicitly identifies owner-review decisions. The phase status below is **implementation status only**, not unconditional product acceptance.

| Phase | Deliverables | Status |
| --- | --- | --- |
| 0 — Cross-language decision review | Read accepted seven rules plus Python/PHP lessons; record Rust type/trait, Cargo and mock limitations and proposed decisions | Documented; Rust decisions await owner review |
| 1 — Cargo CLI | Standalone installable binary; stable `index`, `scaffold`, `impact`, `validate`; machine-readable output | Implemented |
| 2 — Rust static inventory | `syn` parsing, file/type mapping, import/path references and diagnostics | Initial implementation; macros/workspaces are not exhaustive |
| 3 — Dependency graph + manifest | L1 usage, conservative L2 import-only, shortest reverse impacts, source fingerprints, deterministic regenerated YAML | Initial implementation |
| 4 — Non-destructive scaffolds | Failing unit/interaction placeholders; centralized mock templates; generated Cargo integration entrypoints | Implemented for public integration-test surfaces |
| 5 — Read-only impact selection | Stale-index errors; inclusive impact level; expected versus existing tests; explicit workflow/E2E registrations | Implemented |
| 6 — Validation and cross-platform adoption | Rust CLI fixture tests, GitHub Actions on Windows/Linux, Rust consumer examples, honest unsupported-feature notes | Basic CI validation; larger consumer, performance and macro/trait fixture matrices remain open |
| **7 — Agent consumption (last phase)** | Standalone Rust-project setup instructions and **three separate packaged skills** for Claude, ChatGPT/Codex and Antigravity | Implemented as three independent folders; real consumer activation still requires installation |

## Phase 7 artifacts

The final phase is kept separate because it documents **how to apply MATMA to other Rust applications**, rather than implementing MATMA itself:

1. General setup and CLI semantics: [`../README.md`](../README.md) and [`../integrations/README.md`](../integrations/README.md).
2. Claude Code: [`../integrations/claude/matma-rust/`](../integrations/claude/matma-rust/) — install into `.claude/skills/matma-rust/`.
3. ChatGPT/Codex: [`../integrations/chatgpt/matma-rust/`](../integrations/chatgpt/matma-rust/) — install into `.agents/skills/matma-rust-chatgpt/`.
4. Google Antigravity: [`../integrations/antigravity/matma-rust/`](../integrations/antigravity/matma-rust/) — install into `.agents/skills/matma-rust-antigravity/`.

Each bundle has its own `SKILL.md`, `README.md`, and CLI reference. The Codex bundle also has `agents/openai.yaml`. They are independently portable and do not contain credentials. The skills instruct an agent to preserve MATMA's strict tier boundaries, stop when tests are unfinished or an index is stale, and **run tests explicitly** rather than claim an impact query executed them.

## Outstanding acceptance before merge/release

- Owner accepts or revises the proposed Rust-specific treatment of trait-only files and the private API strategy.
- Expand coverage for cfg, macros, modules/reexports, workspace crate roots, injectable trait behavior, and Rust-native test runners as needed for production apps.
- Confirm a real consumer's complete index → scaffold → impact → authored test → explicit Cargo verification workflow; execute live-service regression only in an approved environment.
- Document measured benchmark results instead of promising an asymptotic runtime speedup. Manifest lookup is bounded, but source hashing and directory metadata I/O remain proportional to indexed sources.
- Review library dependencies and packaging; obtain explicit approval for any crates.io publication, tag, merge, or deployment.
