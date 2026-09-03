# Chirograph Contract Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a hermetic, data-only contract benchmark framework plus eight curated real-repository cases that score Chirograph's final logical contract graph against human-reviewed golden truth without adding repository-specific analysis code.

**Architecture:** Add a small `chirograph-benchmark` workspace crate that discovers `benchmark/<repository>/<scenario>/<case>/`, validates provenance/golden truth, invokes the public `chirograph analyze ... --format graph-json` boundary as an external process, scores canonical graphs, aggregates results, verifies exact upstream fixture bytes only on explicit maintenance commands, and gates regressions against a checked-in baseline. Add an explicit canonical graph-JSON projection in `chirograph-core`; do not derive public serialization directly from every internal model struct. The benchmark does not implement source-to-contract reconstruction: unsupported real cases remain explicit execution failures until generic Chirograph analysis makes them scoreable.

**Tech Stack:** Rust 2024 workspace; Rust 1.85 minimum; `serde`/`serde_json`; `yaml_serde = "0.10.7"` for typed YAML; `sha2 = "0.10"`; standard-library filesystem/process APIs; Git CLI only for explicit remote fixture verification/refresh.

**Spec:** `docs/superpowers/specs/2026-09-03-contract-benchmark-design.md`

## Global Constraints

- The benchmark corpus root is exactly `benchmark/`; do not create a root `specimens/` or `benchmarks/` directory.
- `benchmark/` is data-only. Source code under `fixture/` is inert input; no repository-specific executable glue, observers, symbol maps, extractors, or stance mapping may live there.
- Fixture files are complete verbatim upstream files pinned to exact 40-character Git revisions and SHA-256 digests.
- Normal scoring is hermetic/offline. Only explicit `--verify-sources` and `--refresh` operations may use Git/network access.
- Matching is strict by canonical logical identity; no fuzzy matching.
- Every unmatched emitted logical contract is false for the current run until human-reviewed golden truth changes.
- The headline metrics are contract precision/recall/F1, false-contract rate, contract inflation, authority correctness, relationship precision/recall, lifecycle correctness when observable, and finding precision/recall. No composite score in v1.
- Report case, repository, scenario, and overall views. Macro is headline; micro is secondary.
- CI gates regression against a reviewed baseline, not an aspirational absolute threshold.
- The runner invokes only the public general Chirograph analysis boundary. It never privately dispatches adapters.
- The current repository has no general source-tree → contract-graph analyzer. This plan does not invent one inside the benchmark. Real cases may initially baseline as `execution-failure`.
- Project execution/orchestration must use Overcenter. GitHub remains source authority; Overcenter remains run/lease/transition authority.

---

### Task 1: Align Overcenter project truth before implementation

**Files:**
- Read only before mutation: `laurajoyhutchins/overcenter:mcp/project.inspect.js`
- Read only before mutation: `laurajoyhutchins/overcenter:mcp/project.amend.js`
- Read only before mutation: `laurajoyhutchins/chirograph:.overcenter/definitions/chirograph.json`

**Interfaces:**
- Consumes: deployed Overcenter `project.inspect` and `project.amend` semantic commands.
- Produces: authoritative Chirograph project transitions for benchmark implementation; removes the superseded Cargo benchmark transition before execution can claim it.

- [ ] **Step 1: Re-read the live Overcenter contracts**

Read `mcp/project.inspect.js`, `mcp/project.amend.js`, and the current semantic command descriptor/schema from the live Overcenter source before invoking either command.

- [ ] **Step 2: Inspect exact current project authority**

Invoke:

```json
{
  "command": "project.inspect",
  "input": {
    "project_ref": "github:laurajoyhutchins/chirograph"
  }
}
```

Record the returned exact `authority_revision`. Do not reuse the planning-time revision if main moved.

- [ ] **Step 3: Amend the graph semantically**

At the exact revision from Step 2, remove the future `validate-cargo-rust-specimen` transition because it encodes the now-rejected plural `benchmarks/` layout and Cargo-specific stance mapping.

Upsert these semantic transitions (each executor uses the Superpowers TDD plan in this file; do not encode branch/run/lease bookkeeping):

```json
[
  {"id":"implement-benchmark-graph-json","priority":93,"requires":[]},
  {"id":"implement-contract-benchmark-framework","priority":92,"requires":["implement-benchmark-graph-json"]},
  {"id":"curate-benchmark-cargo","priority":86,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-kafka","priority":85,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-kubernetes","priority":84,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-pydantic","priority":83,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-rails","priority":82,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-envoy","priority":81,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-arrow","priority":80,"requires":["implement-contract-benchmark-framework"]},
  {"id":"curate-benchmark-temporal","priority":79,"requires":["implement-contract-benchmark-framework"]},
  {"id":"establish-contract-benchmark-baseline","priority":75,"requires":["curate-benchmark-cargo","curate-benchmark-kafka","curate-benchmark-kubernetes","curate-benchmark-pydantic","curate-benchmark-rails","curate-benchmark-envoy","curate-benchmark-arrow","curate-benchmark-temporal"]},
  {"id":"wire-contract-benchmark-ci","priority":74,"requires":["establish-contract-benchmark-baseline"]}
]
```

