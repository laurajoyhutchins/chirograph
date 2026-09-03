# Shared Tree-sitter Adapter Architecture

**Date:** 2026-09-03

## Goal

Give Chirograph one reusable source-acquisition substrate for programming languages parsed by Tree-sitter, then express Java, Python, Rust, Go, Ruby, C++, TypeScript/TSX, and Protocol Buffers as thin language adapters over that substrate.

The architecture must preserve Chirograph's existing epistemic boundary: parsers acquire deterministic, provenance-rich source facts; they do not decide contract truth, authority, or whether an observation supports or contradicts a logical clause.

Rust is the first new acceptance adapter after the shared substrate because it immediately enables the Cargo benchmark and lets Chirograph dogfood the same parser architecture on its own implementation language.

## Current state

The repository currently has a language-agnostic Rust core plus bespoke external adapters for Overcenter and Pydantic. The Overcenter project definition already contains planned generic Tree-sitter Java and Python adapter transitions, but main does not yet contain reusable Tree-sitter infrastructure or first-class Java/Python adapter directories.

The new design therefore needs to do two things at once:

1. prevent Java and Python from becoming independent parsing stacks; and
2. create the stable seam needed for the remaining language adapters.

## Design choice

Use a small shared Rust crate for Tree-sitter acquisition mechanics and keep language semantics in separate adapter crates.

```text
source file
   |
   v
chirograph-tree-sitter
   - parser lifecycle
   - source identity
   - exact revision provenance
   - byte/point spans
   - query execution helpers
   - deterministic fact ordering
   - parse diagnostics
   |
   +--> chirograph-java
   +--> chirograph-python
   +--> chirograph-rust
   +--> chirograph-go
   +--> chirograph-ruby
   +--> chirograph-cpp
   +--> chirograph-typescript
   +--> chirograph-proto
              |
              v
    language-level source facts
              |
              v
  evidence-candidate / adapter layer
              |
              v
     chirograph-evidence-v1
              |
              v
       chirograph-core
```

`chirograph-core` must not depend on Tree-sitter and must not gain language-specific concepts.

## Alternatives considered

### A. Independent adapter crates with direct Tree-sitter use

Each language crate could own parser setup, provenance, spans, diagnostics, query execution, and ordering independently.

This is initially cheap but duplicates exactly the deterministic machinery Chirograph should centralize. Java and Python would quickly become templates copied into Rust and Go with small behavioral differences. Exact source provenance and failure behavior would then be harder to keep identical across languages.

Rejected.

### B. One generic adapter driven entirely by `.scm` query files

A single runtime could load a grammar plus declarative queries and emit generic captures.

This is attractive for syntax-only acquisition, but language differences become awkward once facts need structure such as Rust attributes, Java annotations, Python decorators, Go receiver methods, C++ templates, TypeScript type syntax, or Proto field/options semantics. Forcing all of that through anonymous captures would create a second, weak schema language inside query files.

Rejected as the primary abstraction. Query files remain useful implementation details inside each language adapter.

### C. Shared acquisition substrate plus thin typed language adapters

Centralize parser/provenance mechanics while allowing each language adapter to define a small typed fact model and extraction rules.

This preserves reuse without pretending the languages are structurally identical.

Selected.

## Shared crate boundary

Create a crate conceptually named `chirograph-tree-sitter`.

It owns only mechanics that are identical across languages:

- Tree-sitter parser construction and language installation;
- UTF-8 source loading from bytes supplied by the caller;
- exact repository/source identity supplied by the caller;
- exact revision identity supplied by the caller;
- parse-tree creation;
- root error / missing-node detection and diagnostics;
- byte offsets and row/column points;
- conversion of Tree-sitter nodes into stable Chirograph source spans;
- query compilation/execution helpers where useful;
- deterministic traversal and result ordering;
- small utilities for extracting node text without losing source location.

It does **not** own:

- repository retrieval;
- Git operations;
- network access;
- project-specific symbol knowledge;
- framework-specific behavior;
- contract clauses;
- support/contradiction stance;
- authority ranking;
- cross-file name resolution;
- whole-program type inference.

