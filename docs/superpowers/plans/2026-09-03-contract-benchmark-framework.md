# Chirograph Contract Benchmark Implementation Plan

**Goal:** Add a hermetic, data-only contract benchmark framework plus eight curated real-repository cases that score Chirograph's final logical contract graph against human-reviewed golden truth, without adding repository-specific analysis code.

**Architecture:** Add a small `chirograph-benchmark` workspace crate that discovers `benchmark/<repository>/<scenario>/<case>/`, validates specimen provenance and golden truth, invokes the public `chirograph analyze ... --format graph-json` boundary as an external process, scores valid graph output, aggregates results, verifies exact upstream fixture bytes on explicit maintenance commands, and compares against a checked-in regression baseline. Add a canonical graph-JSON projection in `chirograph-core` so the product has one stable machine-readable final-graph representation without making every internal Rust struct layout public. The benchmark must not implement source-to-contract reconstruction itself. Until the public analyzer exists for a case, the case is recorded as an explicit execution-failure baseline; later generic analyzer improvements make cases score without changing benchmark logic.

**Tech Stack:** Rust 2024 workspace, `serde`/`serde_json`, maintained `yaml_serde` 0.10 for typed YAML parsing, `sha2` for local fixture digests, standard-library filesystem/process APIs, Git CLI only for explicit source verification/refresh maintenance operations.

**Design Spec:** `docs/superpowers/specs/2026-09-03-contract-benchmark-design.md`

---

## Scope boundary discovered during planning

The repository currently has `chirograph inspect <evidence.json>` but no general source-tree-to-contract-graph analyzer. The benchmark must not fill that gap with private adapter orchestration or specimen-specific semantics. This plan therefore builds the benchmark and its corpus against the approved public-analysis boundary, but does **not** implement the semantic `analyze` subsystem.

That produces a deliberate first checkpoint:

```text
real fixture -> public chirograph analyze boundary -> execution failure (today)
                                              or -> valid chirograph-graph-v1 -> score
```

The scorer is fully testable with synthetic canonical graphs. All eight real cases land immediately with explicit current execution status. A later, separate public-analysis implementation plan can make them executable using only generic Chirograph capabilities.

Do not weaken this boundary merely to make the initial scoreboard green.

---

## Task 1: Add a canonical final-graph JSON projection

**Files:**
- Create: `crates/chirograph-core/src/graph_json.rs`
- Modify: `crates/chirograph-core/src/lib.rs`
- Test: `crates/chirograph-core/src/graph_json.rs` (`#[cfg(test)]` module)

### Step 1: Write the failing tests

Add tests that build a tiny valid `ContractGraph` containing:

- one source;
- one logical contract;
- two representations;
- one clause with one supporting and one contradicting assertion;
- one relation;
- one facet-scoped authority claim.

Require `graph_json::encode_graph_json(&graph)` to return JSON with:

```json
{
  "schema": "chirograph-graph-v1",
  "contracts": [],
  "representations": [],
  "relations": [],
  "authority_claims": [],
  "clauses": [],
  "clause_assessments": []
}
```

The test must assert deterministic lexical ordering by canonical IDs even when the source `ContractGraph` vectors are deliberately inserted out of order.

Add a second test requiring invalid `ContractGraph` input to fail rather than serialize a graph that violates core invariants.

Run:

```sh
cargo test -p chirograph-core graph_json -- --nocapture
```

Expected: FAIL because `graph_json` does not exist.

### Step 2: Implement explicit DTOs, not blanket `Serialize` derives

Create public projection types in `graph_json.rs` rather than adding `Serialize` to every internal model type.

The stable v1 projection must contain only benchmark/product-facing semantics:

```rust
pub const GRAPH_JSON_SCHEMA: &str = "chirograph-graph-v1";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphJsonV1 {
    pub schema: String,
    pub contracts: Vec<GraphContractV1>,
    pub representations: Vec<GraphRepresentationV1>,
    pub relations: Vec<GraphRelationV1>,
    pub authority_claims: Vec<GraphAuthorityClaimV1>,
    pub clauses: Vec<GraphClauseV1>,
    pub clause_assessments: Vec<GraphClauseAssessmentV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<GraphLifecycleV1>,
}
```

`GraphLifecycleV1` is included as a forward-compatible transport slot but core conversion emits an empty vector until Chirograph has a real lifecycle model. It contains only strings and is never inferred by the encoder:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphLifecycleV1 {
    pub subject: String,
    pub status: String,
}
```

Use explicit string mappings for every current enum (`ContractFacet`, `RepresentationKind`, `RelationKind`, `AuthorityBasis`, `ClauseKind`, `ClauseStatus`) so future Rust enum refactors cannot silently change the wire format.

Represent relation endpoints with typed node refs:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphNodeRefV1 {
    pub kind: String,
    pub id: String,
}
```

Generate `clause_assessments` mechanically by calling `ContractGraph::assess_clause` for every clause after `graph.validate()` succeeds.

Do not include parser facts, candidate ranks, AST nodes, or adapter diagnostics in this schema.

### Step 3: Re-run focused tests

```sh
cargo test -p chirograph-core graph_json -- --nocapture
```

Expected: PASS.

### Step 4: Verify the core remains language-agnostic

```sh
cargo check -p chirograph-core
cargo clippy -p chirograph-core --all-targets --all-features -- -D warnings
```

Expected: PASS, with no Tree-sitter or repository-specific dependency added to `chirograph-core`.

### Step 5: Commit

```sh
git add crates/chirograph-core/src/graph_json.rs crates/chirograph-core/src/lib.rs
git commit -m "feat: add canonical contract graph json"
```

