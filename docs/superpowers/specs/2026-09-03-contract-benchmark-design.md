# Chirograph Contract Benchmark

**Date:** 2026-09-03

## Goal

Create a formal, in-repository benchmark that measures whether Chirograph reconstructs the right logical contract graph from real source code.

The benchmark is not a parser coverage contest and is not scored by the number of symbols, observations, candidates, or apparent contracts Chirograph can emit. It evaluates final semantic reconstruction against small, human-reviewed golden truths.

The benchmark should answer:

> Given these exact source bytes, how accurately does Chirograph recover the real logical contracts, their facet-scoped authority, manifestations, relationships, lifecycle, and findings without inventing unsupported contractual structure?

A central quality property is restraint. An analyzer that finds every real contract but promotes ordinary implementation structure into thousands of false contracts is not useful.

## Core design choice

Keep the benchmark inside the Chirograph repository under one root-level `benchmark/` directory.

```text
chirograph/
├── adapters/                 # generic source acquisition
├── crates/                   # Chirograph product and benchmark tooling
├── benchmark/                # data-only curated evaluation corpus
└── docs/
```

Do not create a second root-level `specimens/` hierarchy for benchmark cases. Curated evaluation cases, their source fixtures, provenance, configuration, and golden truth all belong under `benchmark/`.

The benchmark corpus is data-only. It contains no repository-specific executable code, handwritten extractors, framework adapters, or semantic glue.

## Architectural invariant: no benchmark cheat codes

A benchmark case may configure existing public Chirograph capabilities, but it may not teach Chirograph how a repository works.

The corpus must not contain code such as:

- a Pydantic-specific observer;
- a Kafka symbol map;
- a Cargo authority hint implemented as code;
- a Rails schema interpreter used only by the benchmark;
- a repository-specific script that assembles the expected graph;
- custom runtime probes that call framework APIs directly.

If a case requires special code for Chirograph to understand it, that is evidence of a missing general Chirograph capability. The benchmark should expose that gap rather than hide it.

This pressure is intentional. Benchmark failures should improve Chirograph itself.

## Corpus shape

Use three structural identity dimensions:

```text
benchmark/<repository>/<scenario>/<case>/
```

For example:

```text
benchmark/
├── cargo/
│   └── schema-enum-drift/
│       └── <case>/
├── kafka/
│   └── message-spec-generation/
│       └── <case>/
├── kubernetes/
│   └── go-protobuf-openapi/
│       └── <case>/
├── pydantic/
│   └── validation-vs-serialization/
│       └── <case>/
├── rails/
│   └── migration-db-schema-authority/
│       └── <case>/
├── envoy/
│   └── v2-v3-lifecycle/
│       └── <case>/
├── arrow/
│   └── cross-language-schema/
│       └── <case>/
└── temporal/
    └── multi-dialect-persistence/
        └── <case>/
```

The initial corpus must establish all eight repositories and scenarios, with at least one manually reviewed case in each.

A case directory contains only benchmark data:

```text
<case>/
├── specimen.yaml
├── golden.yaml
└── fixture/
    └── <verbatim upstream files>
```

`repository`, `scenario`, and canonical case `id` are first-class benchmark dimensions. If they are repeated in `specimen.yaml`, the runner must validate that they agree exactly with the directory path so there is only one effective identity.

The canonical case ID is the path relative to `benchmark/`, for example:

```text
cargo/schema-enum-drift/profile-debug-info
```

## Fixture policy

Fixture files are verbatim upstream source files.

Do not hand-reduce files to toy excerpts. Reduction can accidentally remove contextual signals that a general analyzer should be able to use, such as comments, generated-file markers, neighboring declarations, imports, version annotations, or lifecycle clues.

Each case should include the smallest useful set of complete upstream files, but every included file must preserve its upstream bytes exactly.

Ordinary benchmark execution is hermetic and offline. The benchmark does not clone or fetch upstream repositories while scoring.

## Provenance

Every specimen records exact upstream provenance.

Conceptually:

