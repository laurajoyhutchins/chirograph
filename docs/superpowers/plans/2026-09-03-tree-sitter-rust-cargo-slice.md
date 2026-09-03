# Tree-sitter Rust + Cargo Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one reusable Tree-sitter acquisition substrate, build a generic Rust source adapter on it, and prove the path against Cargo's real historical `manifest.schema.json` enum-drift defect at exact upstream revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`.

**Architecture:** `chirograph-core` stays language-agnostic. A new `chirograph-tree-sitter` crate owns parser lifecycle, caller-supplied provenance, exact spans, deterministic traversal, and parse diagnostics; a new `chirograph-rust` crate owns the Rust grammar and Rust-specific source facts. The Cargo benchmark vendors exact upstream bytes and maps generic Rust facts plus structured JSON observations into ordinary `chirograph-evidence-v1`; live source retrieval is explicitly outside benchmark scoring.

**Tech Stack:** Rust 2024 / workspace MSRV 1.85, `tree-sitter = =0.26.13`, `tree-sitter-rust = =0.24.2`, `sha2 = =0.11.0` for fixture verification, existing `serde`/`serde_json`, Chirograph core and CLI. Tree-sitter 0.26.13 declares Rust 1.77; sha2 0.11.0 declares Rust 1.85.

**Spec:** `docs/superpowers/specs/2026-09-03-tree-sitter-adapter-architecture-design.md`

## Global Constraints

- Chirograph remains read-only with respect to analyzed repositories.
- `chirograph-core` must not depend on Tree-sitter or gain Rust/Cargo-specific concepts.
- Acquisition reports source-local facts only. It never assigns contract truth, authority, `supports`, or `contradicts` stance.
- The shared crate accepts bytes plus provenance from the caller. It performs no Git operations, repository discovery, filesystem discovery, or network access.
- Exact revision and exact byte/row/column locations survive acquisition unchanged.
- Parser uncertainty becomes diagnostics, never stronger semantic claims.
- Unsupported semantics are omitted rather than guessed. No macro expansion, name resolution, trait solving, build evaluation, or compiler-equivalent type inference is in scope.
- JSON/YAML/TOML use semantic structured-data parsers by default. The Cargo schema side uses `serde_json`, not Tree-sitter.
- Benchmark bytes are pinned and vendored. Runtime fetching is a demo concern, not a scored benchmark behavior.
- Use TDD and commit after each independently reviewable task.

## File Map

### Shared acquisition crate

- `crates/chirograph-tree-sitter/Cargo.toml` — Tree-sitter dependency and generic core model dependency.
- `crates/chirograph-tree-sitter/src/lib.rs` — public exports only.
- `crates/chirograph-tree-sitter/src/provenance.rs` — provenance and span value types.
- `crates/chirograph-tree-sitter/src/parse.rs` — parser lifecycle, diagnostics, node text/span helpers, deterministic traversal.
- `crates/chirograph-tree-sitter/tests/parse.rs` — parser/provenance contract tests.
- `crates/chirograph-tree-sitter/README.md` — shared boundary and failure semantics.

### Rust adapter crate

- `crates/chirograph-rust/Cargo.toml` — shared substrate plus exact Rust grammar.
- `crates/chirograph-rust/src/lib.rs` — `extract_rust_facts` entrypoint.
- `crates/chirograph-rust/src/fact.rs` — typed Rust facts.
- `crates/chirograph-rust/src/extract.rs` — source-local Rust extraction.
- `crates/chirograph-rust/tests/extract.rs` — tiny syntax fixtures and exact-span tests.
- `crates/chirograph-rust/README.md` — v0 capability and explicit non-goals.

### First formal benchmark specimen

- `benchmarks/cargo/schema-enum-drift/README.md`
- `benchmarks/cargo/schema-enum-drift/case.json`
- `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs`
- `benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json`
- `crates/chirograph-rust/tests/cargo_schema_enum_drift.rs`

### Workspace

- `Cargo.toml` — add the two crates as members only.

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
- Consumes: `chirograph_core::{Revision, SourceId}`, caller-supplied UTF-8 bytes, and a caller-supplied `tree_sitter::Language`.
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseDiagnosticKind {
    ErrorNode,
    MissingNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub kind: ParseDiagnosticKind,
    pub span: SourceSpan,
}

pub struct ParsedSource { /* owned String + Tree + provenance + diagnostics */ }

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

`preorder()` is child-index preorder. Diagnostics sort/deduplicate by `(start_byte, end_byte, kind)`.

- [ ] **Step 1: Write the failing exact-provenance/span test**

