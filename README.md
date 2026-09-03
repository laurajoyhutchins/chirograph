# Chirograph

Chirograph is a read-only tool for discovering and explaining logical software contracts across heterogeneous codebases: what representations exist, how they relate, where they disagree, and which source appears to govern each part of the contract.

The core model separates source-backed observations from derived interpretations. Logical contracts have structural, executable, semantic, failure, concurrency, recovery, and verification facets. Atomic contract clauses can be supported or contradicted by concrete representations, letting Chirograph expose contract drift without choosing truth by file count or heuristic consensus.

See [`docs/model.md`](docs/model.md) for the model and invariants.

## Workspace

- `crates/chirograph-core` — language-agnostic contract graph and analysis core
- `crates/chirograph-cli` — `chirograph` command-line executable
- `adapters/python` — generic Tree-sitter-backed Python source acquisition
- `adapters/overcenter` — adapter for Overcenter's existing contract-evidence catalog
- `specimens/python_runtime.py` — generic Python executable-observation boundary for specimens
- `specimens/pydantic` — external specimen proving perspective-sensitive contract analysis over the generic Python boundaries

## External specimens

The Pydantic specimen observes exact Chirograph source and upstream Pydantic revisions. Static Python facts come from the generic Tree-sitter adapter; executable validation, serialization, JSON Schema, computed-field, and runtime-output observations come through the generic Python runtime probe boundary. The specimen proves that intentionally different validation and serialization perspectives remain consistent while a stale schema contradicting the same validation-input clause becomes contested.

See [`specimens/pydantic/README.md`](specimens/pydantic/README.md).

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