---

## Task 2: Create the benchmark crate and typed file formats

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/chirograph-benchmark/Cargo.toml`
- Create: `crates/chirograph-benchmark/src/lib.rs`
- Create: `crates/chirograph-benchmark/src/main.rs`
- Create: `crates/chirograph-benchmark/src/model.rs`
- Test: `crates/chirograph-benchmark/src/model.rs`

### Step 1: Write failing typed-format tests

Define tests that parse the following minimal specimen shape:

```yaml
schema: chirograph-benchmark-specimen-v1
id: cargo/schema-enum-drift/toml-debug-info-spellings
repository: cargo
scenario: schema-enum-drift
upstream:
  repository: rust-lang/cargo
  revision: 2ceefa0090080354b80cc2f5415039bdb0d2bf0b
files:
  - fixture_path: fixture/crates/cargo-util-schemas/src/manifest/mod.rs
    upstream_path: crates/cargo-util-schemas/src/manifest/mod.rs
    sha256: fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c
```

Define a golden format with product vocabulary plus benchmark-only evaluation truth:

```yaml
schema: chirograph-benchmark-golden-v1
contracts:
  - id: cargo.profile.debug-info
    facets: [structural]
representations:
  - id: cargo.profile.debug-info.implementation
    contract: cargo.profile.debug-info
    kind: source-code
    locator: crates/cargo-util-schemas/src/manifest/mod.rs
    facets: [structural]
authority_claims:
  - contract: cargo.profile.debug-info
    facet: structural
    representation: cargo.profile.debug-info.implementation