The crate should accept source bytes plus provenance from the caller rather than silently reading a repository itself. Live source retrieval is a separate demo/acquisition concern and should not be entangled with parsing correctness.

## Common provenance model

Every emitted language fact must carry enough information to trace it back to an exact source location without reconstructing context later.

Minimum provenance:

```text
repository/source identity
exact revision
path
start byte
end byte
start row/column
end row/column
```

Language facts may also carry a containing declaration path when deterministically available, but containment never replaces the exact span.

Unknown revision is permitted only when the caller explicitly supplies unknown provenance. Adapters must not silently label a mutable checkout as exact.

## Parse failure semantics

Parsing is read-only and fail-closed with respect to claims about extracted facts.

- Failure to install a grammar: adapter error, no facts.
- Catastrophic parse failure: adapter error, no facts.
- Tree contains error or missing nodes: return structured diagnostics and only emit facts from regions whose extraction does not depend on malformed nodes.
- Invalid UTF-8 when the adapter requires UTF-8 source: explicit error.
- Unsupported language feature that still parses: omit unsupported semantic fact kinds rather than guessing.

An adapter must never convert parser uncertainty into a stronger semantic observation.

## Language adapter contract

Each language crate owns:

1. the grammar dependency;
2. file-extension / language selection rules;
3. typed language-level fact kinds;
4. Tree-sitter queries or visitors that produce those facts;
5. safe association rules such as comments/docstrings/attributes to declarations;
6. tests showing exact spans and deterministic output;
7. documentation of intentionally unsupported semantics.

A language adapter may use Tree-sitter queries, typed traversal, or both. The abstraction is the emitted fact contract, not the query mechanism.

The v0 fact vocabulary should stay narrow and useful. Common categories include:

- declaration;
- field/constant/variable binding;
- literal or default value;
- type expression;
- attribute/annotation/decorator;
- call;
- return;
- conditional branch;
- throw/raise/panic-like control transfer;
- assertion/test expectation;
- associated documentation/comment.

Not every language must support every category in v0.

## Rust adapter v0

Rust is the first acceptance adapter for the shared substrate.

Target fact coverage:

- modules;
- structs, enums, unions, traits, impl blocks, functions, methods;
- fields and enum variants;
- `const` and `static` declarations with literals/expressions;
- type expressions and generic bounds where source-local;
- attributes and attribute arguments;
- function/method/macro calls when safely identifiable syntactically;
- `if`, `match`, `return`, and explicit `panic!`/`assert!` family uses;
- doc comments and ordinary comments when safely associable;
- test functions and assertions.

The Rust adapter should not attempt macro expansion, name resolution, trait solving, or compiler-equivalent type semantics.

### Cargo acceptance specimen

Use the curated Cargo contract specimen as the first external acceptance case.

Success means Chirograph can mechanically acquire meaningful Rust source evidence that previously would have required hand-authored observations, preserve the exact upstream Cargo revision and source spans, and feed ordinary `chirograph-evidence-v1` without adding Cargo concepts to core.

The benchmark should measure contract-discovery/analysis behavior against a known specimen. Fetching the upstream source at runtime may be demonstrated separately, but live retrieval is not part of the parser benchmark score.

## Java and Python migration

The existing planned Java and Python transitions should depend on or incorporate the shared substrate rather than each introducing its own parser/provenance utilities.

Their language-specific acceptance goals remain intact:

- Java: declarations/constants/literals, calls, conditionals/throws, documentation associations, assertions, Kafka specimen.
- Python: declarations, annotated assignments/types, decorators, calls, returns/raises/conditionals, comments/docstrings, assertions, Pydantic specimen.

The shared crate is not allowed to weaken those acceptance tests. Migration is considered successful only if exact provenance and contested-contract behavior remain unchanged.

## Remaining adapter order

After Rust and migration of Java/Python onto the common substrate:

1. **Go** for Kubernetes and Temporal.
2. **Ruby** for Rails migration/schema/runtime authority cases.
3. **C++** for Envoy and Arrow.
4. **TypeScript/TSX** for Overcenter and TypeScript-heavy projects.
5. **Protocol Buffers** for Kubernetes/Envoy schema authority.