Supply complete executor objects accepted by the live Overcenter project-definition contract, with role `implementation` or `contract-analysis-engineer` and skill text pointing to the exact task(s) in this plan. Preserve any already-confirmed transition meanings; do not rewrite confirmed transitions in place.

- [ ] **Step 4: Read back authoritative graph**

Invoke `project.inspect` again and verify the semantic dependency shape is present. If mutation outcome/readback is uncertain, stop and use Overcenter recovery semantics; do not blindly retry the mutation.

- [ ] **Step 5: Execute subsequent tasks through Overcenter**

For implementation, use `project.advance` to obtain each READY transition and its bounded execution packet. Settle work only through the same semantic boundary with exact evidence. Do not manually recreate leases, claims, settlement, or continuation.

---

### Task 2: Add canonical final-graph JSON

**Files:**
- Create: `crates/chirograph-core/src/graph_json.rs`
- Modify: `crates/chirograph-core/src/lib.rs`
- Test: `crates/chirograph-core/src/graph_json.rs`

**Interfaces:**
- Consumes: `chirograph_core::model::ContractGraph` and `ContractGraph::validate` / `assess_clause`.
- Produces: `graph_json::GraphJsonV1`, `graph_json::encode_graph_json(&ContractGraph) -> Result<String, GraphJsonError>`, schema constant `chirograph-graph-v1`.

- [ ] **Step 1: Write failing projection tests**

Add a test that constructs a valid graph with one contract, two representations, one relation, one authority claim, and one contested clause; insert vectors out of lexical order and assert canonical output ordering.

```rust
#[test]
fn encodes_valid_graph_in_canonical_order() {
    let graph = fixture_graph_with_reversed_vectors();
    let json = encode_graph_json(&graph).expect("valid graph");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "chirograph-graph-v1");
    assert_eq!(value["contracts"][0]["id"], "example.contract");
    assert_eq!(value["clause_assessments"][0]["status"], "contested");
}

#[test]
fn refuses_invalid_graph() {
    let graph = fixture_graph_with_dangling_representation();
    assert!(encode_graph_json(&graph).is_err());
}
```

- [ ] **Step 2: Run focused tests and confirm red**

```sh
cargo test -p chirograph-core graph_json -- --nocapture
```

Expected: FAIL because `graph_json` does not exist.

- [ ] **Step 3: Implement explicit wire DTOs**

