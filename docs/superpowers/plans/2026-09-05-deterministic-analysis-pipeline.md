# Deterministic Analysis Pipeline + Cargo Score Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the public `chirograph analyze` path produce its first mechanically justified non-empty semantic graph and improve the reviewed Cargo benchmark from the zero-output baseline without weakening Chirograph's epistemic or benchmark boundaries.

**Architecture:** Keep `chirograph-core` language-agnostic and keep raw parsing in adapters. Add a `chirograph-analysis` crate that owns explicit source context, deterministic source discovery, structured-data observations, representation candidates, conservative provenance-bound identity bridges, and final graph assembly. The first scored slice is deliberately narrow: emit a logical contract only when distinct implementation and schema representations expose the same explicit consumer-facing semantic path and comparable closed value sets, with all evidence bound to exact source provenance. This is a generic cross-representation rule, not a Cargo recognizer. Ambiguous or weak correspondence remains unresolved and produces no semantic graph entity.

**Tech Stack:** Rust 2024 / workspace MSRV 1.85, existing `chirograph-core`, existing `chirograph-tree-sitter`, the generic `chirograph-rust` adapter from the 2026-09-03 plan, `serde`/`serde_json`, and the existing `chirograph-benchmark` public-process runner.

**Spec:** `docs/superpowers/specs/2026-09-05-deterministic-analysis-pipeline-design.md`

**Related plan:** `docs/superpowers/plans/2026-09-03-tree-sitter-rust-cargo-slice.md`

## Non-negotiable boundaries

- `benchmark/` remains data-only. Do not add executable recognizers, mappings, hints, or generated answers under it.
- The analyzer must never read `golden.yaml`, benchmark case/scenario IDs, or parent benchmark directories.
- No production branch may test for Cargo, `rust-lang/cargo`, the Cargo fixture path, `TomlManifest`, `TomlDebugInfo`, or the golden contract ID.
- The scored path is offline, deterministic, model-free, and clock-independent.
- Exact source repository and revision are explicit analyzer inputs. They are provenance, not semantic truth.
- Raw adapters emit source-local facts only. They do not assign contract truth, authority, clause stance, or benchmark meaning.
- Exact-name similarity, token overlap, file proximity, repetition, and majority agreement are never sufficient alignment evidence.
- Unresolved ambiguity stays unresolved and does not become a placeholder contract.
- Authority remains facet-scoped. The first slice does not need to emit an authority claim to count as a successful honest score improvement.
- Prefer zero output over a guessed contract. A score increase is valid only if the full reviewed baseline still passes directionally.
- Do not rewrite `benchmark/baseline.json` merely because the score improves.

## Execution dependency boundary

The new pipeline work depends on generic acquisition that already has separate Overcenter authority and a separate implementation plan. Do not duplicate it here.

1. `implement-tree-sitter-substrate` must be settled first. At the planning revision, `crates/chirograph-tree-sitter` already exists with `parse.rs`, `provenance.rs`, tests, and the expected public exports, while the Overcenter transition is still unconfirmed. Execute that transition as a verification/reconciliation task against Task 1 of the 2026-09-03 plan. If the current code already satisfies the contract, verify and confirm it rather than rewriting it.
2. `implement-rust-tree-sitter-adapter` must then execute Task 2 of the 2026-09-03 plan and produce the generic `chirograph-rust` crate. Do not extend that adapter with Cargo-specific semantics.
3. The tasks below begin only after those two transitions are DONE at the authoritative Chirograph revision.

---

## Task 1: Add explicit analysis source context and deterministic discovery

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/chirograph-analysis/Cargo.toml`
- Create: `crates/chirograph-analysis/src/lib.rs`
- Create: `crates/chirograph-analysis/src/context.rs`
- Create: `crates/chirograph-analysis/src/discovery.rs`
- Create: `crates/chirograph-analysis/tests/context_and_discovery.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisSourceContext {
    pub repository: String, // canonical owner/repo input
    pub source: SourceId,   // mechanically derived github:owner/repo
    pub namespace: String,  // repository-name component only
    pub revision: Revision,
}