Proto should use a pinned and tested Tree-sitter grammar. Because its grammar ecosystem is less canonical than the primary Tree-sitter language grammars, compatibility tests are required before treating grammar upgrades as routine dependency bumps.

## Structured-data boundary

Do not route JSON, YAML, or TOML through the Tree-sitter adapter by default.

Those formats should use semantic parsers that preserve their native data model. Tree-sitter is appropriate only when a use case specifically requires concrete source trivia or source-position behavior that the semantic parser cannot provide.

Likewise, OpenAPI and generated descriptors should be parsed through their native structured representation first.

## Evidence conversion boundary

Language facts are observations about source syntax and source-local structure. Converting them into `chirograph-evidence-v1` may happen in a generic candidate/evidence adapter layer, but the conversion must preserve this rule:

> Acquisition can say what the source contains. It cannot decide what the logical contract means.

A fact may therefore become an evidence candidate with provenance and normalized search terms without automatically receiving `supports` or `contradicts` stance.

Project-specific acceptance fixtures may supply the final clause mapping needed to test Chirograph's contested/consistent behavior, but that mapping must remain outside the generic language adapters.

## Testing strategy

Use TDD for the shared substrate and every language adapter.

### Shared crate tests

Require:

- exact byte and row/column span preservation;
- deterministic ordering;
- explicit exact/unknown revision handling;
- parse diagnostic behavior for malformed input;
- no hidden file/network access;
- grammar-neutral helper behavior.

### Language unit tests

Use tiny source fixtures that isolate one syntactic fact at a time. Assert both fact content and exact spans.

### Acceptance tests

Use real upstream specimens pinned to exact revisions:

- Rust -> Cargo;
- Java -> Kafka;
- Python -> Pydantic;
- Go -> Kubernetes/Temporal;
- Ruby -> Rails;
- C++ -> Envoy/Arrow;
- TypeScript -> Overcenter or another pinned TS repository;
- Proto -> Kubernetes/Envoy.

Acceptance fixtures should prove useful evidence acquisition, not exhaustive parser coverage.

### CI

At minimum run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Any external specimen checkout used in CI must be exact-revision pinned and deterministic. Large or network-heavy live retrieval should remain outside the default unit-test path unless explicitly justified.

## Overcenter graph shape

Represent this work as small executable transitions rather than one giant "add language support" node.

Recommended dependency structure:

```text
implement-tree-sitter-substrate
          |
          +--> implement-rust-tree-sitter-adapter
          |          |
          |          +--> validate-cargo-rust-specimen
          |
          +--> implement-java-tree-sitter-adapter
          |          |
          |          +--> rank-java-evidence-candidates / Kafka validation
          |
          +--> implement-python-tree-sitter-adapter
          |          |
          |          +--> migrate-pydantic-to-python-specimen
          |
          +--> implement-go-tree-sitter-adapter
          +--> implement-ruby-tree-sitter-adapter
          +--> implement-cpp-tree-sitter-adapter
          +--> implement-typescript-tree-sitter-adapter
          +--> implement-proto-tree-sitter-adapter
```

The individual language adapters can proceed independently after the substrate is verified. Their real-repository acceptance specimens should remain separate transitions so parser implementation and contract-analysis evaluation do not collapse into one opaque unit of work.

## Non-goals

This design does not attempt:

- compiler replacement;
- semantic name resolution across a repository;
- macro expansion;
- build-system evaluation;
- dependency resolution;
- automatic contract truth inference;
- automatic authority selection;
- runtime source retrieval as part of benchmark scoring;
- universal syntax normalization across all languages.

## Success criteria

The architecture is successful when:

1. Java, Python, and Rust share one provenance/parser substrate without sharing language semantics.
2. The Rust adapter can acquire useful evidence from an exact Cargo revision and feed ordinary Chirograph evidence.
3. Adding Go does not require copying parser lifecycle, span, provenance, or diagnostics code from Rust.
4. No Tree-sitter dependency enters `chirograph-core`.
5. Language adapters remain project-agnostic and never assign clause truth or authority.
6. Existing Kafka and Pydantic contested-contract behavior is preserved when those adapters migrate to the shared substrate.
7. The benchmark measures Chirograph's contract-analysis behavior independently from optional live source retrieval.
