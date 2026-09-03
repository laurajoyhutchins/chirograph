# Chirograph v0.2 Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Chirograph's contract model into one end-to-end read-only workflow that accepts neutral evidence, explains it from the CLI, adapts Overcenter's existing contract evidence, and proves cross-representation drift detection.

**Architecture:** Keep the Rust kernel authoritative for validation and assessment. Add a versioned JSON interchange at the core boundary, then make the CLI consume only that interchange. Overcenter-specific conversion remains an adapter outside the kernel, and the final fixture deliberately introduces disagreement so the generic engine must derive `Contested` from evidence rather than repo-specific knowledge.

**Tech Stack:** Rust 2024, serde/serde_json, existing GitHub Actions Rust CI, Node.js only for the Overcenter adapter because the source catalog is already JavaScript/JSON-native.

**Spec:** `docs/model.md`

## Global Constraints

- Chirograph remains read-only with respect to analyzed repositories.
- Source observations remain distinct from derived interpretations.
- Contract authority remains facet-scoped and evidence-backed.
- `Consistent` means no recorded contradiction, not proven truth.
- `Contested` preserves disagreement; no vote-counting or automatic winner selection.
- Adapters emit the neutral interchange and do not add repo-specific concepts to `chirograph-core`.

---

### Task 1: Versioned evidence interchange

**Files:**
- Create: `crates/chirograph-core/src/evidence.rs`
- Modify: `crates/chirograph-core/src/lib.rs`
- Modify: `crates/chirograph-core/src/model.rs`
- Modify: `crates/chirograph-core/Cargo.toml`
- Test: `crates/chirograph-core/tests/evidence.rs`
- Document: `docs/evidence-v1.md`

**Interfaces:**
- Produces: `EVIDENCE_SCHEMA_V1: &str`, `parse_evidence_json(&str) -> Result<ContractGraph, EvidenceError>`, and `render_evidence_json_pretty(&ContractGraph) -> Result<String, EvidenceError>`.
- Wire enums use stable snake_case names. `Revision` uses `{ "kind": "exact", "value": "..." }`, `{ "kind": "unversioned" }`, or `{ "kind": "unknown" }`.
- IDs remain validated through their existing constructors during deserialization.

- [ ] **Step 1: Write failing interchange tests**

Use `crates/chirograph-core/tests/evidence.rs` to require valid parsing, unsupported-schema rejection, graph validation after parse, and round-trip rendering.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p chirograph-core --test evidence`

Expected: compilation fails because `chirograph_core::evidence` does not exist.

- [ ] **Step 3: Implement minimal v1 wire support**

Add serde serialization for the existing model without changing model semantics. Implement a versioned document with exactly these top-level fields:

```text
schema
sources
contracts
representations
observations
clauses
clause_assertions
relations
authority_claims
```

`parse_evidence_json` must deserialize, require schema `chirograph-evidence-v1`, call `ContractGraph::validate`, and return the validated graph. `render_evidence_json_pretty` must validate first and then emit pretty JSON in the same v1 shape.

- [ ] **Step 4: Verify GREEN**

Run:

```sh
cargo test -p chirograph-core --test evidence
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Document the wire contract**

Create `docs/evidence-v1.md` with the schema identifier, field meanings, revision encoding, epistemic boundary, and forward-compatibility rule: unknown schema versions fail closed.

---

### Task 2: `chirograph inspect`

**Files:**
- Modify: `crates/chirograph-cli/Cargo.toml`
- Replace: `crates/chirograph-cli/src/main.rs`
- Test: `crates/chirograph-cli/tests/inspect.rs`
- Create fixture: `crates/chirograph-cli/tests/fixtures/consistent.json`

**Interfaces:**
- Consumes: `parse_evidence_json` and `ContractGraph::assess_clause`.
- Produces CLI: `chirograph inspect <evidence.json>`.
- Exit 0 for a valid graph, including contested clauses. Nonzero only for invalid invocation or invalid evidence.

- [ ] **Step 1: Write a failing CLI integration test**

