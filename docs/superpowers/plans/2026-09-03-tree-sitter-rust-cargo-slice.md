# Tree-sitter Rust + Cargo Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one reusable Tree-sitter acquisition substrate, build a generic Rust source adapter on it, and prove the path against Cargo's real historical `manifest.schema.json` enum-drift defect at exact upstream revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`.

**Architecture:** `chirograph-core` stays language-agnostic. A new `chirograph-tree-sitter` crate owns parser lifecycle, exact source provenance, spans, deterministic traversal, and parse diagnostics; a new `chirograph-rust` crate owns Rust grammar and Rust-specific fact extraction. The Cargo benchmark vendors exact upstream source bytes and maps generic acquired facts plus structured JSON observations into ordinary `chirograph-evidence-v1`; live retrieval is explicitly outside benchmark scoring.

**Tech Stack:** Rust 2024 / MSRV 1.85, `tree-sitter = =0.26.13`, `tree-sitter-rust = =0.24.2`, existing `serde`/`serde_json`, existing Chirograph core and CLI. `tree-sitter` 0.26.13 declares Rust 1.77, so it remains below the workspace MSRV.

**Spec:** `docs/superpowers/specs/2026-09-03-tree-sitter-adapter-architecture-design.md`

## Global Constraints

- Chirograph remains read-only with respect to analyzed repositories.
- `chirograph-core` must not depend on Tree-sitter and must not gain Rust- or Cargo-specific concepts.
- Acquisition says what source contains; it does not assign contract truth, authority, `supports`, or `contradicts` stance.
- The shared crate accepts source bytes plus provenance from its caller; it performs no Git operations, repository retrieval, filesystem discovery, or network access.
- Exact repository revision and exact byte/row/column source locations must survive acquisition.
- An adapter must never convert parser uncertainty into a stronger semantic observation.
- Unsupported semantics are omitted rather than guessed. No macro expansion, name resolution, trait solving, build evaluation, or compiler-equivalent type inference is in scope.
- JSON/YAML/TOML remain on semantic structured-data parsers by default; the Cargo generated schema side uses `serde_json`, not Tree-sitter.
- Benchmark inputs are pinned and vendored. Fetching upstream source at runtime is a separate demo concern and is not part of benchmark scoring.
- Use TDD and commit after each independently reviewable task.

## File Map

### New shared acquisition crate

- `crates/chirograph-tree-sitter/Cargo.toml` — exact Tree-sitter dependency and dependency on generic Chirograph model types.
- `crates/chirograph-tree-sitter/src/lib.rs` — public exports only.
- `crates/chirograph-tree-sitter/src/provenance.rs` — `SourceProvenance`, `SourcePoint`, `SourceSpan`.
- `crates/chirograph-tree-sitter/src/parse.rs` — parser lifecycle, owned parsed source, diagnostics, node text/span helpers, deterministic node walk.
- `crates/chirograph-tree-sitter/tests/parse.rs` — exact spans, malformed-source diagnostics, explicit revision behavior, traversal order.
- `crates/chirograph-tree-sitter/README.md` — boundary and failure semantics.

### New Rust adapter crate

- `crates/chirograph-rust/Cargo.toml` — shared substrate plus exact Rust grammar.
- `crates/chirograph-rust/src/lib.rs` — `extract_rust_facts` public entrypoint.
- `crates/chirograph-rust/src/fact.rs` — typed `RustFact` and `RustFactKind` only.
- `crates/chirograph-rust/src/extract.rs` — Rust syntax-tree traversal and safe source-local associations.
- `crates/chirograph-rust/tests/extract.rs` — tiny syntax fixtures and exact-span assertions.
- `crates/chirograph-rust/README.md` — supported v0 facts and explicit non-goals.

### First benchmark specimen

