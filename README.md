# Chirograph

Chirograph is a read-only tool for discovering and explaining logical software contracts across heterogeneous codebases: what representations exist, how they relate, and which source appears to govern them.

The project is intentionally at the bootstrap stage. The Rust workspace currently separates the language-agnostic core from the command-line executable without committing to an extractor API or contract model prematurely.

## Workspace

- `crates/chirograph-core` — language-agnostic analysis core
- `crates/chirograph-cli` — `chirograph` command-line executable

## Development

The repository uses the stable Rust toolchain with the 2024 edition.

```sh
cargo check --workspace --all-targets
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Run the current CLI skeleton with:

```sh
cargo run -p chirograph-cli
```