Create `crates/chirograph-tree-sitter/tests/parse.rs`:

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
    ).unwrap();

    assert_eq!(parsed.provenance().revision,
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()));
    let function = parsed.preorder().into_iter()
        .find(|node| node.kind() == "function_item").unwrap();
    let span = parsed.span(function);
    assert_eq!((span.start_byte, span.end_byte), (0, 13));
    assert_eq!((span.start.row, span.start.column), (0, 0));
    assert_eq!((span.end.row, span.end.column), (0, 13));
    assert_eq!(parsed.text(function), "fn alpha() {}");
}
```

- [ ] **Step 2: Verify RED**

Run:

```sh
cargo test -p chirograph-tree-sitter --test parse preserves_exact_revision_and_source_coordinates
```

Expected: Cargo reports that package `chirograph-tree-sitter` does not exist.

- [ ] **Step 3: Add the workspace member and manifest**

Add `crates/chirograph-tree-sitter` to root workspace members. Create:

```toml
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

- [ ] **Step 4: Implement provenance/span types**

Create `provenance.rs` with the interface types and:

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

`parse_utf8` must reject invalid UTF-8, surface grammar ABI mismatch, fail if no tree is returned, own its source string/tree/provenance, and collect `ERROR` and missing nodes.

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
    parser.set_language(language)
        .map_err(|error| ParseError::Language(error.to_string()))?;
    let tree = parser.parse(&source, None).ok_or(ParseError::NoTree)?;
    let diagnostics = collect_diagnostics(tree.root_node());
    Ok(ParsedSource { source, tree, provenance, diagnostics })
}
```

Implement `Display` and `std::error::Error` for `ParseError` without another dependency.

- [ ] **Step 6: Verify GREEN for exact provenance/span**

Run the Step 2 command. Expected: PASS.

- [ ] **Step 7: Add failure/determinism tests**

Add tests that require:

```rust
assert!(!parse_utf8(&language, b"fn broken( {", provenance(Revision::Unknown))
    .unwrap().diagnostics().is_empty());

assert_eq!(parse_utf8(&language, b"const X: u8 = 1;", provenance(Revision::Unknown))
    .unwrap().provenance().revision, Revision::Unknown);

let parsed = parse_utf8(&language, b"fn b() {}\nfn a() {}\n", provenance(Revision::Unknown)).unwrap();
let left: Vec<_> = parsed.preorder().into_iter().map(|n| (n.kind().to_owned(), n.start_byte(), n.end_byte())).collect();
let right: Vec<_> = parsed.preorder().into_iter().map(|n| (n.kind().to_owned(), n.start_byte(), n.end_byte())).collect();
assert_eq!(left, right);
```

Also test invalid UTF-8 returns `ParseError::InvalidUtf8` and no parsed facts can be consumed.

- [ ] **Step 8: Run shared-crate verification**

```sh
cargo test -p chirograph-tree-sitter
cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 9: Document and commit Task 1**

README must say: caller supplies bytes/provenance; exactness is never inferred; malformed regions are diagnostics; no filesystem/network/project semantics; no stance/authority.

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
- Consumes: Task 1's `parse_utf8`, `SourceProvenance`, `SourceSpan`, `ParseDiagnostic`; `tree_sitter_rust::LANGUAGE`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustFactKind {
    Module, Struct, Enum, Variant, Trait, Impl, Function, Method,
    Field, Const, Static, TypeExpression, Attribute, Call, MacroCall,
    If, Match, MatchArm, Return, Comment, Assertion,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustExtraction {
    pub facts: Vec<RustFact>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

pub fn extract_rust_facts(
    source: &[u8],
    provenance: SourceProvenance,
) -> Result<RustExtraction, RustAdapterError>;
```

Facts sort by `(start_byte, end_byte, kind, name)` and deduplicate by that identity plus text.

- [ ] **Step 1: Write the failing acceptance-critical extraction test**

Create `tests/extract.rs` with:

```rust
const SOURCE: &str = r#"
#[derive(Debug)]
pub enum Mode { None, Full }

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Full => "full",
        }
    }
}

#[test]
fn labels() { assert_eq!(Mode::Full.label(), "full"); }
"#;

#[test]
fn extracts_declarations_variants_match_arms_and_assertions() {
    let p = exact_fixture_provenance();
    let extraction = extract_rust_facts(SOURCE.as_bytes(), p.clone()).unwrap();
    assert!(extraction.facts.iter().any(|f| f.kind == RustFactKind::Enum && f.name.as_deref() == Some("Mode")));
    assert!(extraction.facts.iter().any(|f| f.kind == RustFactKind::Variant && f.name.as_deref() == Some("None")));
    assert!(extraction.facts.iter().any(|f| f.kind == RustFactKind::MatchArm && f.text.contains("Self::None => \"none\"")));
    assert!(extraction.facts.iter().any(|f| f.kind == RustFactKind::Assertion && f.text.starts_with("assert_eq!")));
    assert!(extraction.facts.iter().all(|f| f.provenance == p));
}
```

- [ ] **Step 2: Verify RED**

```sh
cargo test -p chirograph-rust --test extract extracts_declarations_variants_match_arms_and_assertions
```

Expected: package `chirograph-rust` does not exist.

- [ ] **Step 3: Add Rust adapter member and manifest**

Add `crates/chirograph-rust` to root workspace members. Create:

```toml
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