- `benchmarks/cargo/schema-enum-drift/README.md` — defect provenance, logical clause, why it is useful.
- `benchmarks/cargo/schema-enum-drift/case.json` — exact Cargo revision, source paths, byte hashes, expected source/schema values, and benchmark-only stance mapping.
- `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs` — vendored exact Rust source from Cargo revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`.
- `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json` — vendored exact generated schema from the same revision.
- `crates/chirograph-rust/tests/cargo_schema_enum_drift.rs` — dynamically acquire Rust facts, parse structured JSON, build ordinary evidence, and require `CONTESTED`.

### Workspace

- `Cargo.toml` — add the two crates as workspace members; do not add Tree-sitter to `chirograph-core`.

---

### Task 1: Shared Tree-sitter acquisition substrate

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/chirograph-tree-sitter/Cargo.toml`
- Create: `crates/chirograph-tree-sitter/src/lib.rs`
- Create: `crates/chirograph-tree-sitter/src/provenance.rs`
- Create: `crates/chirograph-tree-sitter/src/parse.rs`
- Create: `crates/chirograph-tree-sitter/tests/parse.rs`
- Create: `crates/chirograph-tree-sitter/README.md`

**Interfaces:**
- Consumes: `chirograph_core::{Revision, SourceId}` and a caller-supplied `tree_sitter::Language` plus UTF-8 source bytes.
- Produces:

```rust
pub struct SourceProvenance {
    pub source: SourceId,
    pub revision: Revision,
    pub locator: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePoint {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start: SourcePoint,
    pub end: SourcePoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseDiagnosticKind {
    ErrorNode,
    MissingNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub kind: ParseDiagnosticKind,
    pub span: SourceSpan,
}

pub struct ParsedSource {
    /* owned UTF-8 source + Tree + provenance + sorted diagnostics */
}

pub fn parse_utf8(
    language: &tree_sitter::Language,
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<ParsedSource, ParseError>;

impl ParsedSource {
    pub fn provenance(&self) -> &SourceProvenance;
    pub fn source(&self) -> &str;
    pub fn tree(&self) -> &tree_sitter::Tree;
    pub fn diagnostics(&self) -> &[ParseDiagnostic];
    pub fn span(&self, node: tree_sitter::Node<'_>) -> SourceSpan;
    pub fn text(&self, node: tree_sitter::Node<'_>) -> &str;
    pub fn preorder(&self) -> Vec<tree_sitter::Node<'_>>;
}
```

`preorder()` must be deterministic Tree-sitter child-index preorder. Diagnostics must be sorted by `(start_byte, end_byte, kind)` and deduplicated.

- [ ] **Step 1: Add a failing exact-provenance/span test**

Create `crates/chirograph-tree-sitter/tests/parse.rs` with a dev-only Rust grammar so the shared crate can be tested without making Rust grammar part of its production API:

```rust
use chirograph_core::{Revision, SourceId};
use chirograph_tree_sitter::{parse_utf8, SourceProvenance};

fn provenance(revision: Revision) -> SourceProvenance {
    SourceProvenance {
        source: SourceId::new("fixture.repo").unwrap(),
        revision,
        locator: "github:fixture/repo".into(),
        path: "src/lib.rs".into(),
    }
}

#[test]
fn preserves_exact_revision_and_source_coordinates() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(
        &language,
        b"fn alpha() {}\n",
        provenance(Revision::Exact("0123456789abcdef0123456789abcdef01234567".into())),
    )
    .unwrap();

    assert_eq!(
        parsed.provenance().revision,
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into())
    );
    let function = parsed
        .preorder()
        .into_iter()
        .find(|node| node.kind() == "function_item")
        .unwrap();
    let span = parsed.span(function);
    assert_eq!((span.start_byte, span.end_byte), (0, 13));
    assert_eq!((span.start.row, span.start.column), (0, 0));
    assert_eq!((span.end.row, span.end.column), (0, 13));
    assert_eq!(parsed.text(function), "fn alpha() {}");
}
```

- [ ] **Step 2: Run the test to verify RED**

Run:

```sh
cargo test -p chirograph-tree-sitter --test parse preserves_exact_revision_and_source_coordinates
```

Expected: Cargo reports that workspace package `chirograph-tree-sitter` does not exist.

- [ ] **Step 3: Add the workspace member and minimal crate manifests**

Update root `Cargo.toml` members to include `crates/chirograph-tree-sitter`. Create:

```toml
# crates/chirograph-tree-sitter/Cargo.toml
[package]
name = "chirograph-tree-sitter"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
repository.workspace = true

[dependencies]
chirograph-core = { path = "../chirograph-core" }
tree-sitter = "=0.26.13"

[dev-dependencies]
tree-sitter-rust = "=0.24.2"

[lints.rust]
unsafe_code = "forbid"
```

Do not modify `crates/chirograph-core/Cargo.toml`.

- [ ] **Step 4: Implement provenance value types**

Create `src/provenance.rs` with the exact public types above and a single conversion helper:

```rust
impl From<tree_sitter::Point> for SourcePoint {
    fn from(value: tree_sitter::Point) -> Self {
        Self { row: value.row, column: value.column }
    }
}

pub(crate) fn span_of(node: tree_sitter::Node<'_>) -> SourceSpan {
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start: node.start_position().into(),
        end: node.end_position().into(),
    }
}
```

- [ ] **Step 5: Implement minimal owned parsing**

Create `src/parse.rs`. `parse_utf8` must:

1. reject invalid UTF-8 before parser invocation;
2. call `Parser::set_language(language)` and surface ABI mismatch as a typed `ParseError`;
3. parse once with no prior tree;
4. fail if Tree-sitter returns `None`;
5. own the UTF-8 `String`, `Tree`, provenance, and diagnostics;
6. collect `ERROR` and missing nodes by deterministic preorder.

The core implementation shape is:

```rust
pub fn parse_utf8(
    language: &Language,
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<ParsedSource, ParseError> {
    let source = std::str::from_utf8(source)
        .map_err(|error| ParseError::InvalidUtf8(error.valid_up_to()))?
        .to_owned();
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .map_err(|error| ParseError::Language(error.to_string()))?;
    let tree = parser.parse(&source, None).ok_or(ParseError::NoTree)?;
    let diagnostics = collect_diagnostics(tree.root_node());
    Ok(ParsedSource { source, tree, provenance, diagnostics })
}
```

`ParseError` must implement `Display` and `std::error::Error` without adding another error dependency.

- [ ] **Step 6: Verify the exact-provenance/span test turns GREEN**

Run the Step 2 command.

Expected: PASS.

- [ ] **Step 7: Add malformed-source, unknown-revision, and deterministic-order tests**

Add three tests:

```rust
#[test]
fn malformed_regions_are_reported_without_inventing_clean_parse_state() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(&language, b"fn broken( {", provenance(Revision::Unknown)).unwrap();
    assert!(!parsed.diagnostics().is_empty());
}

#[test]
fn unknown_revision_stays_unknown() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(&language, b"const X: u8 = 1;", provenance(Revision::Unknown)).unwrap();
    assert_eq!(parsed.provenance().revision, Revision::Unknown);
}

#[test]
fn preorder_is_stable_and_source_order_preserving() {
    let language = tree_sitter_rust::LANGUAGE.into();
    let parsed = parse_utf8(&language, b"fn b() {}\nfn a() {}\n", provenance(Revision::Unknown)).unwrap();
    let starts: Vec<_> = parsed.preorder().into_iter().map(|node| node.start_byte()).collect();
    assert!(starts.windows(2).all(|pair| pair[0] <= pair[1]));
}
```

- [ ] **Step 8: Run substrate tests and workspace checks**

Run:

```sh
cargo test -p chirograph-tree-sitter
cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 9: Document the substrate boundary**

`crates/chirograph-tree-sitter/README.md` must state that callers supply bytes and provenance, parsing is read-only, exactness is never inferred, malformed regions produce diagnostics, and no contract stance/authority is assigned here.

- [ ] **Step 10: Commit Task 1**

```sh
git add Cargo.toml crates/chirograph-tree-sitter
git commit -m "feat: add shared Tree-sitter acquisition substrate"
```

---

### Task 2: Generic Rust source-fact adapter

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/chirograph-rust/Cargo.toml`
- Create: `crates/chirograph-rust/src/lib.rs`
- Create: `crates/chirograph-rust/src/fact.rs`
- Create: `crates/chirograph-rust/src/extract.rs`
- Create: `crates/chirograph-rust/tests/extract.rs`
- Create: `crates/chirograph-rust/README.md`