```yaml
id: cargo/schema-enum-drift/profile-debug-info
repository: cargo
scenario: schema-enum-drift

upstream:
  repository: rust-lang/cargo
  revision: <exact 40-character commit SHA>

files:
  - fixture_path: fixture/src/cargo/core/profiles.rs
    upstream_path: src/cargo/core/profiles.rs
    sha256: <digest of verbatim fixture bytes>
```

The exact pinned revision and per-file digest are required facts, not documentation hints.

The three authorities remain separate:

```text
upstream repository @ exact SHA  -> authority for fixture bytes
golden.yaml                      -> human-reviewed benchmark truth
Chirograph output                -> system under evaluation
```

## Source verification and refresh

Live remote retrieval is not part of the benchmark score.

Provide explicit maintenance operations that may contact the upstream repository:

```text
cargo benchmark --verify-sources
cargo benchmark --refresh <selector>
```

`--verify-sources` checks that vendored fixture bytes match the paths at the pinned upstream revision.

`--refresh` fetches source from an explicitly selected exact upstream revision and updates fixture bytes and provenance metadata. It must never silently rewrite `golden.yaml`.

Changing golden truth is always a separate human-reviewed change.

Runtime retrieval from a real remote repository can later become a Chirograph demo or end-to-end integration feature. That demo answers a different question: whether Chirograph can find and reconstruct the same contract situation starting from a live repository. It must not contaminate the hermetic semantic benchmark.

## Static v1 boundary

Benchmark v1 is static.

It evaluates Chirograph from repository/source inputs only. It does not define framework-specific or specimen-specific runtime probes.

Runtime evidence remains important, but it must first exist as a general public Chirograph capability. If Chirograph later gains a generic runtime-observation mechanism, benchmark cases may opt into that normal product capability without gaining custom executable code.

Until then, inability to establish a runtime-only truth is a legitimate benchmark limitation or failure. The benchmark should not make the first scoreboard prettier by adding special paths.

## Public analysis boundary

The benchmark must exercise the same public analysis path a normal user would exercise.

Target product interface:

```text
chirograph analyze <source-tree> --format graph-json
```

The exact command spelling may evolve during implementation, but the architectural requirement does not: the benchmark runner receives a canonical final Chirograph graph from a public general analysis entry point.

The benchmark runner must not privately orchestrate language adapters, invoke repository-specific commands, or assemble semantic evidence itself.

```text
verbatim fixture
      |
      v
public Chirograph analysis
      |
      v
canonical final graph
      |
      v
benchmark scorer
      |
      v
golden.yaml
```

If a supported case cannot be processed through the public analysis boundary, that is a product capability gap.

## Adapter boundary

Generic adapters remain outside `benchmark/` and are repository-agnostic.

Examples include:

```text
adapters/rust/
adapters/java/
adapters/python/
adapters/go/
adapters/ruby/
adapters/cpp/
adapters/proto/
```

Adapters acquire deterministic source facts. They do not contain Cargo, Kafka, Kubernetes, Pydantic, Rails, Envoy, Arrow, or Temporal knowledge merely to satisfy the benchmark.

Repository-specific acceptance success must arise from general acquisition plus general Chirograph analysis.

## Benchmark tooling boundary

Create a small executable benchmark tool as ordinary repository code, conceptually a workspace crate such as:

```text
crates/chirograph-benchmark/
```

It owns only evaluation mechanics:

- corpus discovery and validation;
- selector resolution;
- invoking the public Chirograph analysis boundary;
- parsing canonical graph output;
- golden comparison;
- metric calculation;
- aggregation;
- baseline regression comparison;
- deterministic human-readable and machine-readable reports;
- explicit fixture source verification/refresh maintenance commands.

It does not own product contract semantics or repository-specific interpretation.

`benchmark/` remains data-only even though the scorer is executable.

## Developer interface

The official developer interface should not require raw benchmark paths.

Use a Cargo alias backed by the benchmark executable so common invocations are concise:

```text
cargo benchmark all
cargo benchmark --list
cargo benchmark cargo
cargo benchmark scenario:schema-enum-drift
cargo benchmark cargo/schema-enum-drift
cargo benchmark cargo/schema-enum-drift/profile-debug-info
```

Selectors operate on first-class benchmark dimensions:

- `all` selects the entire corpus;
- a repository selects every case for that repository;
- `scenario:<name>` selects the same contract situation across repositories;
- `<repository>/<scenario>` selects all cases for that intersection;
- the full canonical ID selects one case.

This makes two equally important aggregate views natural:

```text
by repository
  cargo
  kafka
  kubernetes
  ...

by scenario
  schema-enum-drift
  lifecycle
  authority-resolution
  ...
```

Adding more Cargo cases does not require a new invocation style, and adding the same scenario to several repositories allows direct cross-repository evaluation.

## Golden truth

`golden.yaml` describes semantic truth, not syntax inventory.

It should use Chirograph's own logical vocabulary wherever that vocabulary exists rather than inventing a parallel benchmark ontology.

Conceptually:

```yaml
contracts:
  - id: cargo.profile.debug-info
    facets:
      - structural
      - executable

    authority_claims:
      - facet: structural
        representation: cargo.profile.schema

    manifestations:
      - id: cargo.profile.schema
      - id: cargo.profile.implementation

    relationships:
      - kind: projects-to
        target: cargo.manifest.profile.debug-info

    lifecycle:
      status: active

    clauses:
      - ...

    expected_findings:
      - ...

non_contracts:
  - source: ...
    reason: implementation-detail
```

`non_contracts` is benchmark-only evaluation truth. It exists specifically to test whether Chirograph incorrectly promotes ordinary implementation structure into logical contracts.

Golden truth must remain small enough for a human reviewer to audit.

## Matching rule

Matching is strict in v1.

Canonical logical identity is not rescued by fuzzy similarity.

```text
observed logical ID
      |
      +-- exact golden identity -> compare semantics
      |
      +-- no golden match       -> false contract
```

If Chirograph cannot reconstruct the same logical identity deterministically, the benchmark should expose that defect instead of hiding it behind approximate matching.

Every emitted logical contract that cannot be matched to golden truth counts as false by default.

The analyzer does not get to classify its own surprises as valid.

If human review establishes that an unmatched emission is genuinely contractual, the corpus is amended explicitly. Until that review lands, the current run remains scored as a false contract.

## Scoring model

The authoritative score is against the final reconstructed Chirograph graph.

Parser facts, AST node counts, candidate ranking, acquisition coverage, and similar internals are diagnostics only.

### Contract metrics

- **Contract precision** = matched emitted contracts / all emitted contracts.
- **Contract recall** = matched golden contracts / all golden contracts.
- **Contract F1** = harmonic mean of contract precision and recall.
- **False contract rate** = unmatched emitted contracts / all emitted contracts.
- **Contract inflation ratio** = emitted logical contracts / golden logical contracts.

False contract rate is a headline metric even though it is closely related to precision. It names the operational failure Chirograph particularly needs to avoid.

For example:

```text
golden contracts:      25
correctly recovered:   23
emitted contracts:    500
false contracts:      477

recall:               92.0%
precision:             4.6%
false contract rate:  95.4%
contract inflation:     20x
```

A 92% recall result in this situation is not success.

### Authority correctness

Score authority per contract facet.

Different representations may legitimately govern different facets of one logical contract. A contract with three scored authority facets and two correct authorities receives 2/3 authority correctness rather than one all-or-nothing result.

### Relationship correctness

Compare relationships as typed graph edges:

```text
(source logical identity, relationship kind, target logical identity)
```

Correct endpoints with the wrong relationship kind are incorrect.

Report relationship precision and relationship recall.

### Lifecycle correctness

Compare lifecycle classifications against golden truth using Chirograph's stable lifecycle vocabulary once available.

Finding both an old and new representation is insufficient when the benchmark truth says one is deprecated, compatibility-only, historical, generated, superseded, or otherwise not equivalent in current authority.