Implement DTOs rather than adding `Serialize` derives to core domain structs:

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphNodeRefV1 {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphLifecycleV1 {
    pub subject: GraphNodeRefV1,
    pub status: String,
}
```

Core conversion emits `lifecycle: Vec::new()` until Chirograph gains a real lifecycle model. It must never synthesize lifecycle labels from filenames/comments merely to satisfy benchmarks.

Add explicit conversion functions for every current enum string so Rust enum names are not the wire contract. Sort every list deterministically by stable IDs/typed edge tuples.

- [ ] **Step 4: Compute clause assessments mechanically**

After `graph.validate()?`, call `graph.assess_clause(&clause.id)` for every clause and encode the resulting status/supporting/contradicting representation IDs.

- [ ] **Step 5: Re-run focused and core quality checks**

```sh
cargo test -p chirograph-core graph_json -- --nocapture
cargo check -p chirograph-core
cargo clippy -p chirograph-core --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```sh
git add crates/chirograph-core/src/graph_json.rs crates/chirograph-core/src/lib.rs
git commit -m "feat: add canonical contract graph json"
```

---

### Task 3: Build corpus model, discovery, and validation

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/chirograph-benchmark/Cargo.toml`
- Create: `crates/chirograph-benchmark/src/lib.rs`
- Create: `crates/chirograph-benchmark/src/model.rs`
- Create: `crates/chirograph-benchmark/src/corpus.rs`
- Test: `crates/chirograph-benchmark/tests/corpus.rs`

**Interfaces:**
- Consumes: `chirograph_core::graph_json::{GraphJsonV1, GraphNodeRefV1}`.
- Produces: `SpecimenV1`, `GoldenV1`, `BenchmarkCase`, `discover_corpus(root: &Path) -> Result<Vec<BenchmarkCase>, CorpusError>`.

- [ ] **Step 1: Add the crate and dependencies**

Root workspace gains `crates/chirograph-benchmark`. Its dependency block is:

```toml
[dependencies]
chirograph-core = { path = "../chirograph-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
yaml_serde = "0.10.7"
sha2 = "0.10"
```

- [ ] **Step 2: Write failing typed-format tests**

Parse this exact specimen in a test:

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

Golden structs must support:

```rust
pub struct GoldenV1 {
    pub schema: String,
    pub contracts: Vec<GoldenContractV1>,
    pub representations: Vec<GoldenRepresentationV1>,
    pub authority_claims: Vec<GoldenAuthorityClaimV1>,
    pub relationships: Vec<GoldenRelationshipV1>,
    pub clauses: Vec<GoldenClauseV1>,
    pub lifecycle: Vec<GoldenLifecycleV1>,
    pub expected_findings: Vec<GoldenFindingV1>,
    pub non_contracts: Vec<GoldenNonContractV1>,
}
```

Use `#[serde(deny_unknown_fields)]` on benchmark-file structs so typos fail closed.

- [ ] **Step 3: Run and confirm red**

```sh
cargo test -p chirograph-benchmark --test corpus -- --nocapture
```

Expected: FAIL because the crate/types do not exist.

- [ ] **Step 4: Implement strict model validation**

Require:

- schema names exactly `chirograph-benchmark-specimen-v1` / `chirograph-benchmark-golden-v1`;
- non-empty IDs;
- exact 40-hex revision;
- exact 64-hex SHA-256;
- no duplicate contract/representation/clause IDs;
- all authority/relationship/clause/lifecycle/finding references resolve;
- at least one golden contract per case.

`GoldenFindingV1` v1 supports only:

```rust
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GoldenFindingV1 {
    ContestedClause { clause: String },
}
```

`GoldenNonContractV1` is:

```rust
pub struct GoldenNonContractV1 {
    pub locator: String,
    pub reason: String,
}
```

- [ ] **Step 5: Write failing fixed-layout discovery tests**

Create temporary cases under exactly:

```text
benchmark/<repository>/<scenario>/<case>/
```

Assert rejection for metadata/path disagreement, undeclared fixture bytes, missing fixture bytes, local SHA mismatch, path traversal, and executable benchmark glue next to `specimen.yaml`/`golden.yaml`.

Source files of any extension are legal under `fixture/`.

- [ ] **Step 6: Implement fixed-depth discovery/local digest verification**

Use `std::fs::read_dir` for repository/scenario/case. Recursively enumerate only `fixture/` to ensure every byte file is declared exactly once. Hash raw bytes with SHA-256.

- [ ] **Step 7: Run tests and commit**

```sh
cargo test -p chirograph-benchmark --test corpus -- --nocapture
cargo check -p chirograph-benchmark
```

Expected: PASS.

```sh
git add Cargo.toml crates/chirograph-benchmark
git commit -m "feat: add benchmark corpus model"
```

---

### Task 4: Add selectors and ergonomic `cargo benchmark`

**Files:**
- Create: `crates/chirograph-benchmark/src/selector.rs`
- Create: `crates/chirograph-benchmark/src/main.rs`
- Create: `.cargo/config.toml`
- Test: `crates/chirograph-benchmark/tests/selectors.rs`

**Interfaces:**
- Consumes: `Vec<BenchmarkCase>` from Task 3.
- Produces: `select_cases(cases: &[BenchmarkCase], selector: &str) -> Result<Vec<BenchmarkCase>, SelectorError>` and CLI selectors.

- [ ] **Step 1: Write failing selector tests**

For three synthetic cases, assert exact selection for:

```text
all
cargo
scenario:schema-enum-drift
cargo/schema-enum-drift
cargo/schema-enum-drift/case-a
```

Unknown/zero-match selectors must error.

- [ ] **Step 2: Run red test**

```sh
cargo test -p chirograph-benchmark --test selectors -- --nocapture
```

Expected: FAIL.

- [ ] **Step 3: Implement selection rules**

One token means repository, `scenario:` means scenario dimension, two path components mean repository/scenario, three mean exact case. Return lexical case order.

- [ ] **Step 4: Implement a small hand-written CLI**

Support exactly:

```text
chirograph-benchmark --help
chirograph-benchmark --list
chirograph-benchmark all
chirograph-benchmark <selector>
chirograph-benchmark --verify-sources [selector]
chirograph-benchmark --refresh <selector> --revision <40-hex-sha>
chirograph-benchmark <selector> --baseline benchmark/baseline.json
chirograph-benchmark <selector> --write-baseline benchmark/baseline.json
chirograph-benchmark <selector> --chirograph-bin <path>
chirograph-benchmark <selector> --format json
```

Do not add a general CLI framework dependency.

- [ ] **Step 5: Add Cargo alias**

```toml
[alias]
benchmark = "run --quiet -p chirograph-benchmark --"
```

- [ ] **Step 6: Verify and commit**

```sh
cargo test -p chirograph-benchmark --test selectors -- --nocapture
cargo benchmark --help
```

Expected: PASS.

```sh
git add .cargo/config.toml crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/src/selector.rs crates/chirograph-benchmark/tests/selectors.rs
git commit -m "feat: add benchmark selectors"
```

---

### Task 5: Implement scoring, aggregation, and reports

**Files:**
- Create: `crates/chirograph-benchmark/src/score.rs`
- Create: `crates/chirograph-benchmark/src/aggregate.rs`
- Create: `crates/chirograph-benchmark/src/report.rs`
- Test: `crates/chirograph-benchmark/tests/score.rs`
- Test: `crates/chirograph-benchmark/tests/report.rs`

**Interfaces:**
- Consumes: `GoldenV1`, `GraphJsonV1`.
- Produces: `score_case(golden: &GoldenV1, observed: &GraphJsonV1) -> CaseScore`; `aggregate_report(results: &[CaseResult]) -> BenchmarkReportV1`.

- [ ] **Step 1: Write failing metric tests**

Use synthetic graphs to cover perfect match, missing contract, false contract, zero emissions, authority 2/3 correct, wrong relation kind, missing relation, false relation, contested-finding hit/miss/false positive, lifecycle exact match, and lifecycle unavailable.

The zero-emission test must assert:

```rust
assert_eq!(score.contract_precision, None);
assert_eq!(score.contract_recall, Some(0.0));
assert_eq!(score.false_contract_rate, None);
assert_eq!(score.contract_inflation, 0.0);
```

- [ ] **Step 2: Run red metric tests**

```sh
cargo test -p chirograph-benchmark --test score -- --nocapture
```

Expected: FAIL.

- [ ] **Step 3: Implement exact contract/authority/relation scoring**

Contract matching is exact ID membership. Authority compares exact `(contract, facet, representation)` triples. Relationship compares exact `(from.kind, from.id, kind, to.kind, to.id)` tuples.

No fuzzy aliases or similarity scoring.

- [ ] **Step 4: Implement lifecycle/finding behavior**

When golden lifecycle expectations exist but observed lifecycle is empty, return lifecycle metric `None` plus diagnostic `lifecycle_not_observed`. When lifecycle facts exist, compare exact `(subject,status)` pairs.

Map `GoldenFindingV1::ContestedClause` to observed clause assessment status `contested`.

- [ ] **Step 5: Add known-negative diagnostics**

If an observed representation locator matches `golden.non_contracts[].locator` and its owning observed contract is not a golden contract, add diagnostic `known_non_contract_promoted`. It remains an ordinary false contract; this diagnostic does not change the numerator/denominator.

- [ ] **Step 6: Write failing macro/micro report tests**

Use one 100-contract case and one 1-contract case so macro and micro differ. Require repository, scenario, and overall aggregate rows.

- [ ] **Step 7: Implement deterministic human/JSON reporting**

Human columns, in order:

```text
scope | contract P/R/F1 | false-rate | inflation | authority | relations P/R | lifecycle | findings P/R | status
```

JSON schema is `chirograph-benchmark-report-v1`. Sort cases/scopes lexically and format ratios deterministically.

- [ ] **Step 8: Verify and commit**

```sh
cargo test -p chirograph-benchmark --test score -- --nocapture
cargo test -p chirograph-benchmark --test report -- --nocapture
```

Expected: PASS.

```sh
git add crates/chirograph-benchmark/src/score.rs crates/chirograph-benchmark/src/aggregate.rs crates/chirograph-benchmark/src/report.rs crates/chirograph-benchmark/tests/score.rs crates/chirograph-benchmark/tests/report.rs
git commit -m "feat: score and report contract benchmarks"
```

---

### Task 6: Enforce the public analyzer process boundary

**Files:**
- Create: `crates/chirograph-benchmark/src/runner.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Test: `crates/chirograph-benchmark/tests/runner.rs`

**Interfaces:**
- Consumes: selected `BenchmarkCase`, product executable path.
- Produces: `run_case(case: &BenchmarkCase, chirograph_bin: &Path) -> CaseResult` with status `execution-failure`, `invalid-output`, or `scored`.

- [ ] **Step 1: Write failing fake-binary tests**

Use temporary executable helpers for: nonzero exit, zero/non-JSON stdout, zero/wrong graph schema, and zero/valid graph JSON. Assert the three failure/status classes exactly.

- [ ] **Step 2: Run red test**

```sh
cargo test -p chirograph-benchmark --test runner -- --nocapture
```

Expected: FAIL.

- [ ] **Step 3: Implement product binary resolution**

Resolution order:

```text
--chirograph-bin
CHIROGRAPH_BIN
target/debug/chirograph (target/debug/chirograph.exe on Windows)
```

If default binary is absent, run `cargo build --quiet -p chirograph-cli` from workspace root once, then resolve it. This build step is generic product tooling, not adapter dispatch.

- [ ] **Step 4: Invoke exactly the public contract**

```rust
Command::new(chirograph_bin)
    .arg("analyze")
    .arg(&case.fixture_dir)
    .arg("--format")
    .arg("graph-json")
```

No language flags, semantic queries, repository names, symbol maps, or adapter commands.

- [ ] **Step 5: Classify output fail-closed**

Nonzero exit => `execution-failure`; zero but malformed/wrong-schema graph => `invalid-output`; only valid canonical graph => score. Preserve bounded stderr as diagnostics but never infer success/failure subtype from message wording.

Today the real CLI is expected to reject `analyze`; that is a truthful execution failure.

- [ ] **Step 6: Verify and commit**

```sh
cargo test -p chirograph-benchmark --test runner -- --nocapture
```

Expected: PASS.

```sh
git add crates/chirograph-benchmark/src/runner.rs crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/tests/runner.rs
git commit -m "feat: enforce public benchmark analysis boundary"
```

---

### Task 7: Add exact source verification and refresh

**Files:**
- Create: `crates/chirograph-benchmark/src/source.rs`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Test: `crates/chirograph-benchmark/tests/source.rs`

**Interfaces:**
- Consumes: selected `BenchmarkCase` provenance.
- Produces: `verify_sources(cases, fetcher)`, `refresh_sources(cases, exact_revision, fetcher)`; production `GitSourceFetcher`.

- [ ] **Step 1: Write failing fake-fetcher tests**

Define:

```rust
pub trait SourceFetcher {
    fn fetch(&self, repository: &str, revision: &str, path: &str) -> Result<Vec<u8>, SourceError>;
}
```

With an in-memory fake, test remote mismatch, read-only verify, refresh byte replacement, SHA update, exact-revision requirement, and that `golden.yaml` bytes are unchanged.

- [ ] **Step 2: Run red test**

```sh
cargo test -p chirograph-benchmark --test source -- --nocapture
```

Expected: FAIL.

- [ ] **Step 3: Implement generic Git-backed fetcher**

For each upstream repository/revision group, use a temporary directory and exact commands:

```sh
git init
git remote add origin https://github.com/OWNER/REPO.git
git fetch --depth=1 origin EXACT_40_HEX_REVISION
git show FETCH_HEAD:UPSTREAM_PATH
```

Fail closed if the exact revision or any path cannot be fetched. Normal benchmark scoring never calls this code.

- [ ] **Step 4: Implement maintenance CLI semantics**

`--verify-sources` compares remote exact bytes with committed fixture bytes and committed SHA-256.

`--refresh SELECTOR --revision SHA` requires one exact SHA, rewrites only fixture bytes plus `specimen.yaml` revision/digests, and never modifies `golden.yaml`.

- [ ] **Step 5: Verify and commit**

```sh
cargo test -p chirograph-benchmark --test source -- --nocapture
```

Expected: PASS offline via fake fetcher.

```sh
git add crates/chirograph-benchmark/src/source.rs crates/chirograph-benchmark/src/main.rs crates/chirograph-benchmark/tests/source.rs
git commit -m "feat: verify benchmark source provenance"
```

---

### Task 8: Curate Cargo schema-enum drift

**Files:**
- Create: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/specimen.yaml`
- Create: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/golden.yaml`
- Create verbatim files under: `benchmark/cargo/schema-enum-drift/toml-debug-info-spellings/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `cargo/schema-enum-drift/toml-debug-info-spellings`.

- [ ] **Step 1: Copy exact upstream bytes**

Upstream is `rust-lang/cargo@2ceefa0090080354b80cc2f5415039bdb0d2bf0b`:

```text
crates/cargo-util-schemas/src/manifest/mod.rs
crates/cargo-util-schemas/manifest.schema.json
```

Required SHA-256:

```text
fc503d6532663f2b0f3217b53f235b6c24690e9c85116f1364ec134ca78cd92c  crates/cargo-util-schemas/src/manifest/mod.rs
a8f038d7ef99e69810c5cafd17d340b9aa42f7c9dd01e9ff70fb4205fec2f21e  crates/cargo-util-schemas/manifest.schema.json
```

- [ ] **Step 2: Write golden truth**

Use one logical contract `cargo.profile.debug-info`, representations `cargo.profile.debug-info.implementation` and `cargo.profile.debug-info.schema`, structural authority on the implementation representation, a `projects` relationship from implementation to schema, and clause `cargo.profile.debug-info.enum-spellings` describing the accepted serialized spellings. Mark that clause as an expected `contested-clause` because the generated schema uses stale enum spellings.

No code maps Cargo symbols to stances.

- [ ] **Step 3: Validate provenance and current status**

```sh
cargo benchmark --verify-sources cargo/schema-enum-drift/toml-debug-info-spellings
cargo benchmark cargo/schema-enum-drift/toml-debug-info-spellings
```

Expected: provenance PASS; current run may be `execution-failure` until public analysis exists.

- [ ] **Step 4: Commit**

```sh
git add benchmark/cargo
git commit -m "test: add Cargo contract benchmark case"
```

---

### Task 9: Curate Kafka message-spec generation

**Files:**
- Create: `benchmark/kafka/message-spec-generation/produce-request-data/specimen.yaml`
- Create: `benchmark/kafka/message-spec-generation/produce-request-data/golden.yaml`
- Create verbatim files under: `benchmark/kafka/message-spec-generation/produce-request-data/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `kafka/message-spec-generation/produce-request-data`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `apache/kafka@b57cf6e56eb59a952db7236b4da67cc2fdbb8cdf`:

```text
clients/src/main/resources/common/message/ProduceRequest.json
generator/src/main/java/org/apache/kafka/message/MessageGenerator.java
generator/src/main/java/org/apache/kafka/message/MessageSpec.java
generator/src/main/java/org/apache/kafka/message/MessageDataGenerator.java
clients/src/main/java/org/apache/kafka/common/requests/ProduceRequest.java
```

After copying, compute `sha256sum` (or platform equivalent) over each exact byte file and write those actual 64-hex values into `specimen.yaml` before committing.

- [ ] **Step 2: Write golden truth**

Use logical contract `kafka.protocol.produce-request`. `ProduceRequest.json` is the structural authority/spec representation. Represent the generated logical data type `org.apache.kafka.common.message.ProduceRequestData` as a generated manifestation even though the generated build output is not vendored; its locator is the generated type identity. Represent `MessageDataGenerator` as generation machinery and `ProduceRequest.java` as an implementation consumer. Encode `generates`/`depends-on` relationships without promoting generator helper classes into logical contracts.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources kafka/message-spec-generation/produce-request-data
cargo benchmark kafka/message-spec-generation/produce-request-data
```

```sh
git add benchmark/kafka
git commit -m "test: add Kafka contract benchmark case"
```

---

### Task 10: Curate Kubernetes Go/Protobuf/OpenAPI projection

**Files:**
- Create: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/specimen.yaml`
- Create: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/golden.yaml`
- Create verbatim files under: `benchmark/kubernetes/go-protobuf-openapi/core-v1-pod/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `kubernetes/go-protobuf-openapi/core-v1-pod`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `kubernetes/kubernetes@82ca8014fabed9e61cf6c14560cdb9f1e4e1d067`:

```text
staging/src/k8s.io/api/core/v1/types.go
staging/src/k8s.io/api/core/v1/generated.proto
staging/src/k8s.io/api/core/v1/generated.pb.go
pkg/generated/openapi/zz_generated.openapi.go
```

Record actual SHA-256 values. Keep the complete generated files; do not hand-slice Pod excerpts.

- [ ] **Step 2: Write golden truth narrowly around Pod**

Use logical contract `kubernetes.core.v1.Pod`. Represent the Go API type as authority, and Protobuf, generated Go Protobuf, and generated OpenAPI as manifestations linked through `projects`/`generates` relationships. Golden truth names only the Pod contract, not every declaration in these large files. This case is intentionally hostile to contract inflation.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources kubernetes/go-protobuf-openapi/core-v1-pod
cargo benchmark kubernetes/go-protobuf-openapi/core-v1-pod
```

```sh
git add benchmark/kubernetes
git commit -m "test: add Kubernetes contract benchmark case"
```

---

### Task 11: Curate Pydantic validation-vs-serialization

**Files:**
- Create: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/specimen.yaml`
- Create: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/golden.yaml`
- Create verbatim files under: `benchmark/pydantic/validation-vs-serialization/json-schema-mode/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `pydantic/validation-vs-serialization/json-schema-mode`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `pydantic/pydantic@001dea020e0809844e5b17666432c9135a976f46`:

```text
pydantic/json_schema.py
pydantic/main.py
tests/test_json_schema.py
```

Record actual SHA-256 values.

- [ ] **Step 2: Write golden truth**

Use logical contract `pydantic.json-schema.mode`. Represent the mode definition and relevant tests. Encode two clauses: validation mode describes accepted input shape; serialization mode describes emitted output shape. They are intentional perspectives, so `expected_findings` is empty. A Chirograph implementation that flags ordinary validation/serialization differences as drift should lose finding precision.

Do not add runtime Python code under `benchmark/`.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources pydantic/validation-vs-serialization/json-schema-mode
cargo benchmark pydantic/validation-vs-serialization/json-schema-mode
```

```sh
git add benchmark/pydantic
git commit -m "test: add Pydantic contract benchmark case"
```

---

### Task 12: Curate Rails migration/database/schema authority

**Files:**
- Create: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/specimen.yaml`
- Create: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/golden.yaml`
- Create verbatim files under: `benchmark/rails/migration-db-schema-authority/schema-dump-current-state/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `rails/migration-db-schema-authority/schema-dump-current-state`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `rails/rails@2164c6c6f7fb91cc1caff5ee4b05931445de42ea`:

```text
guides/source/active_record_migrations.md
activerecord/lib/active_record/schema_dumper.rb
```

Record actual SHA-256 values.

- [ ] **Step 2: Write golden truth**

Use logical contract `rails.database.schema-current-state`. Represent `rails.database.live-schema` as the conceptual active authority, `rails.schema-dump` as a generated current-state manifestation, and `rails.migrations` as historical change instructions. The fixture source explicitly documents that the dump is generated from current DB state and that old migrations are not reliable current-state reconstruction authority. Encode `projects` from live schema to schema dump. Lifecycle labels: live schema `active`, dump `generated`, migrations `historical`.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources rails/migration-db-schema-authority/schema-dump-current-state
cargo benchmark rails/migration-db-schema-authority/schema-dump-current-state
```

```sh
git add benchmark/rails
git commit -m "test: add Rails contract benchmark case"
```

---

### Task 13: Curate Envoy v2/v3 lifecycle

**Files:**
- Create: `benchmark/envoy/v2-v3-lifecycle/config-source/specimen.yaml`
- Create: `benchmark/envoy/v2-v3-lifecycle/config-source/golden.yaml`
- Create verbatim files under: `benchmark/envoy/v2-v3-lifecycle/config-source/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `envoy/v2-v3-lifecycle/config-source`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `envoyproxy/envoy@6609c01e330fb84049d54a9dbffae8609b1ec9f7`:

```text
api/API_VERSIONING.md
api/envoy/api/v2/core/config_source.proto
api/envoy/config/core/v3/config_source.proto
```

Record actual SHA-256 values.

- [ ] **Step 2: Write golden truth**

Use logical contract `envoy.config-source`. Represent v2 and v3 Proto files. Structural authority is v3. Lifecycle: v2 `frozen`, v3 `active`. Preserve v2 `move_to_package = "envoy.config.core.v3"` and v3 `previous_message_type = "envoy.api.v2.core.ConfigSource"` as lineage evidence. Do not promote deprecated enum values into separate logical contracts.

Lifecycle correctness is initially `null` if Chirograph emits no lifecycle facts; the golden truth remains unchanged until generic lifecycle support arrives.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources envoy/v2-v3-lifecycle/config-source
cargo benchmark envoy/v2-v3-lifecycle/config-source
```

```sh
git add benchmark/envoy
git commit -m "test: add Envoy contract benchmark case"
```

---

### Task 14: Curate Arrow cross-language schema

**Files:**
- Create: `benchmark/arrow/cross-language-schema/field-and-schema/specimen.yaml`
- Create: `benchmark/arrow/cross-language-schema/field-and-schema/golden.yaml`
- Create verbatim files under: `benchmark/arrow/cross-language-schema/field-and-schema/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `arrow/cross-language-schema/field-and-schema`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `apache/arrow@1b0b16a8f68c0082534aa185c0bb4052614df924`:

```text
format/Schema.fbs
cpp/src/arrow/type.h
python/pyarrow/types.pxi
python/pyarrow/includes/libarrow.pxd
```

Record actual SHA-256 values. Do not assume a Java implementation exists in this exact monorepo revision.

- [ ] **Step 2: Write golden truth**

Use the smallest defensible shared contract `arrow.schema.field`. Represent FlatBuffers schema as interchange/spec authority, C++ `Field`/schema semantics as an implementation manifestation, and Python/Cython declarations as language binding manifestations. Encode project/implementation relationships without declaring every Arrow datatype a separate benchmark contract.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources arrow/cross-language-schema/field-and-schema
cargo benchmark arrow/cross-language-schema/field-and-schema
```

```sh
git add benchmark/arrow
git commit -m "test: add Arrow contract benchmark case"
```

---

### Task 15: Curate Temporal multi-dialect persistence

**Files:**
- Create: `benchmark/temporal/multi-dialect-persistence/executions-table/specimen.yaml`
- Create: `benchmark/temporal/multi-dialect-persistence/executions-table/golden.yaml`
- Create verbatim files under: `benchmark/temporal/multi-dialect-persistence/executions-table/fixture/`

**Interfaces:**
- Consumes: Tasks 3-7 framework.
- Produces: case ID `temporal/multi-dialect-persistence/executions-table`.

- [ ] **Step 1: Copy exact upstream bytes**

Use `temporalio/temporal@cd667daadb88d189df0302d8f473858ee7168ce5`:

```text
schema/mysql/v8/temporal/schema.sql
schema/postgresql/v12/temporal/schema.sql
schema/cassandra/temporal/schema.cql
tools/tests/test_data.go
common/persistence/tests/mysql_test_util.go
common/persistence/tests/postgresql_test_util.go
common/persistence/tests/cassandra_test_util.go
```

Record actual SHA-256 values.

- [ ] **Step 2: Write golden truth**

Use logical contract `temporal.persistence.executions`. Represent MySQL, PostgreSQL, and Cassandra schema forms as dialect-specific manifestations. Do not invent one global dialect authority. Encode `equivalent-to` only for the shared logical executions persistence role, and use the test utility references as verification manifestations. Do not promote dialect-only indexes/helper tables into the selected logical contract.

- [ ] **Step 3: Verify/run/commit**

```sh
cargo benchmark --verify-sources temporal/multi-dialect-persistence/executions-table
cargo benchmark temporal/multi-dialect-persistence/executions-table
```

```sh
git add benchmark/temporal
git commit -m "test: add Temporal contract benchmark case"
```

---

### Task 16: Establish reviewed baseline, CI, and methodology docs

**Files:**
- Create: `crates/chirograph-benchmark/src/baseline.rs`
- Create: `crates/chirograph-benchmark/tests/baseline.rs`
- Create: `benchmark/baseline.json`
- Create: `benchmark/README.md`
- Modify: `crates/chirograph-benchmark/src/main.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces:**
- Consumes: all eight cases and `BenchmarkReportV1`.
- Produces: `compare_baseline(current, baseline) -> RegressionResult`; CI command `cargo benchmark all --baseline benchmark/baseline.json`.

- [ ] **Step 1: Write failing baseline tests**

Test these exact transitions:

```text
execution-failure -> scored        allowed improvement
scored -> execution-failure        regression
scored -> invalid-output           regression
higher-is-better metric decreases  regression
false-rate increases               regression
|inflation-1| increases             regression
null lifecycle -> scored lifecycle allowed activation
specimen/golden digest changes      requires baseline update
missing/extra case                  validation failure
```

- [ ] **Step 2: Run red test**

```sh
cargo test -p chirograph-benchmark --test baseline -- --nocapture
```

Expected: FAIL.

- [ ] **Step 3: Implement baseline format/directionality**

Each baseline entry stores case status, metric vector when scored, SHA-256 of `specimen.yaml`, and SHA-256 of `golden.yaml`.

`--write-baseline PATH` is an explicit operator action that writes current truth to the exact path. Ordinary failing runs never mutate baseline.

- [ ] **Step 4: Generate the initial eight-case baseline**

```sh
cargo benchmark all --write-baseline benchmark/baseline.json
cargo benchmark all --baseline benchmark/baseline.json
```

Expected: second command PASS. It is acceptable for cases to be `execution-failure` because the public source analyzer does not yet exist. Do not hand-author observed graphs to avoid that status.

- [ ] **Step 5: Wire hermetic CI**

Add after workspace tests:

```yaml
      - name: Build Chirograph CLI
        run: cargo build --quiet -p chirograph-cli
      - name: Run contract benchmark regression suite
        run: cargo benchmark all --baseline benchmark/baseline.json
```

Do not run `--verify-sources` in normal CI.

- [ ] **Step 6: Write benchmark methodology docs**

`benchmark/README.md` documents data-only fixtures, exact provenance, selectors, metric vector, false-contract rate, macro/micro, baseline ratchet, failure classes, and case-addition rules. Root README adds one concise link; do not duplicate the methodology.

- [ ] **Step 7: Run full verification**

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --quiet -p chirograph-cli
cargo benchmark --list
cargo benchmark all --baseline benchmark/baseline.json
```

Expected: PASS.

Then run the explicit networked provenance check separately:

```sh
cargo benchmark --verify-sources
```

Expected: all committed fixture bytes match their exact upstream revisions.

- [ ] **Step 8: Commit**

```sh
git add crates/chirograph-benchmark/src/baseline.rs crates/chirograph-benchmark/tests/baseline.rs benchmark/baseline.json benchmark/README.md .github/workflows/ci.yml README.md crates/chirograph-benchmark/src/main.rs
git commit -m "ci: gate contract benchmark regressions"
```

---

## Final evidence required before settlement

The implementing worker must provide exact evidence for:

```text
Chirograph commit SHA
all test/format/check/clippy commands
cargo benchmark --list
cargo benchmark all --baseline benchmark/baseline.json
cargo benchmark --verify-sources
all eight upstream revision SHAs
all committed fixture SHA-256 values
case statuses and scored metric vectors
macro/micro aggregates for scored cases
remaining execution-failure cases and the generic capability they await
CI result
```

Do not claim benchmark semantic success from acquisition diagnostics. Do not convert execution failures into bespoke benchmark observations. Settle the Overcenter transition only with evidence from the exact implementation revision.