**Interfaces:**
- Consumes: `parse_utf8`, `SourceProvenance`, and `SourceSpan` from Task 1; `tree_sitter_rust::LANGUAGE`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustFactKind {
    Module,
    Struct,
    Enum,
    Variant,
    Trait,
    Impl,
    Function,
    Method,
    Field,
    Const,
    Static,
    TypeExpression,
    Attribute,
    Call,
    MacroCall,
    If,
    Match,
    MatchArm,
    Return,
    Comment,
    Assertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFact {
    pub kind: RustFactKind,
    pub name: Option<String>,
    pub text: String,
    pub container: Vec<String>,
    pub span: SourceSpan,
    pub provenance: SourceProvenance,
}

pub struct RustExtraction {
    pub facts: Vec<RustFact>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

pub fn extract_rust_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<RustExtraction, RustAdapterError>;
```

Facts are sorted by `(start_byte, end_byte, kind, name)` and deduplicated by the same identity plus `text`. `RustAdapterError` wraps only acquisition failures; malformed subregions remain diagnostics, not guessed facts.

- [ ] **Step 1: Write a failing Rust fact extraction test**

Create a fixture inline in `tests/extract.rs` that exercises the acceptance-critical constructs plus representative declarations:

```rust
const SOURCE: &str = r#"
#[derive(Debug)]
pub enum Mode {
    None,
    Full,
}

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Full => "full",
        }
    }
}

#[test]
fn labels() {
    assert_eq!(Mode::Full.label(), "full");
}
"#;

#[test]
fn extracts_declarations_variants_match_arms_and_assertions() {
    let extraction = extract_rust_facts(SOURCE.as_bytes(), exact_fixture_provenance()).unwrap();
    assert!(extraction.facts.iter().any(|fact| fact.kind == RustFactKind::Enum && fact.name.as_deref() == Some("Mode")));
    assert!(extraction.facts.iter().any(|fact| fact.kind == RustFactKind::Variant && fact.name.as_deref() == Some("None")));
    assert!(extraction.facts.iter().any(|fact| fact.kind == RustFactKind::MatchArm && fact.text.contains("Self::None => \"none\"")));
    assert!(extraction.facts.iter().any(|fact| fact.kind == RustFactKind::Assertion && fact.text.starts_with("assert_eq!")));
    assert!(extraction.facts.iter().all(|fact| fact.provenance == exact_fixture_provenance()));
}
```

- [ ] **Step 2: Run the test to verify RED**

Run:

```sh
cargo test -p chirograph-rust --test extract extracts_declarations_variants_match_arms_and_assertions
```

Expected: package `chirograph-rust` does not exist.

- [ ] **Step 3: Add the Rust crate and exact grammar dependency**

Add `crates/chirograph-rust` to root workspace members. Create:

```toml
# crates/chirograph-rust/Cargo.toml
[package]
name = "chirograph-rust"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
repository.workspace = true

[dependencies]
chirograph-core = { path = "../chirograph-core" }
chirograph-tree-sitter = { path = "../chirograph-tree-sitter" }
tree-sitter = "=0.26.13"
tree-sitter-rust = "=0.24.2"
serde_json = "1"

