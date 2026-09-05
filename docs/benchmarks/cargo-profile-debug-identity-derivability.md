# Cargo profile debug identity derivability

## Status

The reviewed Cargo `schema-enum-drift/toml-debug-info-spellings` case currently contains an identity contradiction between its allowed analyzer inputs and its reviewed golden contract ID. This note records the contradiction so production analysis is not weakened to fit benchmark truth.

## Exact source evidence

The specimen is pinned to `rust-lang/cargo@2ceefa0090080354b80cc2f5415039bdb0d2bf0b`.

The Rust representation exposes the path mechanically as:

```text
TomlManifest.profile -> TomlProfiles -> TomlProfile.debug -> TomlDebugInfo
```

`TomlProfile` has `#[serde(default, rename_all = "kebab-case")]`, but the field is literally `debug`, so its serialized consumer key remains `debug`.

The generated JSON Schema exposes the corresponding path mechanically as:

```text
properties.profile -> $defs.TomlProfiles -> additionalProperties -> $defs.TomlProfile -> properties.debug -> $defs.TomlDebugInfo
```

The schema property is literally `debug`. The exact token `debug-info` is not present in either reviewed fixture file.

Both representations therefore support the same explicit consumer-facing semantic path:

```text
profile.debug
```

They also independently expose the local type identity `TomlDebugInfo`, which is useful alignment evidence but is not itself a consumer key named `debug-info`.

## Reviewed golden

The unchanged golden requires:

```text
cargo.profile.debug-info
```

and derives all representation, clause, relationship, and finding IDs from that contract ID.

## Reproduced analyzer behavior

On Chirograph work head `bd8ceedaec50b3ea0c7b676397efbae67cbea60d`, the deterministic pipeline emits exactly one Cargo contract after generic transparent-newtype traversal and manual `Deserialize` string-vocabulary extraction are enabled.

CI run `33999005131` shows:

- contract inflation improves from `0.000000` to `1.000000`;
- contract precision becomes `0.000000`;
- contract recall remains `0.000000`;
- false-contract rate becomes `1.000000`.

All formatting, workspace check, Clippy, unit/integration tests, and CLI build pass before the benchmark rejects the emitted contract. This is consistent with an exact identity mismatch rather than missing acquisition or execution.

## Planning error

The implementation plan's Task 4 synthetic example assumes a Rust field named `debug_info` under `serde(rename_all = "kebab-case")`, which mechanically serializes to `debug-info`. The reviewed Cargo fixture instead uses the field `debug`. The planned Cargo acceptance therefore relied on a source shape that is not present in the pinned specimen.

## Epistemic boundary

The approved design requires stable contract IDs to derive from explicit source context plus an evidenced semantic path. It also forbids benchmark/golden access, repository-specific recognition, token-similarity promotion, fuzzy matching, and guessed identity.

Under that contract, rewriting `TomlDebugInfo` into `debug-info`, stripping a `Toml` prefix, or otherwise replacing the explicit terminal consumer key `debug` merely because the golden says `debug-info` would be an unsupported semantic promotion.

The scorer may use golden truth to evaluate output, but production `chirograph analyze` may not use that truth to manufacture the output identity.

## Consequence

With the current fixture, golden, and deterministic identity contract all held fixed, the Cargo acceptance condition `contract recall > 0 with no false-contract regression` is not mechanically satisfiable.

Do not solve this by adding a Cargo recognizer, a `TomlDebugInfo` naming heuristic, benchmark-path inspection, a fuzzy scorer alias, or hidden golden-derived metadata.

A later benchmark/architecture decision must choose one explicit resolution, for example:

1. editorially revise benchmark truth so the contract identity follows the evidenced consumer path `cargo.profile.debug`; or
2. add a separate explicit provenance-bearing source for editorial logical-contract identity, and revise the benchmark contract to require that source rather than expecting the deterministic analyzer to infer it.

This transition does not make either change. `benchmark/baseline.json` and the Cargo golden remain unchanged.
