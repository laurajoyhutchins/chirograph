# Chirograph

Chirograph is a read-only tool for discovering and explaining logical software contracts across heterogeneous codebases: what representations exist, how they relate, where they disagree, and which source appears to govern each part of the contract.

The core model separates source-backed observations from derived interpretations. Logical contracts have executable, semantic, failure, concurrency, recovery, and verification facets. Atomic contract clauses can be supported or contradicted by concrete representations, letting Chirograph expose contract drift without choosing truth by file count or heuristic consensus.

See [`docs/model.md`](docs/model.md) for the model and invariants.

## Workspace

- `crates/chirograph-core` — language-agnostic contract graph and analysis core
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
