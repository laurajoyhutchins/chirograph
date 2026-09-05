# chirograph-rust

`chirograph-rust` is Chirograph's generic Rust source-acquisition adapter. It parses caller-supplied Rust bytes through the shared `chirograph-tree-sitter` substrate and emits deterministic, provenance-rich source-local facts.

## v0 facts

The adapter can report modules, structs, enums and variants, traits, impl blocks, functions and impl methods, fields, constants, statics, explicit type expressions, attributes, calls, macro invocations, `if` and `match` expressions, match arms, returns, comments, and assertion macro invocations.

Every fact preserves the caller-supplied source identity and revision plus exact Tree-sitter byte/row/column spans. Facts are deterministically ordered and parser uncertainty is returned as diagnostics.

## Epistemic boundary

These are syntax observations, not contract truth. The adapter does not assign logical-contract membership, authority, clause stance, or cross-representation relationships.

The v0 adapter intentionally does **not** perform:

- macro expansion;
- name or import resolution;
- trait solving;
- cross-file type inference;
- Cargo-specific symbol recognition;
- clause support/contradiction inference.

Higher Chirograph layers may combine these facts with other explicit provenance-bearing evidence. Ambiguous semantics remain unresolved rather than being guessed here.