relationships: []
lifecycle: []
expected_findings: []
non_contracts: []
```

Tests must reject:

- unknown schema versions;
- empty IDs;
- malformed exact revisions;
- malformed SHA-256 strings;
- duplicate contract IDs;
- authority claims referencing unknown contracts or representations;
- relationships referencing unknown nodes;
- lifecycle subjects that refer to no golden contract/representation;
- expected contested-clause findings that reference no known clause.

Run:

```sh
cargo test -p chirograph-benchmark model -- --nocapture
```

Expected: FAIL because the crate does not exist.

### Step 2: Add workspace member and dependencies

Add `crates/chirograph-benchmark` to the root workspace.

Use:

```toml
[dependencies]
chirograph-core = { path = "../chirograph-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
yaml_serde = "0.10"
sha2 = "0.10"
```

`yaml_serde` is used only for strongly typed benchmark files. Do not use a dynamic YAML `Value` tree.

### Step 3: Implement strict v1 data types

In `model.rs`, define:

- `SpecimenV1`;
- `UpstreamV1`;
- `FixtureFileV1`;
- `GoldenV1`;
- golden contract/representation/authority/relationship/clause/lifecycle/finding/non-contract structs;
- `CaseStatus`;
- `CaseMetrics`;
- `CaseResult`;
- `BenchmarkReportV1`;
- `BenchmarkBaselineV1`.

Golden relationship endpoints must use the same typed `contract`/`representation` node-ref concept as `chirograph-graph-v1`.

Lifecycle truth is intentionally stored as an exact non-empty status string because core has no lifecycle vocabulary yet. A current run reports lifecycle correctness as `null` when observed graph output contains no lifecycle facts. Do not invent lifecycle in product code to satisfy the benchmark.

Expected findings v1 supports one mechanically scoreable kind:

```yaml
kind: contested-clause
clause: cargo.profile.debug-info.spelling
```

Additional finding kinds require an explicit future schema change.

### Step 4: Re-run tests

```sh
cargo test -p chirograph-benchmark model -- --nocapture
```

Expected: PASS.

### Step 5: Commit

```sh
git add Cargo.toml crates/chirograph-benchmark
git commit -m "feat: add benchmark data model"
```

---

## Task 3: Implement corpus discovery and fail-closed self-validation

**Files:**
- Create: `crates/chirograph-benchmark/src/corpus.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Test: `crates/chirograph-benchmark/tests/corpus.rs`

### Step 1: Write failing filesystem tests

In temporary directories, create cases at exactly three identity dimensions:

```text
benchmark/cargo/schema-enum-drift/case-a/
benchmark/cargo/schema-enum-drift/case-b/
benchmark/kafka/message-spec-generation/case-c/
```

Require discovery to return canonical IDs in lexical order.

Add rejection tests for:

- metadata ID does not equal relative path;
- `repository` or `scenario` metadata disagrees with path;
- missing `specimen.yaml` or `golden.yaml`;
- undeclared file under `fixture/`;
- declared fixture missing from disk;
- local fixture SHA-256 mismatch;
- benchmark implementation files outside `fixture/` (for example `observe.py`, `extract.rs`, `run.sh` next to `golden.yaml`);
- nested fourth identity directory that is not under `fixture/`;
- duplicate canonical IDs;
- golden internal-reference failure.

Important: source-code extensions such as `.rs`, `.java`, `.go`, `.rb`, `.cc`, `.proto`, `.sql`, `.cql`, `.py`, `.json`, `.fbs`, and generated source are legal **inside `fixture/`**. They are inert input, not executable benchmark glue.

Run:

```sh
cargo test -p chirograph-benchmark --test corpus -- --nocapture
```

Expected: FAIL.

### Step 2: Implement fixed-depth discovery

Do not add a generic recursive crawler dependency. The layout is fixed:

```text
benchmark/<repository>/<scenario>/<case>/
```

Use `std::fs::read_dir` for exactly those three dimensions, then validate the case directory.

### Step 3: Implement local byte validation

Compute SHA-256 over fixture bytes, not decoded text. Require every file under `fixture/` to be declared exactly once in `specimen.yaml`, and every declaration to resolve beneath that case's `fixture/` directory after path normalization.

Reject absolute paths and `..` escapes.

### Step 4: Re-run tests

```sh
cargo test -p chirograph-benchmark --test corpus -- --nocapture
```

Expected: PASS.

### Step 5: Commit

```sh
git add crates/chirograph-benchmark/src/corpus.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/tests/corpus.rs
git commit -m "feat: validate benchmark corpus"
```

---

## Task 4: Add multidimensional selectors and the ergonomic Cargo command

**Files:**
- Create: `crates/chirograph-benchmark/src/selector.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Create: `.cargo/config.toml`
- Test: `crates/chirograph-benchmark/tests/selectors.rs`

### Step 1: Write failing selector tests

Given cases across multiple repositories/scenarios, require:

```text
all
cargo
scenario:schema-enum-drift
cargo/schema-enum-drift
cargo/schema-enum-drift/toml-debug-info-spellings
```

Selectors return case IDs in lexical order.

Reject ambiguous/unknown selectors rather than silently returning zero cases.

### Step 2: Implement selectors

Rules:

- `all`: every case;
- one path-free token: repository;
- `scenario:<name>`: every matching scenario;
- two path components: repository/scenario intersection;
- three path components: exact case.

Do not introduce freeform tag selection in v1.

### Step 3: Add CLI parser and Cargo alias

Use a simple hand-written CLI parser; do not add a large argument-parser dependency for this small surface.

Support:

```text
cargo benchmark --list
cargo benchmark all
cargo benchmark cargo
cargo benchmark scenario:schema-enum-drift
cargo benchmark cargo/schema-enum-drift
cargo benchmark cargo/schema-enum-drift/toml-debug-info-spellings
```

Create:

```toml
[alias]
benchmark = "run --quiet -p chirograph-benchmark --"
```

### Step 4: Verify

```sh
cargo test -p chirograph-benchmark --test selectors -- --nocapture
cargo benchmark --help
```

Expected: PASS and useful help text.

### Step 5: Commit

```sh
git add .cargo/config.toml crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/src/selector.rs crates/chirograph-benchmark/tests/selectors.rs
git commit -m "feat: add benchmark selectors"
```

---

## Task 5: Implement strict graph scoring

**Files:**
- Create: `crates/chirograph-benchmark/src/score.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Test: `crates/chirograph-benchmark/tests/score.rs`

### Step 1: Write failing metric tests first

Use synthetic `chirograph-graph-v1` values, not real repositories.

Cover at least:

1. perfect contract match;
2. one missing golden contract;
3. one unmatched emitted contract;
4. zero emitted contracts;
5. facet authority 2/3 correct;
6. correct relation endpoints with wrong relation kind;
7. missing relationship;
8. one correct and one false relationship;
9. lifecycle exact match when lifecycle exists;
10. lifecycle `None`/not-scorable when observed graph has no lifecycle facts;
11. expected contested clause found;
12. expected contested clause missed;
13. unexpected contested clause emitted;
14. contract inflation below, equal to, and above `1.0`.

### Step 2: Implement exact identity matching

Logical contract matching is exact ID set membership only. No token similarity, edit distance, aliases, or fuzzy rescue.

Compute:

```text
contract precision = TP / emitted
contract recall    = TP / golden
contract F1        = harmonic mean
false contract rate = FP / emitted
contract inflation = emitted / golden
```

Use explicit zero-denominator rules:

- emitted=0 and golden>0: precision `1.0` only in the mathematical vacuous sense is misleading; report precision `null`, recall `0.0`, false rate `null`, inflation `0.0`;
- golden=0 is invalid for an initial benchmark case and rejected by corpus validation.

This avoids manufacturing a perfect precision score from silence.

### Step 3: Implement authority scoring per facet

Compare exact triples:

```text
(contract_id, facet, representation_id)
```

Authority basis is reported in mismatch diagnostics but not part of the v1 correctness numerator, because the approved metric is authority selection correctness.

### Step 4: Implement relationship scoring

Compare exact typed edges:

```text
(from_kind, from_id, relation_kind, to_kind, to_id)
```

Report precision and recall.

### Step 5: Implement lifecycle and finding scoring

Lifecycle:

- if golden has no lifecycle expectations, metric is `null`;
- if golden has lifecycle expectations and observed graph has no lifecycle facts, metric is `null` plus diagnostic `lifecycle_not_observed` rather than pretending zero-confidence inference is an incorrect classification;
- when observed lifecycle exists, compare exact `(subject,status)` pairs.

Findings:

- golden `contested-clause` maps to observed `clause_assessments.status == "contested"`;
- report finding precision and recall independently of contract reconstruction.

### Step 6: Keep `unclassified-but-real` out of current-run exemptions

The scorer never accepts a flag from observed output that exempts an unmatched contract. Any unmatched emitted contract is false in the current run.

Corpus history can later record human adjudications separately; no code in `score.rs` may let the analyzer self-certify surprises.

### Step 7: Verify

```sh
cargo test -p chirograph-benchmark --test score -- --nocapture
```

Expected: PASS.

### Step 8: Commit

```sh
git add crates/chirograph-benchmark/src/score.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/tests/score.rs
git commit -m "feat: score contract benchmark graphs"
```

---

## Task 6: Implement macro/micro aggregation and deterministic reports

**Files:**
- Create: `crates/chirograph-benchmark/src/aggregate.rs`
- Create: `crates/chirograph-benchmark/src/report.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Test: `crates/chirograph-benchmark/tests/report.rs`

### Step 1: Write failing aggregation tests

Create synthetic results where one case has 100 contracts and another has 1 contract so macro and micro differ materially.

Require reports at:

- case;
- repository;
- scenario;
- overall corpus.

Macro must average per-case metric values and skip `null` metrics rather than coercing them to zero.

Micro must recompute from pooled counts, not average percentages.

### Step 2: Implement metric counts alongside ratios

Keep internal count data for contracts/relationships/findings so micro aggregation is exact.

Do not attempt to micro-average authority or lifecycle when their denominator semantics are unavailable; aggregate their scored numerators/denominators directly when present.

### Step 3: Implement human and JSON reports

Human output headline order:

```text
case/repository/scenario
contract P/R/F1
false-rate
inflation
ower authority
relations P/R
lifecycle
findings P/R
status/diagnostics
```

Correct the label to `authority` in implementation; the deliberately misspelled line above is a test-planning reminder that report snapshot tests should catch presentation drift.

Machine output schema:

```text
chirograph-benchmark-report-v1
```

Use deterministic lexical ordering and stable decimal formatting.

### Step 4: Verify

```sh
cargo test -p chirograph-benchmark --test report -- --nocapture
```

Expected: PASS.

### Step 5: Commit

```sh
git add crates/chirograph-benchmark/src/aggregate.rs crates/chirograph-benchmark/src/report.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/tests/report.rs
git commit -m "feat: report benchmark aggregates"
```

---

## Task 7: Invoke the public Chirograph analysis boundary and classify failures

**Files:**
- Create: `crates/chirograph-benchmark/src/runner.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Test: `crates/chirograph-benchmark/tests/runner.rs`

### Step 1: Write failing fake-binary tests

Create temporary executable scripts/programs for four behaviors:

1. exit nonzero;
2. exit zero with non-JSON stdout;
3. exit zero with wrong graph schema;
4. exit zero with valid `chirograph-graph-v1`.

Require classifications:

```text
nonzero child        -> execution-failure
invalid graph output -> invalid-output
valid graph          -> scored
```

Do not parse stderr text to decide semantics. Preserve bounded stderr as diagnostics only.

### Step 2: Implement binary resolution

Resolution order:

1. explicit `--chirograph-bin PATH`;
2. `CHIROGRAPH_BIN` environment variable;
3. repository-local `target/debug/chirograph` (or platform `.exe`).

If the default binary does not exist, run exactly:

```sh
cargo build --quiet -p chirograph-cli
```

from the workspace root, then resolve the built binary.

This is build orchestration only. The benchmark must never invoke adapter-specific commands.

### Step 3: Invoke the public contract

For each case, invoke exactly:

```text
chirograph analyze <absolute-case-fixture-directory> --format graph-json
```

No language flag, repository-specific symbol map, fixture hint, or custom evidence input is allowed.

Today this command is expected to exit nonzero because the semantic analyzer does not yet exist. That is an honest benchmark execution failure.

### Step 4: Validate successful output

On exit zero:

- parse `GraphJsonV1`;
- require `schema == chirograph-graph-v1`;
- reject duplicate IDs or dangling graph references before scoring;
- then call the scorer.

### Step 5: Verify

```sh
cargo test -p chirograph-benchmark --test runner -- --nocapture
```

Expected: PASS.

### Step 6: Commit

```sh
git add crates/chirograph-benchmark/src/runner.rs crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/tests/runner.rs
git commit -m "feat: run public analysis for benchmarks"
```

---

## Task 8: Add exact upstream source verification and refresh maintenance

**Files:**
- Create: `crates/chirograph-benchmark/src/source.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Test: `crates/chirograph-benchmark/tests/source.rs`

### Step 1: Write failing transport-independent tests

Define an internal `SourceFetcher` trait used only by maintenance code. Test with an in-memory fake fetcher that returns exact bytes for `(repository, revision, path)`.

Require:

- `verify_sources` detects one mismatched remote byte sequence;
- `verify_sources` does not modify files;
- refresh replaces fixture bytes and updates the corresponding SHA-256;
- refresh updates the specimen revision only when an explicit 40-character `--revision` is supplied;
- refresh never reads or writes `golden.yaml`;
- refresh rejects selectors resolving to cases from multiple upstream repositories when one `--revision` would be ambiguous.

### Step 2: Implement generic Git fetcher

Production maintenance fetching may use the installed Git CLI, not a repository-specific API.

For one `(owner/repo, revision)` group:

1. create a temporary directory;
2. `git init`;
3. `git remote add origin https://github.com/<owner>/<repo>.git`;
4. `git fetch --depth=1 origin <exact-revision>`;
5. for each upstream path, read exact bytes with `git show FETCH_HEAD:<path>`;
6. delete the temporary directory.

Fail closed if Git cannot fetch the exact revision or any path.

No benchmark scoring path invokes Git or the network.

### Step 3: Add maintenance commands

Support:

```text
cargo benchmark --verify-sources
cargo benchmark --verify-sources cargo
cargo benchmark --refresh cargo/schema-enum-drift/toml-debug-info-spellings --revision 2ceefa0090080354b80cc2f5415039bdb0d2bf0b
```

For `--refresh`, require the revision argument even when it equals the currently pinned revision. Never infer HEAD or a branch tip.

### Step 4: Verify

```sh
cargo test -p chirograph-benchmark --test source -- --nocapture
```

Expected: PASS without network access because tests use the fake fetcher.

### Step 5: Commit

```sh
git add crates/chirograph-benchmark/src/source.rs crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/tests/source.rs
git commit -m "feat: verify benchmark source provenance"
```

---

## Task 9: Curate Cargo `schema-enum-drift`

**Files:**
- Create: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/specimen.yaml`
- Create: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/golden.yaml`
- Create verbatim fixtures under: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/fixture/`

### Step 1: Copy exact upstream files

Use upstream `rust-lang/cargo` revision:

```text
2ceefa0090080354b80cc2f5415039bdb0d2bf0b
```

Copy verbatim:

```text
crates/cargo-util-schemas/src/manifest/mod.rs
crates/cargo-util-schemas/manifest.schema.json
```

Require the already-established SHA-256 values:

```text
crates/cargo-util-schemas/src/manifest/mod.rs
fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c

crates/cargo-util-schemas/manifest.schema.json
a8f038d7ef99e69810c5cafd17d340b9aa42f7c9dd01e9ff70fb4205fec2f21e
```

### Step 2: Establish golden truth manually

At minimum encode:

- logical contract `cargo.profile.debug-info`;
- Rust implementation/type representation;
- generated JSON Schema representation;
- structural authority on the implementation/source definition rather than the stale generated manifestation;
- relationship from authoritative source representation to generated schema manifestation;
- one clause for the accepted TOML/string spellings;
- expected contested-clause finding for the known enum/spelling drift;
- explicit non-contract examples for nearby implementation-only declarations that a naive syntax enumerator might promote.

Do **not** add Cargo-specific code to compute the stance.

### Step 3: Validate corpus/provenance

```sh
cargo benchmark --list
cargo benchmark --verify-sources cargo/schema-enum-drift/toml-debug-info-spellings
```

Expected: case discovered; source verification PASS.

### Step 4: Record current execution status

```sh
cargo benchmark cargo/schema-enum-drift/toml-debug-info-spellings
```

Expected today: execution failure until the public analyzer exists. Do not work around it.

### Step 5: Commit

```sh
git add benchmark/cargo
git commit -m "test: add Cargo contract benchmark case"
```

---

## Task 10: Curate Kafka `message-spec-generation`

**Files:**
- Create: `benchmark/kafka/message-spec-generation/produce-request-data/specimen.yaml`
- Create: `benchmark/kafka/message-spec-generation/produce-request-data/golden.yaml`
- Create verbatim fixtures under: `benchmark/kafka/message-spec-generation/produce-request-data/fixture/`

### Step 1: Pin exact upstream revision

Use `apache/kafka` revision:

```text
b57cf6e56eb59a952db7236b4da67cc2fdbb8cdf
```

### Step 2: Copy the minimum complete files

Copy verbatim:

```text
clients/src/main/resources/common/message/ProduceRequest.json
generator/src/main/java/org/apache/kafka/message/MessageGenerator.java
generator/src/main/java/org/apache/kafka/message/MessageSpec.java
generator/src/main/java/org/apache/kafka/message/MessageDataGenerator.java
clients/src/main/java/org/apache/kafka/common/requests/ProduceRequest.java
```

Compute SHA-256 for each copied file and write the actual 64-hex digest into `specimen.yaml` in the same commit.

### Step 3: Establish golden truth

At minimum encode:

- logical contract `kafka.protocol.produce-request`;
- `ProduceRequest.json` as the schema/spec representation and structural authority;
- message-generator classes as generation machinery, not independent logical contracts;
- `ProduceRequest.java` as an implementation/wrapper manifestation that consumes the generated data type;
- typed generation/projection relationships;
- non-contract truth for helper methods/classes that are mechanically present but not logical contracts.

The benchmark should penalize an analyzer that reports every generator class or JSON field as a top-level logical contract.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources kafka/message-spec-generation/produce-request-data
cargo benchmark kafka/message-spec-generation/produce-request-data
```

Expected today: provenance PASS; benchmark execution may remain an execution failure until general Java/structured-data analysis is available.

### Step 5: Commit

```sh
git add benchmark/kafka
git commit -m "test: add Kafka contract benchmark case"
```

---

## Task 11: Curate Kubernetes `go-protobuf-openapi`

**Files:**
- Create: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/specimen.yaml`
- Create: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/golden.yaml`
- Create verbatim fixtures under: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/fixture/`

### Step 1: Pin exact upstream revision

Use `kubernetes/kubernetes` revision:

```text
82ca8014fabed9e61cf6c14560cdb9f1e4e1d067
```

### Step 2: Copy complete source/generation manifestations

Copy verbatim:

```text
staging/src/k8s.io/api/core/v1/types.go
staging/src/k8s.io/api/core/v1/generated.proto
staging/src/k8s.io/api/core/v1/generated.pb.go
pkg/generated/openapi/zz_generated.openapi.go
```

These files are large by benchmark standards, but they are complete upstream manifestations and must not be hand-sliced.

Compute and record exact SHA-256 values.

### Step 3: Establish golden truth narrowly around Pod

Do not golden-label every type in these files.

At minimum encode:

- logical contract `kubernetes.core.v1.Pod`;
- Go source representation;
- generated protobuf representation;
- generated Go protobuf manifestation;
- generated OpenAPI manifestation;
- authority on the source/API type rather than generated artifacts;
- generation/projection relationships among those representations;
- explicit non-contract examples among adjacent generated helper machinery.

This case is deliberately designed to test false-contract restraint in a huge generated file.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources kubernetes/go-protobuf-openapi/core-v1-pod
cargo benchmark kubernetes/go-protobuf-openapi/core-v1-pod
```

Expected today: provenance PASS; execution may fail until generic Go/Proto/OpenAPI analysis exists.

### Step 5: Commit

```sh
git add benchmark/kubernetes
git commit -m "test: add Kubernetes contract benchmark case"
```

---

## Task 12: Curate Pydantic `validation-vs-serialization`

**Files:**
- Create: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/specimen.yaml`
- Create: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/golden.yaml`
- Create verbatim fixtures under: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/fixture/`

### Step 1: Use the already-pinned Pydantic revision

Use `pydantic/pydantic` revision:

```text
001dea020e0809844e5b17666432c9135a976f46
```

### Step 2: Copy complete relevant files

Copy verbatim:

```text
pydantic/json_schema.py
pydantic/main.py
tests/test_json_schema.py
```

Compute and record exact SHA-256 values.

### Step 3: Establish golden truth without runtime probes

At minimum encode:

- the logical JSON-schema mode contract;
- validation and serialization as intentionally distinct perspectives/manifestations rather than accidental drift;
- `JsonSchemaMode = Literal['validation', 'serialization']` as structural evidence;
- source logic/documentation explaining that validation describes inputs while serialization describes outputs;
- tests as verification manifestations where directly relevant;
- **no** expected contested finding merely because validation and serialization schemas differ intentionally.

If the current Chirograph ontology cannot represent the distinction cleanly, preserve the golden truth and let the benchmark expose that gap. Do not add a Pydantic-specific runtime observer under `benchmark/`.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources pydantic/validation-vs-serialization/json-schema-mode
cargo benchmark pydantic/validation-vs-serialization/json-schema-mode
```

### Step 5: Commit

```sh
git add benchmark/pydantic
git commit -m "test: add Pydantic contract benchmark case"
```

---

## Task 13: Curate Rails `migration-db-schema-authority`

**Files:**
- Create: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/specimen.yaml`
- Create: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/golden.yaml`
- Create verbatim fixtures under: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/fixture/`

### Step 1: Pin exact Rails revision

Use `rails/rails` revision:

```text
2164c6c6f7fb91cc1caff5ee4b05931445de42ea
```

### Step 2: Copy complete relevant files

Copy verbatim:

```text
guides/source/active_record_migrations.md
activerecord/lib/active_record/schema_dumper.rb
```

If manual golden review determines one database-task implementation file is necessary to establish load/dump direction, add that complete file before finalizing the case; do not add excerpts.

Compute and record exact SHA-256 values.

### Step 3: Establish golden truth

At minimum encode:

- logical contract `rails.database.schema-current-state`;
- live database state as the conceptual authority for current schema truth;
- schema dump as a generated/current-state manifestation;
- migrations as historical change instructions, not complete current-state authority;
- `SchemaDumper` as machinery projecting live database metadata into the schema dump;
- lifecycle/authority notes supported by the guide and generated header text;
- non-contract examples for individual migration helper methods.

A representation in golden truth may describe an external authority such as the live database even when no fixture file contains that runtime state; its locator must be explicit and auditably justified by the fixture source.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources rails/migration-db-schema-authority/schema-dump-current-state
cargo benchmark rails/migration-db-schema-authority/schema-dump-current-state
```

### Step 5: Commit

```sh
git add benchmark/rails
git commit -m "test: add Rails contract benchmark case"
```

---

## Task 14: Curate Envoy `v2-v3-lifecycle`

**Files:**
- Create: `benchmark/envoy/v2-v3-lifecycle/config-source/specimen.yaml`
- Create: `benchmark/envoy/v2-v3-lifecycle/config-source/golden.yaml`
- Create verbatim fixtures under: `benchmark/envoy/v2-v3-lifecycle/config-source/fixture/`

### Step 1: Pin exact Envoy revision

Use `envoyproxy/envoy` revision:

```text
6609c01e330fb84049d54a9dbffae8609b1ec9f7
```

### Step 2: Copy complete files

Copy verbatim:

```text
api/API_VERSIONING.md
api/envoy/api/v2/core/config_source.proto
api/envoy/config/core/v3/config_source.proto
```

Compute and record exact SHA-256 values.

### Step 3: Establish lifecycle golden truth

At minimum encode:

- logical contract `envoy.config-source`;
- v2 representation with lifecycle status `frozen`;
- v3 representation with lifecycle status `active`;
- current structural authority on v3;
- v2 migration annotation pointing to `envoy.config.core.v3`;
- v3 `previous_message_type` links back to v2;
- relationship connecting the lifecycle representations;
- explicit non-contract truth for deprecated enum constants that should not become separate logical contracts.

This case should remain present even while lifecycle correctness is initially `null` because current `ContractGraph` has no lifecycle model. When general Chirograph lifecycle support arrives, the same golden file becomes scoreable without benchmark changes.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources envoy/v2-v3-lifecycle/config-source
cargo benchmark envoy/v2-v3-lifecycle/config-source
```

### Step 5: Commit

```sh
git add benchmark/envoy
git commit -m "test: add Envoy contract benchmark case"
```

---

## Task 15: Curate Arrow `cross-language-schema`

**Files:**
- Create: `benchmark/arrow/cross-language-schema/field-and-schema/specimen.yaml`
- Create: `benchmark/arrow/cross-language-schema/field-and-schema/golden.yaml`
- Create verbatim fixtures under: `benchmark/arrow/cross-language-schema/field-and-schema/fixture/`

### Step 1: Pin exact Arrow revision

Use `apache/arrow` revision:

```text
1b0b16a8f68c0082534aa185c0bb4052614df924
```

### Step 2: Copy complete cross-language files from the current monorepo

Copy verbatim:

```text
format/Schema.fbs
cpp/src/arrow/type.h
python/pyarrow/types.pxi
python/pyarrow/includes/libarrow.pxd
```

Do not assume the current Apache Arrow monorepo contains the Java implementation; it does not at this exact revision. This case uses canonical FlatBuffers format plus C++ and Python/Cython manifestations available in the pinned repository.

Compute and record exact SHA-256 values.

### Step 3: Establish golden truth

At minimum encode:

- logical contract `arrow.schema.field` and/or the smallest defensible shared schema contract established by manual review;
- canonical FlatBuffers format representation;
- C++ representation of Field/Schema type semantics;
- Python/Cython binding manifestation;
- authority on the canonical format for interchange semantics while language implementation details remain implementation manifestations;
- cross-language projection/implementation relationships;
- non-contract examples for helper/binding machinery.

Do not inflate every Arrow physical/logical type into a benchmark contract merely because the files contain them.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources arrow/cross-language-schema/field-and-schema
cargo benchmark arrow/cross-language-schema/field-and-schema
```

### Step 5: Commit

```sh
git add benchmark/arrow
git commit -m "test: add Arrow contract benchmark case"
```

---

## Task 16: Curate Temporal `multi-dialect-persistence`

**Files:**
- Create: `benchmark/temporal/multi-dialect-persistence/executions-table/specimen.yaml`
- Create: `benchmark/temporal/multi-dialect-persistence/executions-table/golden.yaml`
- Create verbatim fixtures under: `benchmark/temporal/multi-dialect-persistence/executions-table/fixture/`

### Step 1: Pin exact Temporal revision

Use `temporalio/temporal` revision:

```text
cd667daadb88d189df0302d8f473858ee7168ce5
```

### Step 2: Copy complete dialect and test-reference files

Copy verbatim:

```text
schema/mysql/v8/temporal/schema.sql
schema/postgresql/v12/temporal/schema.sql
schema/cassandra/temporal/schema.cql
tools/tests/test_data.go
common/persistence/tests/mysql_test_util.go
common/persistence/tests/postgresql_test_util.go
common/persistence/tests/cassandra_test_util.go
```

Compute and record exact SHA-256 values.

### Step 3: Establish golden truth narrowly around `executions`

At minimum encode:

- logical contract `temporal.persistence.executions`;
- MySQL, PostgreSQL, and Cassandra schema manifestations;
- no false claim that one SQL dialect file is globally authoritative over the others;
- equivalence/projection relationships only where the same logical persistence concept is genuinely represented;
- test utility references as verification evidence that each dialect schema is intentionally exercised;
- non-contract examples for dialect-specific indexes/helper tables that are not part of the selected logical contract.

This case should expose whether Chirograph can model one logical contract with multiple dialect-specific manifestations without collapsing them into duplicates or choosing a fake single authority.

### Step 4: Verify and run

```sh
cargo benchmark --verify-sources temporal/multi-dialect-persistence/executions-table
cargo benchmark temporal/multi-dialect-persistence/executions-table
```

### Step 5: Commit

```sh
git add benchmark/temporal
git commit -m "test: add Temporal contract benchmark case"
```

---

## Task 17: Add the eighth-case baseline and regression policy

**Files:**
- Create: `benchmark/baseline.json`
- Create: `crates/chirograph-benchmark/src/baseline.rs`
- Modify: `crates/chirograph-benchmark/src/lib.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Test: `crates/chirograph-benchmark/tests/baseline.rs`

### Step 1: Write failing regression tests

Cover:

- `execution-failure -> scored`: improvement, allowed;
- `scored -> execution-failure`: regression;
- `scored -> invalid-output`: regression;
- higher-is-better metric decreases: regression;
- false-contract rate increases: regression;
- contract-inflation distance from `1.0` increases: regression;
- `null lifecycle -> scored lifecycle`: improvement/activation, allowed;
- baseline missing a corpus case: corpus/baseline validation failure;
- baseline contains removed case: validation failure;
- current golden file changes without baseline update: fail via stored golden digest.

### Step 2: Store truth identity in the baseline

Each baseline entry must contain:

- case status;
- scored metrics when available;
- SHA-256 of `golden.yaml`;
- SHA-256 of `specimen.yaml`.

This makes a golden/provenance change an explicit reviewed baseline event.

### Step 3: Implement metric directionality

Regression rules:

- precision/recall/F1/authority/relationship/finding/lifecycle: higher is better;
- false-contract rate: lower is better;
- contract inflation: compare `abs(ratio - 1.0)`; farther from `1.0` is worse;
- `null` metrics are not numerically compared until both baseline and current run have a value.

Use a small exact tolerance only for floating-point formatting noise; all counts remain exact integers.

### Step 4: Generate initial baseline explicitly

Add an operator command:

```text
cargo benchmark all --write-baseline benchmark/baseline.json
```

It must require the exact output path and print a warning that this operation accepts current results. It must not be the default behavior of a failing run.

Run the full corpus once. It is acceptable, and expected before public source analysis lands, for all or most cases to be `execution-failure`. That is a truthful starting baseline, not a reason to add benchmark-specific code.

### Step 5: Verify

```sh
cargo test -p chirograph-benchmark --test baseline -- --nocapture
cargo benchmark all --baseline benchmark/baseline.json
```

Expected: PASS against the just-reviewed baseline.

### Step 6: Commit

```sh
git add benchmark/baseline.json crates/chirograph-benchmark/src/baseline.rs crates/chirograph-benchmark/src/lib.rs crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/tests/baseline.rs
git commit -m "test: establish benchmark regression baseline"
```

---

## Task 18: Add CI and benchmark methodology documentation

**Files:**
- Modify: `.github/workflows/ci.yml`
- Create: `benchmark/README.md`
- Modify: `README.md`

### Step 1: Update CI

After normal workspace tests, add:

```sh
cargo build --quiet -p chirograph-cli
cargo benchmark all --baseline benchmark/baseline.json
```

Do **not** run `--verify-sources` in every normal CI job; live upstream retrieval is explicitly outside the hermetic benchmark run.

If license/provenance CI from the FOSS-foundation transition later exists, ensure fixture provenance validation composes with it rather than creating a duplicate policy checker.

### Step 2: Document benchmark methodology

`benchmark/README.md` must explain concisely:

- data-only corpus invariant;
- verbatim fixture policy;
- exact provenance;
- repository/scenario/case dimensions;
- selector examples;
- scoring vector;
- false-contract-rate importance;
- macro vs micro;
- baseline ratchet behavior;
- why execution failures are distinct from semantic mismatches;
- why remote source verification is explicit maintenance, not ordinary scoring;
- how to add a new case without adding repository-specific code.

### Step 3: Update top-level README navigation

Add one benchmark entry linking to `benchmark/README.md`. Do not duplicate the methodology in the root README.

### Step 4: Verify CI-equivalent commands locally

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --quiet -p chirograph-cli
cargo benchmark --list
cargo benchmark all --baseline benchmark/baseline.json
```

Expected: all commands PASS. Real benchmark cases may be recorded as execution failures if that is their reviewed baseline; the benchmark process itself must still exit successfully when current status is no worse than baseline.

### Step 5: Commit

```sh
git add .github/workflows/ci.yml benchmark/README.md README.md
git commit -m "ci: run contract benchmark regression suite"
```

---

## Task 19: Amend Overcenter project truth to match the approved benchmark architecture

This is orchestration state, not benchmark implementation. Perform it only after re-reading the live `mcp/project.amend.js` contract and re-running `project.inspect` for an exact current authority revision.

**Current conflict to remove:** the existing future `validate-cargo-rust-specimen` transition references the old plural `benchmarks/` layout and explicitly permits Cargo-specific stance mapping in the benchmark layer. That contradicts the approved design.

### Step 1: Inspect exact current project authority

Invoke:

```text
project.inspect { project_ref: "github:laurajoyhutchins/chirograph" }
```

Record the returned exact `authority_revision`. If it differs from the revision used to prepare the amendment, recompute the amendment from the new graph and fail closed on ambiguity.

### Step 2: Amend semantic transitions

Use `project.amend` with the exact observed revision.

Remove or replace the obsolete future Cargo validation transition before it can execute.

Add benchmark work as small semantic transitions corresponding to the implementation checkpoints above, with dependencies that preserve the public-boundary rule. A suitable graph shape is:

```text
implement-benchmark-graph-json
        |
implement-benchmark-framework
        |
        +--> curate-benchmark-cargo
        +--> curate-benchmark-kafka
        +--> curate-benchmark-kubernetes
        +--> curate-benchmark-pydantic
        +--> curate-benchmark-rails
        +--> curate-benchmark-envoy
        +--> curate-benchmark-arrow
        +--> curate-benchmark-temporal
                    |
            establish-benchmark-baseline
                    |
              wire-benchmark-ci
```

The eight curation transitions may proceed independently once the framework exists.

Do **not** encode run IDs, leases, branches, PR numbers, or implementation bookkeeping in the amendment. Overcenter owns those.

### Step 3: Read back authoritative graph

Re-run `project.inspect` and verify the new frontier/dependencies match the intended semantic graph. Treat a mutation with uncertain outcome or mismatched readback as unresolved; do not retry blindly.

---

## Final verification checkpoint

Before claiming the benchmark work complete, run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --quiet -p chirograph-cli
cargo benchmark --list
cargo benchmark all --baseline benchmark/baseline.json
```

Then explicitly run source verification for all eight pinned cases as a separate, networked provenance check:

```sh
cargo benchmark --verify-sources
```

Record:

- exact Chirograph commit SHA;
- exact eight upstream revisions;
- fixture SHA-256 values;
- benchmark case statuses and score vectors;
- macro/micro aggregates for scored cases;
- any execution-failure cases still waiting on generic public analysis capabilities;
- CI result.

Do not convert execution failures into hand-authored graphs to improve the final report.

---

## Definition of done

The implementation is complete when:

1. `benchmark/` is a single data-only corpus root with all eight approved repository/scenario families.
2. Every fixture file is verbatim upstream content pinned to an exact revision with a local SHA-256.
3. No benchmark case contains repository-specific executable analysis glue.
4. `cargo benchmark` supports repository, scenario, intersection, exact-case, and all-corpus selectors.
5. The scorer implements the approved metric vector with strict logical identity and fail-closed false contracts.
6. Macro and micro aggregation are both available, with macro as the headline view.
7. Normal benchmark execution is hermetic/offline.
8. Explicit source verification/refresh is exact-revision-only and never rewrites golden truth.
9. The runner invokes only the public general Chirograph analysis boundary.
10. Unsupported real cases remain explicit execution failures rather than being rescued by private benchmark paths.
11. CI gates regressions against a reviewed baseline and never auto-accepts a failing run.
12. Overcenter project truth no longer advertises the superseded Cargo-specific benchmark mapping path.