### Finding correctness

Score findings independently from graph reconstruction.

Report finding precision and finding recall.

This separation is important because Chirograph may reconstruct the relevant contracts correctly while falsely announcing drift between intentional perspectives, or may identify a real inconsistency while missing other graph structure.

### Unclassified-but-real review metric

Keep `unclassified-but-real` fail-closed.

It is not an analyzer-selected exemption from false-contract scoring. Every unmatched contract is initially false.

When human review later determines that an unmatched discovery is genuinely contractual but exposes a gap in Chirograph's current ontology or golden corpus, that adjudication is recorded in benchmark truth. Corpus history can then report the rate at which unmatched discoveries were later promoted as real.

This metric therefore measures taxonomy/corpus incompleteness without allowing current runs to self-certify surprising output.

## No composite score in v1

Do not introduce one weighted overall score initially.

Report a metric vector instead:

```text
contract     P=.94 R=.91 F1=.92
authority    .89
relations    P=.93 R=.87
lifecycle    .96
findings     P=.88 R=.84
false-rate   .04
inflation    1.03x
```

A composite score would force arbitrary weights before the corpus provides evidence about which failures matter most and would encourage optimization toward the number rather than Chirograph's actual behavior.

## Aggregation

Report four levels:

```text
case
repository
scenario
overall corpus
```

For count-based metrics, expose both macro and micro aggregation.

- **Macro** averages per-case scores so one large case cannot drown out several smaller failures.
- **Micro** pools predictions and truths across cases to describe total system behavior.

Macro is the headline aggregate view. Micro remains a secondary diagnostic.

Repository and scenario aggregation must both be first-class so failures can be distinguished between repository/language weaknesses and contract-reasoning weaknesses.

## CI policy

Initial CI should gate regressions, not aspirational thresholds.

Do not begin with rules such as "contract F1 must be at least 0.90" across a corpus deliberately chosen to expose missing capabilities.

Instead, check in an explicit benchmark baseline containing each case's current execution status and score vector. CI compares current results to that baseline.

The important first invariant is:

> A change may not silently make Chirograph worse on known benchmark truth.

Examples:

- a scored case becoming an execution failure is a regression;
- a metric falling below its checked-in baseline is a regression unless the golden corpus intentionally changed;
- a previously unsupported case becoming executable and scored is an improvement;
- increasing expected truth through human-reviewed golden changes may legitimately lower raw scores and must update the reviewed baseline explicitly.

As capabilities mature, individual case baselines ratchet upward.

The baseline is evaluation data, never inferred from a failing run and silently accepted.

## Failure semantics

Distinguish three failure classes:

```text
case cannot run
  -> execution failure

case runs but final graph violates Chirograph invariants
  -> invalid-output failure

case runs and graph is valid but differs from golden truth
  -> scored semantic mismatch
```

These must remain distinct in human-readable and machine-readable output.

Malformed benchmark data is a fourth class: corpus-validation failure. Scoring must not proceed when benchmark truth or provenance is internally inconsistent.

## Corpus self-validation

Before analysis, validate mechanically:

- unique canonical case IDs;
- valid repository/scenario/case dimensions;
- path and metadata identity agreement;
- exact upstream revision shape;
- every declared fixture exists;
- every fixture has a digest;
- no undeclared fixture bytes influence analysis;
- golden references are internally consistent;
- golden IDs are unique within their required scope;
- no executable files or specimen-specific source code exist under `benchmark/`;
- selectors resolve deterministically;
- baseline entries refer only to existing cases.

Corpus validation is fail-closed.

## Testing strategy

Use TDD for benchmark tooling.

### Scorer unit tests

Use tiny synthetic canonical graphs and golden truths to cover:

- perfect match;
- missing contracts;
- extra false contracts;
- zero-emission cases;
- authority partial correctness by facet;
- wrong relationship kind;
- missing and extra relationships;
- lifecycle mismatch;
- finding false positives and false negatives;
- inflation calculations;
- macro versus micro aggregation;
- malformed golden data;
- invalid Chirograph graph output;
- baseline regression and improvement behavior.