[dev-dependencies]
serde_json = "1"
sha2 = "=0.11.0"

[lints.rust]
unsafe_code = "forbid"
```

- [ ] **Step 4: Implement `RustFact` value types**

Create `fact.rs` with the exact interface above. Re-export the fact types from `lib.rs`.

- [ ] **Step 5: Implement one deterministic source-local traversal**

`extract.rs` parses once, traverses once, tracks named declaration containers, and maps these syntax node kinds:

```rust
fn direct_kind(node: tree_sitter::Node<'_>, in_impl: bool) -> Option<RustFactKind> {
    match node.kind() {
        "mod_item" => Some(RustFactKind::Module),
        "struct_item" => Some(RustFactKind::Struct),
        "enum_item" => Some(RustFactKind::Enum),
        "enum_variant" => Some(RustFactKind::Variant),
        "trait_item" => Some(RustFactKind::Trait),
        "impl_item" => Some(RustFactKind::Impl),
        "function_item" if in_impl => Some(RustFactKind::Method),
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

Rules:

- Names come only from explicit syntax fields such as `child_by_field_name("name")`.
- `TypeExpression` comes only from explicit `type` fields; do not infer types.
- Container stack includes syntactically named module/struct/enum/trait/function/method declarations. Anonymous impls may use their literal `type` field text as a container segment.
- `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`, `debug_assert_eq!`, `debug_assert_ne!` macro invocations emit both `MacroCall` and `Assertion` facts. This is lexical macro-path classification only.
- Do not emit a fact for an error/missing node or when the candidate node itself depends on a malformed subtree.
- Every fact clones caller-supplied provenance unchanged and receives exact node span/text.

- [ ] **Step 6: Verify GREEN for the acceptance-critical test**

Run the Step 2 command. Expected: PASS.

- [ ] **Step 7: Add v0 boundary tests**

Add focused tests for module/struct/trait/impl/function/method, fields/const/static/type fields, attributes, calls/macros, `if`/`match`/`match_arm`/`return`, comments/doc comments, malformed-source diagnostics, deterministic repeated output, and one multibyte UTF-8 fixture proving byte offsets differ correctly from character counts while row/column values remain Tree-sitter coordinates.

Require deterministic equality:

```rust
let left = extract_rust_facts(SOURCE.as_bytes(), p.clone()).unwrap();
let right = extract_rust_facts(SOURCE.as_bytes(), p).unwrap();
assert_eq!(left, right);
```

- [ ] **Step 8: Run adapter/workspace verification**

```sh
cargo test -p chirograph-rust
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 9: Document and commit Task 2**

README must enumerate `RustFactKind` and explicitly reject macro expansion, name resolution, trait solving, cross-file type inference, Cargo-specific symbol knowledge, and clause stance inference.

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
- Consumes: `extract_rust_facts`, `serde_json`, `sha2::Sha256`, and existing Chirograph evidence parsing/clause assessment.
- Produces: one deterministic benchmark case requiring `CONTESTED` for a real source/generated-schema disagreement.

Pinned upstream authority:

```text
repository: rust-lang/cargo
revision:   2ceefa0090080354b80cc2f5415039bdb0d2bf0b
Rust path:  crates/cargo-util-schemas/src/manifest/mod.rs
Rust SHA-256: fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c
JSON path:  crates/cargo-util-schemas/manifest.schema.json
JSON SHA-256: a8f038d7ef99e69810c5cafd17d340b9aa42f7c9dd01e9ff70fb4205fec2f21e
upstream issue: rust-lang/cargo#17201
```

Benchmark clause:

```text
profile debug string values use the manifest spellings:
none, line-directives-only, line-tables-only, limited, full
```

- [ ] **Step 1: Vendor and verify exact upstream bytes**

Fetch both files only at `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`. Preserve bytes exactly under the benchmark source paths. Run:

```sh
sha256sum \
  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs \
  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json
```

Expected exactly:

```text
fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/src/manifest/mod.rs
a8f038d7ef99e69810c5cafd17d340b9aa42f7c9dd01e9ff70fb4205fec2f21e  benchmarks/cargo/schema-enum-drift/sources/crates/cargo-util-schemas/manifest.schema.json
```

If either digest differs, stop. Do not normalize line endings or fetch HEAD.

- [ ] **Step 2: Create exact benchmark metadata**

Create `case.json` exactly with:

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
      "sha256": "fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c"
    },
    {
      "kind": "json-schema",
      "path": "crates/cargo-util-schemas/manifest.schema.json",
      "sha256": "a8f038d7ef99e69810c5cafd17d340b9aa42f7c9dd01e9ff70fb4205fec2f21e"
    }
  ],
  "expected": {
    "source_strings": ["none", "line-directives-only", "line-tables-only", "limited", "full"],
    "schema_strings": ["None", "LineDirectivesOnly", "LineTablesOnly", "Limited", "Full"],
    "assessment": "contested"
  }
}
```

- [ ] **Step 3: Write the failing acceptance test**

`cargo_schema_enum_drift.rs` must:

1. parse `case.json`;
2. SHA-256 both vendored files and require exact metadata hashes before analysis;
3. call `extract_rust_facts` with `Revision::Exact("2ceefa0090080354b80cc2f5415039bdb0d2bf0b".into())` and the exact Rust path;
4. derive manifest spellings from generic `MatchArm`/literal-containing facts in the `TomlDebugInfo` implementation;
5. parse `manifest.schema.json` with `serde_json` and read generated `TomlDebugInfo` enum values;
6. assert the two lists below;
7. map the source observation to support and generated-schema observation to contradiction only in benchmark code;
8. create ordinary `chirograph-evidence-v1`, parse it through existing core evidence parsing, assess the clause, and require `Contested` with exact supporter/contradictor representation IDs.

```rust
assert_eq!(source_strings, vec![
    "none", "line-directives-only", "line-tables-only", "limited", "full",
]);
assert_eq!(schema_strings, vec![
    "None", "LineDirectivesOnly", "LineTablesOnly", "Limited", "Full",
]);
assert_ne!(source_strings, schema_strings);
```

Use stable representation IDs:

```text
cargo.rust.toml-debug-info
cargo.generated.manifest-schema
```

and observation IDs:

```text
cargo.schema-enum-drift.source
cargo.schema-enum-drift.generated-schema
```

No authority claim selects a winner.

- [ ] **Step 4: Verify RED**

```sh
cargo test -p chirograph-rust --test cargo_schema_enum_drift -- --nocapture
```

Expected: FAIL until dynamic Rust acquisition plus benchmark-local evidence mapping produces a valid contested graph. If the generic adapter lacks a required source-local fact, improve the generic adapter rather than adding Cargo branches.

- [ ] **Step 5: Implement only benchmark-local selection/mapping**

Cargo-specific selection may use `TomlDebugInfo` containment, exact string literals, and JSON key traversal inside this test/case layer. Do not add `TomlDebugInfo`, `profile.debug`, Cargo symbol names, or stance logic to `chirograph-rust`, `chirograph-tree-sitter`, or `chirograph-core`.

- [ ] **Step 6: Verify GREEN**

Run the Step 4 command. Expected: PASS with `CONTESTED`, Rust as supporter, generated schema as contradictor.

If useful for manual inspection, render evidence only to a temporary file when `CHIROGRAPH_KEEP_BENCHMARK_OUTPUT=1`; default tests must not dirty the repository.

- [ ] **Step 7: Document the case**

README must record revision, hashes/paths, upstream issue `rust-lang/cargo#17201`, clause, observed disagreement, and this exact boundary:

> The benchmark measures Chirograph's acquisition and cross-representation analysis against pinned bytes. Fetching Cargo at runtime is a separate demo capability and is not scored here.

- [ ] **Step 8: Run full repository verification**

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

The slice is complete only when the final worker reports:

1. exact Chirograph commit SHA containing the slice;
2. resolved exact `tree-sitter` and `tree-sitter-rust` versions;
3. Cargo revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`;
4. both expected fixture SHA-256 digests and passing in-test verification;
5. passing output for every Task 3 Step 8 command;
6. `cargo_schema_enum_drift` assessed `CONTESTED` with exact supporter and contradictor representation IDs;
7. confirmation that `crates/chirograph-core/Cargo.toml` has no Tree-sitter dependency and no Rust/Cargo-specific branch was added to core.

## Explicitly Deferred Follow-on Work

After this slice is verified, use independent transitions for Java migration, Python migration, Go + Kubernetes/Temporal, Ruby + Rails, C++ + Envoy/Arrow, TypeScript/TSX + Overcenter, and Protocol Buffers + Kubernetes/Envoy. Do not fold those languages into this plan. The shared-substrate acceptance test is that adding Go later does not require copying parser lifecycle, provenance, span, or diagnostic machinery from Rust.
