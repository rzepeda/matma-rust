# MATMA Rust consumer acceptance fixture

This is an independent Cargo crate using real, private Rust types. CI invokes the MATMA CLI to scaffold `#[cfg(test)]` bridges, then runs `cargo test` within this crate. The authored tests live only in `tests/{unit,interaction}/mirror/` and use `super::` to access private code. Centralized `MockA` produces a **real** B and provides mutable state. Real A produces real B in an interaction test. The trait contract is tested against real A. The fixture has no external dependencies or live services.