[lints.rust]
unsafe_code = "forbid"
```

`serde_json` is used only by the Cargo acceptance test unless production code later proves it necessary; if Cargo permits it as a dev dependency after tests are written, keep it in `[dev-dependencies]` instead.

- [ ] **Step 4: Implement the typed fact model**

Create `src/fact.rs` with the exact public types above. `SourceProvenance` must derive/implement `Clone + PartialEq + Eq` in Task 1 so facts can preserve it verbatim.

- [ ] **Step 5: Implement source-local extraction with an explicit node-kind table**

`src/extract.rs` must traverse the parsed tree once. Use these Tree-sitter Rust node kinds as the v0 structural vocabulary:

```rust
fn direct_kind(node: tree_sitter::Node<'_>, has_impl_ancestor: bool) -> Option<RustFactKind> {
    match node.kind() {
        "mod_item" => Some(RustFactKind::Module),
        "struct_item" => Some(RustFactKind::Struct),
        "enum_item" => Some(RustFactKind::Enum),
        "enum_variant" => Some(RustFactKind::Variant),
        "trait_item" => Some(RustFactKind::Trait),
        "impl_item" => Some(RustFactKind::Impl),
        "function_item" if has_impl_ancestor => Some(RustFactKind::Method),
        "function_item" => Some(RustFactKind::Function),
        "field_declaration" => Some(RustFactKind::Field),
        "const_item" => Some(RustFactKind::Const),
        "static_item" => Some(RustFactKind::Static),
        "attribute_item" => Some(RustFactKind::Attribute),
        "call_expression" => Some(RustFactKind::Call),
        "macro_invocation" => Some(RustFactKind::MacroCall),
        "if_expression" => Some(RustFactKind::If),
        "match_expression" => Some(RustFactKind::Match),
        "match_arm" => Some(RustFactKind::MatchArm),
        "return_expression" => Some(RustFactKind::Return),
        "line_comment" | "block_comment" => Some(RustFactKind::Comment),
        _ => None,
    }
}
```

Also emit `TypeExpression` facts for named `type` fields on declarations where Tree-sitter exposes the field directly. Do not resolve aliases or inferred types.

For names, read only explicit syntax-tree fields such as `child_by_field_name("name")`. For container paths, maintain a stack only for named declarations (`mod`, `struct`, `enum`, `trait`, `impl`, `function`/method) whose names are syntactically available. An anonymous `impl` may contribute the literal source text of its `type` field as a container segment, but no name resolution is allowed.

Treat an `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`, `debug_assert_eq!`, or `debug_assert_ne!` `macro_invocation` as both `MacroCall` and `Assertion`; the assertion classification is purely lexical on the macro path.

Do not emit facts whose node itself is missing/error or has an error ancestor between the node and the nearest clean declaration boundary.

- [ ] **Step 6: Verify the acceptance-critical extraction test turns GREEN**

Run the Step 2 command.

Expected: PASS.

- [ ] **Step 7: Add coverage for the rest of the v0 fact boundary**

Add focused tests for:

- module/struct/trait/impl/function/method;
- field/const/static and explicit type expressions;
- attributes;
- calls and macro calls;
- `if`, `match`, `match_arm`, `return`;
- comments/doc comments as source facts;
- malformed source diagnostics with no fact emitted from the malformed node;
- deterministic ordering across two runs;
- exact byte and row/column spans for at least one multibyte UTF-8 source fixture.

For deterministic ordering, require full structural equality:

```rust
let left = extract_rust_facts(SOURCE.as_bytes(), provenance.clone()).unwrap();
let right = extract_rust_facts(SOURCE.as_bytes(), provenance).unwrap();
assert_eq!(left.facts, right.facts);
assert_eq!(left.diagnostics, right.diagnostics);
```

- [ ] **Step 8: Run Rust adapter tests and full workspace checks**

Run:

```sh
cargo test -p chirograph-rust
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 9: Document supported facts and non-goals**

`crates/chirograph-rust/README.md` must list the exact `RustFactKind` variants and explicitly state: no macro expansion, name resolution, trait solving, cross-file type inference, Cargo-specific symbol knowledge, or clause stance inference.

- [ ] **Step 10: Commit Task 2**

```sh
git add Cargo.toml crates/chirograph-rust
git commit -m "feat: add generic Rust Tree-sitter adapter"
```

---

### Task 3: Cargo `schema-enum-drift` benchmark specimen

**Files:**
- Create: `benchmarks/cargo/schema-enum-drift/README.md`
- Create: `benchmarks/cargo/schema-enum-drift/case.json`
- Create: `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs`
- Create: `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json`
- Create: `crates/chirograph-rust/tests/cargo_schema_enum_drift.rs`

**Interfaces:**
- Consumes: `extract_rust_facts`, `serde_json`, and `chirograph_core::evidence::parse_evidence_json` / ordinary core clause assessment.
- Produces: one deterministic benchmark case proving that the exact historical Cargo source and generated schema disagree over the string spellings accepted for `profile.*.debug` / `TomlDebugInfo`.