### Corpus validation tests

Require deterministic rejection of:

- duplicate IDs;
- metadata/path disagreement;
- malformed upstream revisions;
- missing fixture digests;
- executable code in the benchmark corpus;
- broken golden references;
- ambiguous selectors.

### Real-corpus integration tests

The eight initial curated cases are integration tests of Chirograph's actual public analysis path.

They are not substitutes for adapter unit tests. Adapter tests prove acquisition mechanics; benchmark cases prove final contract reconstruction quality.

## Initial corpus

The first benchmark version establishes these eight repository/scenario pairs:

1. **Cargo / `schema-enum-drift`** - representation drift around a logical configuration contract.
2. **Kafka / `message-spec-generation`** - authority and relationships across message/spec generation.
3. **Kubernetes / `go-protobuf-openapi`** - a multi-representation Go, protobuf, and OpenAPI chain.
4. **Pydantic / `validation-vs-serialization`** - intentional perspective differences that must not become false drift.
5. **Rails / `migration-db-schema-authority`** - historical migrations versus current schema/database authority.
6. **Envoy / `v2-v3-lifecycle`** - active versus compatibility/deprecated API lifecycle.
7. **Arrow / `cross-language-schema`** - one logical schema represented across language boundaries.
8. **Temporal / `multi-dialect-persistence`** - persistence contracts represented in multiple database dialects.

Each initial scenario gets at least one concrete case with verbatim source, exact revision provenance, explicit non-contract truth where useful, and a manually reviewed golden graph.

The purpose of this set is not repository popularity. Each case should exercise a different way a contract analyzer can be wrong.

## Demo boundary

A future demo may intentionally do more work:

```text
remote repository @ exact revision
        |
        v
source retrieval
        |
        v
generic Chirograph analysis
        |
        v
reconstructed graph
        |
        v
compare/explain against benchmark truth
```

That feature can demonstrate end-to-end retrieval and reconstruction over real repositories.

It is separate from the benchmark because retrieval quality, remote availability, repository layout, authentication, dependency installation, and network behavior are different concerns from semantic reconstruction of controlled source bytes.

The same benchmark golden truth may be reused to evaluate the demo without making live retrieval part of the core benchmark score.

## Non-goals

Benchmark v1 does not attempt:

- live upstream retrieval during ordinary scoring;
- repository-specific executable benchmark code;
- framework-specific runtime probes;
- exhaustive repository coverage;
- exhaustive parser coverage;
- fuzzy logical-contract identity matching;
- automatic promotion of surprising analyzer output;
- one weighted overall quality score;
- replacing adapter unit tests;
- proving Chirograph's golden truth is philosophically final.

## Success criteria

The design is successful when:

1. `benchmark/` is a data-only in-repository corpus with no specimen-specific code.
2. All eight initial repository/scenario pairs have at least one reviewed case.
3. Fixture files are verbatim, exact-revision pinned, digest-verified upstream bytes.
4. Ordinary benchmark scoring is offline and deterministic.
5. The benchmark invokes a public general Chirograph analysis path rather than wiring adapters privately.
6. The scorer evaluates the final reconstructed graph, not acquisition volume.
7. Unmatched emitted contracts count as false until humans change benchmark truth.
8. False contract rate and contract inflation are visible headline metrics alongside precision and recall.
9. Authority, typed relationships, lifecycle, and findings are scored independently.
10. Repository and scenario selectors work independently and aggregate cleanly.
11. Macro aggregation is the headline corpus view, with micro aggregation available as a diagnostic.
12. CI fails on benchmark regressions while allowing known unsupported cases to remain visible and ratchet upward over time.
13. Fixture source verification/refresh can contact upstream explicitly without altering golden truth automatically.
14. Improving a benchmark case requires improving a general Chirograph capability or correcting human-reviewed truth, never adding bespoke benchmark glue.
