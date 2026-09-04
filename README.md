# Chirograph

Chirograph discovers and explains the logical contracts software actually exposes: what representations exist, how they relate, where they disagree, and what evidence supports each conclusion.

The core model keeps source-backed observations separate from derived interpretations. Logical contracts can span structural, executable, semantic, failure, concurrency, recovery, and verification facets. Atomic contract clauses can be supported or contradicted by concrete representations, letting Chirograph expose drift without choosing truth by file count or heuristic consensus.

Chirograph is early-stage software. Its evidence format is explicitly versioned, but interfaces may still change before 1.0.

## Documentation

- [`docs/model.md`](docs/model.md) — concepts, facets, invariants, and the epistemic boundary
- [`docs/guarantees.md`](docs/guarantees.md) — what Chirograph does and does not establish
- [`docs/query-api.md`](docs/query-api.md) — deterministic semantic queries, CLI surfaces, evidence closure, and non-guarantees
- [`docs/evidence-interchange.md`](docs/evidence-interchange.md) — `chirograph-evidence-v1` and compatibility rules
- [`docs/alignment-interchange.md`](docs/alignment-interchange.md) — `chirograph-alignments-v1` for pre-alignment claims
- [`docs/adapters.md`](docs/adapters.md) — requirements for general-purpose acquisition adapters
- [`docs/benchmarks.md`](docs/benchmarks.md) — benchmark structure, scoring, and ground-truth rules
- [`docs/licensing.md`](docs/licensing.md) — project licensing and third-party specimen provenance

## Workspace

- `crates/chirograph-core` — language-agnostic contract graph and analysis core
- `crates/chirograph-cli` — `chirograph` command-line executable
- `adapters/overcenter` — adapter for Overcenter's existing contract-evidence catalog
- `adapters/pydantic` — external specimen exercising perspective-sensitive contract analysis

## External specimens

External repositories are evidence sources and test specimens, not places to hide repository-specific production logic. Third-party material retains its upstream copyright and license terms. Benchmark cases must pin provenance and content identity as described in [`docs/licensing.md`](docs/licensing.md) and [`docs/benchmarks.md`](docs/benchmarks.md).

The current Pydantic specimen observes an exact upstream release revision and feeds public validation, serialization, JSON Schema, annotation, computed-field, and runtime-output observations into ordinary `chirograph-evidence-v1`. It demonstrates that intentionally different validation and serialization perspectives can remain consistent while a stale schema contradicting the same validation-input clause becomes contested.

See [`adapters/pydantic/README.md`](adapters/pydantic/README.md).

## Development

The repository uses the stable Rust toolchain with the 2024 edition.

```sh
cargo check --workspace --all-targets
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python tools/check_release_metadata.py
```

Run the CLI with:

```sh
cargo run -p chirograph-cli -- --help
cargo run -p chirograph-cli -- contestations evidence.json
cargo run -p chirograph-cli -- evidence evidence.json review-status
cargo run -p chirograph-cli -- authority evidence.json review-status semantic
cargo run -p chirograph-cli -- alignment evidence.json alignments.json candidate-example
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Contributions use Developer Certificate of Origin sign-off and are submitted under the project's Apache-2.0 terms.

## License

Chirograph-authored source code, documentation, schemas, adapters, and benchmark infrastructure are licensed under the [Apache License 2.0](LICENSE) unless a file or accompanying provenance record says otherwise. Third-party specimens are not relicensed by inclusion in this repository.