The fixture contains one contract with one supported clause. Assert stdout contains:

```text
example.contract
semantic
example.requirement
CONSISTENT
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p chirograph-cli --test inspect`

Expected: failure because `inspect` is not implemented.

- [ ] **Step 3: Implement argument parsing without a framework**

Accept exactly `inspect <path>` plus `--help` and `--version`. Read UTF-8 JSON from disk, call `parse_evidence_json`, and render contracts in deterministic ID order. For each clause print facet, clause ID, status, supporting representations, and contradicting representations when present. Print facet-scoped authority claims with basis and representation ID.

- [ ] **Step 4: Verify GREEN**

Run the CLI integration test and the full workspace CI commands from Task 1.

---

### Task 3: Overcenter evidence adapter

**Files:**
- Create: `adapters/overcenter/package.json`
- Create: `adapters/overcenter/convert.mjs`
- Create: `adapters/overcenter/convert.test.mjs`
- Create fixture: `adapters/overcenter/fixtures/catalog.json`
- Create fixture: `adapters/overcenter/fixtures/classifications.json`
- Document: `adapters/overcenter/README.md`

**Interfaces:**
- Consumes Overcenter `contract-evidence-catalog-v1` plus `contract-evidence-classifications-v1` shapes.
- Produces `chirograph-evidence-v1` JSON to stdout.
- Candidate `source_identity` becomes a Chirograph representation ID and source locator. Classified `logical_contract` becomes `ContractId`. `projection_of` becomes a `Projects` relation. Overcenter relationships map: `consumes -> DependsOn`, `produces -> Defines`, `persists-as -> Projects`, `derives-from -> DependsOn`, `verified-by -> Validates`, `compatibility-for -> DependsOn`.
- The adapter does not invent clauses when the Overcenter catalog lacks clause-level evidence; it emits topology and authority evidence only.

- [ ] **Step 1: Write failing adapter tests**

Require conversion of one authority plus one projection and one relationship into a valid `chirograph-evidence-v1` document.

- [ ] **Step 2: Verify RED**

Run: `node --test adapters/overcenter/convert.test.mjs`

Expected: failure because `convert.mjs` does not exist.

- [ ] **Step 3: Implement the minimal converter**

Read two JSON paths from argv, validate only the schema identifiers and fields needed for conversion, produce deterministic sorted arrays, and write JSON to stdout. Do not copy Overcenter lifecycle/significance/SemVer concepts into the Chirograph kernel.

- [ ] **Step 4: Verify the adapter through Chirograph**

Pipe the generated file into `cargo run -p chirograph-cli -- inspect <file>` in CI/test harness and require successful parsing.

---

### Task 4: Prove cross-representation drift

**Files:**
- Create fixture: `fixtures/drift/retry-safety.json`
- Test: `crates/chirograph-cli/tests/drift.rs`
- Document: `fixtures/drift/README.md`

**Interfaces:**
- Consumes only `chirograph-evidence-v1`; no fixture-specific code is allowed in core or CLI.
- Produces an observable `CONTESTED` assessment with exact supporters and contradictors.

- [ ] **Step 1: Write the failing drift test**

The fixture models one `Recovery + Guarantee` clause:

```text
"retry after an indeterminate transport failure does not duplicate the mutation"
```

Documentation and a test representation support it; a runtime representation contradicts it. Assert CLI output contains `CONTESTED`, both supporting representation IDs, and the runtime contradictor ID.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p chirograph-cli --test drift`

Expected: failure until the fixture is fully representable and CLI output exposes both sides.

- [ ] **Step 3: Make only generic fixes**

If the test reveals a missing generic concept, change the kernel only when the requirement applies independently of this fixture. Do not add Overcenter-, retry-, or fixture-specific branches.

- [ ] **Step 4: Verify the full vertical slice**

Run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
node --test adapters/overcenter/convert.test.mjs
```

Expected: all pass, and `chirograph inspect fixtures/drift/retry-safety.json` reports `CONTESTED` with preserved evidence on both sides.