The case is pinned to:

```text
repository: rust-lang/cargo
revision:   2ceefa0090080354b80cc2f5415039bdb0d2bf0b
Rust path:  crates/cargo-util-schemas/src/manifest/mod.rs
JSON path:  crates/cargo-util-schemas/manifest.schema.json
upstream issue: rust-lang/cargo#17201
```

The benchmark logical clause is:

```text
profile debug string values use the manifest spellings:
none, line-directives-only, line-tables-only, limited, full
```

The source-side serializer/deserializer logic supports those spellings. At this exact bad revision the generated JSON schema exposes Rust-style enum spellings such as `None`, `LineDirectivesOnly`, `LineTablesOnly`, `Limited`, and `Full`, so the benchmark must remain `CONTESTED` until evaluated against a fixed upstream revision.

- [ ] **Step 1: Vendor the exact two upstream files before writing the test**

Retrieve both files specifically at `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`; do not copy from Cargo HEAD. Preserve bytes exactly under the `sources/` paths above.

Immediately record their SHA-256 values with:

```sh
sha256sum \
  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs \
  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json
```

Put the resulting 64-hex digests into `case.json`. The test will verify those hashes before analysis, so accidental fixture edits fail before semantic assertions.

- [ ] **Step 2: Create the benchmark metadata contract**

Create `case.json` with this shape, replacing the two `sha256` values only with the exact command output from Step 1:

```json
{
  "schema": "chirograph-benchmark-case-v1",
  "id": "cargo.schema-enum-drift",
  "repository": "rust-lang/cargo",
  "revision": "2ceefa0090080354b80cc2f5415039bdb0d2bf0b",
  "clause": {
    "id": "cargo.profile-debug.string-values",
    "facet": "structural",
    "text": "profile debug string values use none, line-directives-only, line-tables-only, limited, full"
  },
  "sources": [
    {
      "kind": "rust",
      "path": "crates/cargo-util-schemas/src/manifest/mod.rs",
      "sha256": "<exact 64-hex digest produced in Step 1>"
    },
    {
      "kind": "json-schema",
      "path": "crates/cargo-util-schemas/manifest.schema.json",
      "sha256": "<exact 64-hex digest produced in Step 1>"
    }
  ],
  "expected": {
    "source_strings": ["none", "line-directives-only", "line-tables-only", "limited", "full"],
    "schema_strings": ["None", "LineDirectivesOnly", "LineTablesOnly", "Limited", "Full"],
    "assessment": "contested"
  }
}
```

The angle-bracket text is not committed literally; the implementation step substitutes the two deterministic digests before the file is added to git.

- [ ] **Step 3: Write the failing Cargo acceptance test**

`cargo_schema_enum_drift.rs` must first verify fixture hash/revision metadata, then parse the Rust source dynamically through `extract_rust_facts` and the schema through `serde_json`.

The source observation must be derived from generic `MatchArm`/string-literal-containing facts, not hard-coded as a hand-authored Rust observation. The schema observation must come from the schema JSON definition for `TomlDebugInfo`.

Core assertions before evidence construction:

```rust
assert_eq!(source_strings, vec![
    "none",
    "line-directives-only",
    "line-tables-only",
    "limited",
    "full",
]);
assert_eq!(schema_strings, vec![
    "None",
    "LineDirectivesOnly",
    "LineTablesOnly",
    "Limited",
    "Full",
]);
assert_ne!(source_strings, schema_strings);
```

Then build an ordinary `chirograph-evidence-v1` document with:

- one repository source at exact revision `2ceefa...`;
- one Rust source representation;
- one generated-schema representation;
- one structural clause `cargo.profile-debug.string-values`;
- a source-backed observation supporting the clause;
- a schema-backed observation contradicting the clause;
- no authority claim selecting a winner.

Parse it with `parse_evidence_json`, assess the clause through the existing core API, and require `Contested` plus the exact supporting and contradicting representation IDs.

- [ ] **Step 4: Run the Cargo acceptance test to verify RED**

