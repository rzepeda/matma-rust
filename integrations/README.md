# MATMA Rust — three independent agent skills

Install **MATMA's executable separately** (`cargo install --path /path/to/matma-rust`) and run from the Rust project's root. The skills are procedural instructions, not a Rust compiler, test runner, or hosted plugin.

Copy the **contents of one source skill folder** into the corresponding destination:

| Product | Copy from this repository | Destination inside consuming Rust project |
| --- | --- | --- |
| Claude Code | `integrations/claude/matma-rust/` | `.claude/skills/matma-rust/` |
| ChatGPT/Codex | `integrations/chatgpt/matma-rust/` | `.agents/skills/matma-rust-chatgpt/` |
| Antigravity | `integrations/antigravity/matma-rust/` | `.agents/skills/matma-rust-antigravity/` |

Every destination must contain its own `SKILL.md` at the first directory level. ChatGPT/Codex and Antigravity use **separate names** to avoid collisions under `.agents/skills`. The ChatGPT web UI may not automatically discover local project folders; use a supported skill upload/connector or attach these instructions to a Project/Work context as appropriate. Codex recognizes `.agents/skills`. Antigravity also recognizes `.agents/skills`, and Claude Code uses `.claude/skills`.

Each package includes a local `references/commands.md`, so copying only one does not break its references. The installation does not edit production code and does not claim that a skill has been enabled inside a service merely because the files exist here.

## Type-scoped mirroring and impact

For A and B in src/services/order_processing.rs, the unit mirrors are tests/unit/mirror/services/order_processing.rs/test_a.rs and test_b.rs; mock mirrors are tests/mocks/mirror/services/order_processing.rs/mock_a.rs and mock_b.rs. Impact of A must not include B just because the two types share a file. Use affected_symbols and returned test paths for selection. Multi-type files need fully qualified symbol identities in the verification registry.