impl AnalysisSourceContext {
    pub fn github(repository: &str, revision: Revision) -> Result<Self, AnalysisError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSource {
    pub relative_path: PathBuf,
    pub kind: SourceFileKind,
}

pub fn discover_sources(root: &Path) -> Result<Vec<DiscoveredSource>, AnalysisError>;
```

`SourceFileKind` initially needs only `Rust` and `Json`; unsupported regular files are ignored deterministically. Discovery must never walk above `root`, must not follow a symlink outside `root`, and returns repository-relative paths in stable lexical order.

- [ ] **Step 1: Write failing source-context tests**

Require `github("acme/fixture-project", Revision::Exact(...))` to derive `SourceId("github:acme/fixture-project")` and namespace `fixture-project`. Reject malformed repository identities and empty namespace components.

- [ ] **Step 2: Write failing discovery tests**

Create a temporary source tree in deliberately scrambled creation order. Assert only supported regular files are returned, paths are root-relative and sorted, and a symlink that escapes the root is not traversed.

- [ ] **Step 3: Verify RED**

Run:

```sh
cargo test -p chirograph-analysis --test context_and_discovery
```

Expected: package or symbols do not exist yet.

- [ ] **Step 4: Add the crate and minimal implementation**

Add `crates/chirograph-analysis` to workspace members. Depend on `chirograph-core`, `chirograph-rust`, `serde`, and `serde_json`; do not add Tree-sitter directly. Implement only the context and discovery behavior required by the tests.

- [ ] **Step 5: Verify GREEN**

Run the Step 3 command and require PASS.

---

## Task 2: Model source-backed representation candidates without semantic commitment

**Files:**
- Create: `crates/chirograph-analysis/src/candidate.rs`
- Create: `crates/chirograph-analysis/tests/candidates.rs`
- Modify: `crates/chirograph-analysis/src/lib.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticPath(Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateMechanism {
    RustSerializedField,
    RustClosedValueSet,
    JsonSchemaProperty,
    JsonSchemaClosedValueSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvidence {
    pub source: SourceId,
    pub revision: Revision,
    pub locator: String,
    pub fact: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationCandidate {
    pub kind: RepresentationKind,
    pub qualified_local_identity: String,
    pub locator: String,
    pub facets: BTreeSet<ContractFacet>,
    pub semantic_path: SemanticPath,
    pub closed_values: Option<BTreeSet<String>>,
    pub mechanisms: BTreeSet<CandidateMechanism>,
    pub evidence: Vec<CandidateEvidence>,
}
```

`qualified_local_identity` is the adapter-observed declaration/schema identity inside the supplied source context; it is not a benchmark ID and is not itself sufficient for alignment. `SemanticPath` is an explicit consumer-facing path, not a tokenized symbol name. `facets` records only facets mechanically supported by the candidate evidence. Candidate evidence must be sorted/deduplicated stably, retain exact revision, and use locators precise enough to preserve the originating source span or structured-data location.

- [ ] **Step 1: Write failing invariants**

Test that semantic paths reject empty segments, evidence preserves exact revision, candidate ordering is independent of insertion order, and a candidate cannot claim a closed value set without corresponding mechanism evidence.

- [ ] **Step 2: Verify RED**

```sh
cargo test -p chirograph-analysis --test candidates
```

- [ ] **Step 3: Implement the minimal candidate model**

Keep these pre-graph types in `chirograph-analysis`. Do not widen `chirograph-core::model` and do not mutate `AlignmentCatalog` to infer anything automatically.

- [ ] **Step 4: Verify GREEN**

Run the Step 2 command and require PASS.

---

## Task 3: Acquire generic JSON Schema property/value-set observations

**Files:**
- Create: `crates/chirograph-analysis/src/json_schema.rs`
- Create: `crates/chirograph-analysis/tests/json_schema.rs`
- Modify: `crates/chirograph-analysis/src/lib.rs`

**Behavior:**

Parse JSON semantically with `serde_json`. Walk schema `properties`, local `$defs`/`definitions`, local `$ref`, and `enum` nodes deterministically. Produce schema `RepresentationCandidate`s only for paths whose consumer-facing property path is explicit. Resolve only local-document references that are mechanically unambiguous. Cycles, unsupported references, malformed schemas, or ambiguous structure must produce deterministic diagnostics or a fail-closed error, never guessed paths.

- [ ] **Step 1: Write a synthetic RED test**

Use an invented schema with `profile -> debug-info` and a closed enum such as `["none", "line-tables-only", "full"]`. Assert one candidate has semantic path `profile.debug-info`, kind `Schema`, exact source/revision, and the exact closed set.

- [ ] **Step 2: Add negative tests**

Require no candidate for a property without a closed value set in the first slice. Require a broken local `$ref` to fail explicitly instead of silently dropping evidence.

- [ ] **Step 3: Implement the bounded schema walker**

No repository-specific schema titles, paths, or property names may influence traversal.

- [ ] **Step 4: Verify**

```sh
cargo test -p chirograph-analysis --test json_schema
```

---

## Task 4: Project generic Rust facts into serialized-path/value-set candidates

**Prerequisite:** `chirograph-rust` from the 2026-09-03 plan is authoritative and exposes source-local declarations, fields/type references, serde/schemars attributes, and enum/literal facts with exact spans.

**Files:**
- Create: `crates/chirograph-analysis/src/rust_projection.rs`
- Create: `crates/chirograph-analysis/tests/rust_projection.rs`
- Modify: `crates/chirograph-analysis/src/lib.rs`

**Behavior:**

The analysis layer may compose source-local Rust facts only through explicit, deterministic links. It may unwrap a small documented set of transparent container type forms needed to follow an explicit field type reference, but it must not perform macro expansion, trait solving, import resolution, or compiler-equivalent name resolution. A link is usable only when the referenced declaration is unique in the analyzed candidate set.

Honor explicit `serde(rename = ...)` and `serde(rename_all = ...)` serialization evidence when the generic Rust facts expose it. Derive nested consumer paths only by following unique field/type edges. Derive a closed value set only from a uniquely referenced enum whose serialized spellings are explicit or mechanically determined by its serialization rule.

- [ ] **Step 1: Write a synthetic RED test**

Use invented Rust source with a manifest-like root struct, nested `profile` field, a `debug_info` field under `#[serde(rename_all = "kebab-case")]`, and a closed enum. Assert the projection yields semantic path `profile.debug-info` and the enum's serialized spellings with exact provenance.

- [ ] **Step 2: Write false-alignment tests**

Add two same-named candidate declarations in different scopes and require the type edge to remain unresolved. Add a field with no explicit/derivable serialization rule and require no consumer semantic path. More weak matches must not make either case resolve.

- [ ] **Step 3: Implement only the deterministic projection needed by the tests**

Do not add Cargo symbols or special-case repository/path/name checks.

- [ ] **Step 4: Verify**

```sh
cargo test -p chirograph-analysis --test rust_projection
```

---

## Task 5: Add conservative identity bridging, explicit alignment decisions, and graph assembly

**Files:**
- Create: `crates/chirograph-analysis/src/alignment.rs`
- Create: `crates/chirograph-analysis/src/assemble.rs`
- Create: `crates/chirograph-analysis/tests/alignment.rs`
- Create: `crates/chirograph-analysis/tests/assemble.rs`
- Modify: `crates/chirograph-analysis/src/lib.rs`

The alignment stage is explicit and provenance-bearing. It uses the existing `chirograph_core::alignment::AlignmentState` values rather than silently collapsing candidates into graph membership. Define a stable candidate key from source, revision, representation kind, qualified local identity, and locator, and record each decision with the exact evidence that justified it. `Confirmed` may enter graph assembly; `Unresolved` is retained in the internal analysis result/diagnostics but does not become a graph contract or representation; `Rejected` is emitted only when explicit evidence proves non-identity, not merely because confirmation evidence is absent. The first slice does not need to invent rejected decisions.

**First-slice alignment and promotion rules:**

Identity alignment and contract promotion are separate decisions. Confirm alignment when one Rust implementation candidate and one JSON Schema candidate carry the same exact `SemanticPath`, both paths are backed by their explicit mechanisms (`RustSerializedField` and `JsonSchemaProperty`), the candidate identity on each side is unique for that semantic path, and source/revision provenance is compatible. Equal closed value sets do not make that identity alignment unresolved.

Do not emit every confirmed schema/property pair as a logical contract. For this first score-improving slice, a confirmed aligned pair is eligible for graph contract promotion only when both candidates also carry comparable closed value sets backed by `RustClosedValueSet` and `JsonSchemaClosedValueSet`, and those sets differ. The differing sets provide direct cross-representation contradiction evidence that this aligned field is a contract-worthy invariant for the first slice.

This deliberately narrow generic promotion rule turns a mechanically established cross-representation drift into a semantic contract while avoiding mass promotion of unrelated neighboring types and schema properties. It does not redefine alignment truth to depend on disagreement.

**Stable identities:**

- contract ID: `<source namespace>.<semantic-path>`, for example a repository namespace plus `profile.debug-info`;
- implementation representation ID: `<contract-id>.implementation`;
- schema representation ID: `<contract-id>.schema`.

These identities must be derived from source context, semantic path, and representation role only. They must not use benchmark metadata.

**Graph output:**

Create the minimum valid `ContractGraph` containing source, contract, two representations, and the source observations needed to justify them. It is acceptable for the first slice to omit authority, relationship, clause, and finding claims until their independent evidence rules are implemented. Do not invent those claims merely to improve additional metrics.

- [ ] **Step 1: Write the positive alignment RED test**

Two explicit candidates at the same path with the required mechanisms produce one `Confirmed` alignment decision with deterministic semantic-path identity, candidate keys, structural facet, and evidence closure whether their comparable closed sets are equal or different. The decision must be identical under input permutation.

- [ ] **Step 2: Write unresolved alignment RED tests**

Require `Unresolved` rather than a guessed confirmation for same-name/no-path evidence, one-to-many ambiguous path matches, schema-only evidence, Rust-only evidence, and candidates with mismatched source revisions. Repeating weak candidates must not change `Unresolved` to `Confirmed`.

- [ ] **Step 3: Write assembly RED tests**

Only a confirmed decision that also satisfies the first-slice drift promotion gate may produce graph membership. The differing-value positive case produces exactly one structural contract and exactly two structural representations with deterministic IDs. An equal-value pair remains `Confirmed` alignment internally but does not satisfy this first-slice promotion gate, so graph JSON stays empty. Unresolved decisions produce no placeholder contracts.

- [ ] **Step 4: Write permutation determinism test**

Reverse candidate, decision, and evidence input order and assert canonical `encode_graph_json` bytes are identical.

- [ ] **Step 5: Implement alignment then assembly**

Keep `alignment.rs` responsible for evidence-to-state decisions and `assemble.rs` responsible only for projecting confirmed decisions into `ContractGraph`. Reuse `chirograph-core` constructors, `AlignmentState`, and graph validation. Do not alter scorer identity rules. The first slice assigns only the `Structural` facet because the compared serialized field path and closed value set are structural evidence; it does not infer executable or semantic facets.

- [ ] **Step 6: Verify**

```sh
cargo test -p chirograph-analysis --test alignment
cargo test -p chirograph-analysis --test assemble
```

---

## Task 6: Wire the production analyzer and explicit CLI provenance boundary

**Files:**
- Create: `crates/chirograph-analysis/src/analyze.rs`
- Create: `crates/chirograph-analysis/tests/analyze.rs`
- Modify: `crates/chirograph-cli/Cargo.toml`
- Modify: `crates/chirograph-cli/src/main.rs`
- Modify or create process tests under: `crates/chirograph-cli/tests/`

**Public command:**

```text
chirograph analyze <source-tree> \
  --source-repository <owner/repo> \
  --revision <40-hex|unversioned|unknown> \
  --format graph-json
```

`--source-repository` and `--revision` are ordinary analyzer provenance inputs. The analyzer must not infer them from the filesystem path. Exact 40-hex revisions map to `Revision::Exact`; the two explicit words map to the existing non-exact revision states.

**Pipeline:**

```text
AnalysisSourceContext
        +
 deterministic discovery
        |
        +--> .rs  -> chirograph-rust facts -> Rust candidates
        +--> .json -> JSON Schema observations -> schema candidates
                                              |
                                              v
                                  explicit alignment
                                              |
                                              v
                                     promotion gate
                                              |
                                              v
                                  conservative assembly
                                              |
                                              v
                                      ContractGraph
```

- [ ] **Step 1: Write CLI RED tests**

Require the old path-only invocation to fail with a provenance error rather than silently inventing context. Require malformed repository/revision values to fail. Require valid explicit context to return canonical graph JSON.

- [ ] **Step 2: Write anti-leakage process test**

Copy identical synthetic fixture bytes into two unrelated temporary directory names. Invoke the built public CLI twice with identical explicit source context and assert stdout is byte-identical. Neither temporary path may appear in semantic IDs.

- [ ] **Step 3: Implement `analyze_tree` orchestration and CLI parsing**

Keep `main.rs` thin. Put discovery, acquisition, explicit alignment, promotion, and assembly behavior in `chirograph-analysis`.

- [ ] **Step 4: Verify**

```sh
cargo test -p chirograph-analysis
cargo test -p chirograph-cli
```

---

## Task 7: Pass only generic provenance from the benchmark runner

**Files:**
- Modify: `crates/chirograph-benchmark/src/runner.rs`
- Modify existing runner tests or create: `crates/chirograph-benchmark/tests/runner.rs`

The runner already owns parsed `SpecimenV1`, including `specimen.upstream.repository` and exact `specimen.upstream.revision`. Pass only those two values to the public CLI:

```text
chirograph analyze <fixture-dir>
  --source-repository <specimen.upstream.repository>
  --revision <specimen.upstream.revision>
  --format graph-json
```

Do not pass `specimen.id`, `repository`, `scenario`, golden values, expected contract IDs, authority answers, findings, or non-contracts.

- [ ] **Step 1: Write a runner invocation RED test**

Use a fake analyzer executable/script that records argv and returns valid empty graph JSON. Assert argv contains only fixture path, upstream repository/revision, and public format flags. Assert no golden path or case/scenario identifier is present.

- [ ] **Step 2: Implement provenance forwarding**

Do not add a private adapter API to the benchmark crate.

- [ ] **Step 3: Verify**

```sh
cargo test -p chirograph-benchmark
```

---

## Task 8: Prove the Cargo score improvement without changing reviewed truth

**Files:**
- Do not modify: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/golden.yaml`
- Do not modify: `benchmark/baseline.json`
- Modify docs only if public CLI syntax documentation requires it.

The acceptance case is the existing reviewed specimen at upstream revision `2ceefa0090080354b80cc2f5415039bdb0d2bf0b`. The implementation must discover the relevant serialized semantic path and differing closed value sets from the fixture bytes through the generic Rust and JSON Schema mechanisms above.

- [ ] **Step 1: Build the exact public binary**

```sh
cargo build -p chirograph-cli
```

- [ ] **Step 2: Run the Cargo case against the existing baseline**

```sh
cargo benchmark cargo/schema-enum-drift/toml-debug-info-spellings \
  --baseline benchmark/baseline.json \
  --chirograph-bin target/debug/chirograph
```

Required result: the case remains `scored`, contract recall is greater than `0`, false-contract rate does not increase, and contract inflation does not move farther from `1.0`. Prefer exactly one emitted contract for this first slice.

- [ ] **Step 3: Inspect the public graph directly**

```sh
target/debug/chirograph analyze \
  benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/fixture \
  --source-repository rust-lang/cargo \
  --revision 2ceefa0090080354b80cc2f5415039bdb0d2bf0b \
  --format graph-json
```

Confirm the graph is justified entirely by fixture evidence and explicit source context. No benchmark path or golden value may be required for semantic identity.

- [ ] **Step 4: Run full repository verification**

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo benchmark all --baseline benchmark/baseline.json --chirograph-bin target/debug/chirograph
```

Required result: all commands pass; at least one reviewed higher-is-better metric improves; no reviewed metric regresses under existing directional baseline semantics; no scored case becomes execution failure.

- [ ] **Step 5: Prove reviewed truth was not rewritten**

Verify the final diff leaves `benchmark/baseline.json` and the Cargo `golden.yaml` unchanged. If a score improvement requires changing either file, stop and treat the implementation as invalid.

---

## Self-review checklist before implementation

- [ ] Every production rule is generic across repositories and does not contain Cargo fixture identifiers.
- [ ] `chirograph-core` remains Tree-sitter-free and language-agnostic.
- [ ] The existing Tree-sitter substrate and Rust adapter plan are reused instead of reimplemented.
- [ ] Source identity/revision are explicit public inputs and survive every emitted observation.
- [ ] Candidate construction remains pre-semantic; explicit provenance-bearing alignment decisions sit between candidates and graph assembly.
- [ ] Candidate local identity and supported facets are preserved without treating either as sufficient alignment evidence.
- [ ] Alignment confirmation depends on the exact semantic-path bridge and unique compatible provenance, not on whether the aligned values agree or disagree.
- [ ] The first-slice graph-promotion rule additionally requires comparable closed value sets and direct drift evidence.
- [ ] Same-name, ambiguous, repeated, or majority evidence cannot promote alignment.
- [ ] The analyzer cannot observe benchmark scenario/case identity or golden truth.
- [ ] Moving fixture bytes to another directory cannot change graph identity/output.
- [ ] The plan targets one honest Cargo contract-score improvement first, not broad speculative recall.
- [ ] Full-corpus directional baseline comparison remains the final acceptance gate.

## Completion evidence

Do not claim this implementation complete without fresh evidence for the exact final authoritative revision:

1. exact integrated Git commit SHA;
2. exact PR and final-head CI result;
3. `cargo fmt`, `cargo check`, `cargo clippy`, and `cargo test --workspace` results;
4. exact Cargo benchmark case report showing improvement over the reviewed zero-output baseline;
5. full `cargo benchmark all --baseline benchmark/baseline.json` non-regression result;
6. confirmation that `benchmark/baseline.json` and Cargo `golden.yaml` were unchanged;
7. Overcenter settlement receipt for the implementation transition(s).