Run:

```sh
cargo test -p chirograph-rust --test cargo_schema_enum_drift -- --nocapture
```

Expected before the benchmark mapping/evidence helper is complete: FAIL because the expected source/schema observations are not yet converted into a valid evidence graph and assessed.

A failure because the generic Rust adapter cannot acquire the required match-arm/string facts is also a valid RED signal; fix the generic adapter, not with Cargo-specific branches.

- [ ] **Step 5: Add the smallest benchmark-local evidence mapping**

Keep Cargo knowledge in this test/case layer. The mapping may select generic Rust facts by `TomlDebugInfo` containment and exact manifest string literals and may select the JSON schema definition by JSON Pointer/key traversal. It must not add `TomlDebugInfo`, `profile.debug`, Cargo symbol names, or stance logic to `chirograph-rust` or `chirograph-core`.

Use stable representation IDs such as:

```text
cargo.rust.toml-debug-info
cargo.generated.manifest-schema
```

and observation IDs derived from the case ID plus side, for example:

```text
cargo.schema-enum-drift.source
cargo.schema-enum-drift.generated-schema
```

The benchmark case explicitly supplies which side supports and contradicts the logical clause; the language adapter does not.

- [ ] **Step 6: Verify GREEN and inspect the contested result**

Run:

```sh
cargo test -p chirograph-rust --test cargo_schema_enum_drift -- --nocapture
cargo run -p chirograph-cli -- inspect <generated-test-evidence-path-if-the-test-persists-one>
```

If the test constructs evidence entirely in memory, the CLI command is optional; in that case print/render the ordinary evidence document in a temporary test artifact only when `CHIROGRAPH_KEEP_BENCHMARK_OUTPUT=1` is set. Default tests must not dirty the repository.

Expected: the test reports/assesses `CONTESTED`, with the Rust representation on the supporting side and the generated schema on the contradicting side.

- [ ] **Step 7: Document why the specimen is real and bounded**

`benchmarks/cargo/schema-enum-drift/README.md` must record the exact revision, upstream issue `rust-lang/cargo#17201`, the two source paths, the logical clause, expected disagreement, and this boundary:

> The benchmark measures Chirograph's acquisition and cross-representation analysis against pinned bytes. Fetching Cargo at runtime is a separate demo capability and is not scored here.

- [ ] **Step 8: Run the full repository verification suite**

Run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
node --test adapters/overcenter/convert.test.mjs
```

Expected: all pass, including `cargo_schema_enum_drift`.

- [ ] **Step 9: Commit Task 3**

```sh
git add benchmarks/cargo/schema-enum-drift crates/chirograph-rust/tests/cargo_schema_enum_drift.rs
git commit -m "test: add Cargo schema enum drift benchmark"
```

---

## Completion Evidence

The slice is complete only when the final worker reports all of the following exact evidence:

1. Chirograph commit SHA containing the completed slice.
2. Exact dependency versions resolved for `tree-sitter` and `tree-sitter-rust`.
3. Exact Cargo upstream revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`.
4. SHA-256 digests of both vendored Cargo source files and a passing test that verifies them.
5. Passing output for every command in Task 3 Step 8.
6. `cargo_schema_enum_drift` result showing `CONTESTED` with exact supporter and contradictor representation IDs.
7. Confirmation that `crates/chirograph-core/Cargo.toml` contains no Tree-sitter dependency and core source contains no Cargo/Rust-specific branch added for this slice.

## Follow-on Work Explicitly Deferred

After this slice is verified, amend/advance independent transitions for:

1. migrating the already-planned Java adapter onto `chirograph-tree-sitter`;
2. migrating the already-planned Python adapter onto `chirograph-tree-sitter`;
3. Go + Kubernetes/Temporal;
4. Ruby + Rails;
5. C++ + Envoy/Arrow;
6. TypeScript/TSX + Overcenter;
7. Protocol Buffers + Kubernetes/Envoy.

Do not fold those languages into this implementation plan. The acceptance criterion for the shared substrate is that adding Go later does not require copying parser lifecycle, provenance, span, or diagnostic machinery from Rust.
