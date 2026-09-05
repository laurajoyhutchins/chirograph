# Benchmark methodology

Chirograph's contract benchmark is a curated, data-only corpus of known cross-representation contract situations. It measures general analysis behavior against reviewed ground truth. Benchmark cases may contain specimen-specific facts; the benchmark runner and production analyzers must not contain specimen-specific recognition code.

## Corpus layout

Cases live at a fixed depth:

```text
benchmark/
  <repository>/
    <scenario>/
      <case>/
        specimen.yaml
        golden.yaml
        fixture/...
```

`specimen.yaml` identifies the upstream repository, an exact 40-character revision, every bundled fixture path, the corresponding upstream path, and its SHA-256 digest. `golden.yaml` describes the reviewed logical contracts, representations, authority claims, relationships, clauses, findings, lifecycle expectations, and known non-contracts for that case.

The runner discovers cases generically. Selectors let the same corpus be evaluated by repository, scenario, repository/scenario, or individual case without adding per-specimen invocation code.

```sh
cargo benchmark --list
cargo benchmark all
cargo benchmark cargo
cargo benchmark scenario:schema-enum-drift
cargo benchmark kubernetes/go-protobuf-openapi/core-v1-pod
```

## Ground truth

Golden truth should be as narrow as the situation under test. It must be reviewable without trusting Chirograph's current output and must preserve genuine contradiction, ambiguity, and known negatives.

A dependency or neighboring type is not automatically another logical contract. The Kubernetes Pod case, for example, deliberately includes a large surrounding API surface while golden truth stays centered on one Pod contract. This makes false-contract restraint measurable rather than rewarding broad name harvesting.

Case directories are data. Executable case-specific glue is rejected by the corpus model.

## Scoring

Each successfully decoded graph is scored by typed identity. The report keeps separate metrics for:

- contract precision, recall, and F1;
- false-contract rate and contract inflation;
- authority correctness;
- relationship precision and recall;
- lifecycle correctness when lifecycle is observed;
- expected-finding precision and recall;
- diagnostics for unsupported or unobserved conditions.

Counts remain attached to ratios. A case that cannot execute or emits invalid graph JSON remains an explicit `execution-failure` or `invalid-output`; failures are never converted into zero-quality scored results.

## Reviewed baseline semantics

`benchmark/baseline.json` is a reviewed regression floor, not a target score. Each entry stores the full case result plus SHA-256 digests of that case's `specimen.yaml` and `golden.yaml`. A corpus change therefore cannot be compared silently against stale ground truth.

Baseline comparison is directional:

- `execution-failure` or `invalid-output` becoming `scored` is an improvement;
- `scored` becoming a failure is a regression;
- higher-is-better quality metrics may increase but may not decrease;
- false-contract rate may decrease but may not increase;
- contract inflation may move toward `1.0` but not farther away;
- changed specimen or golden digests fail closed and require explicit baseline review.

To create a candidate baseline intentionally:

```sh
cargo benchmark all --write-baseline /tmp/baseline.json
```

Review the corpus and result changes before replacing `benchmark/baseline.json`. Do not update the baseline merely to make CI green.

## Hermetic CI

Normal CI analyzes only the committed fixtures. It explicitly builds the public CLI and then runs the benchmark against the reviewed baseline:

```sh
cargo build --quiet -p chirograph-cli
cargo benchmark all \
  --baseline benchmark/baseline.json \
  --chirograph-bin target/debug/chirograph
```

This path has no live source fetch. Benchmark failures therefore describe Chirograph behavior or committed-corpus drift rather than network availability.

## Source verification and refresh

Live upstream access is a maintenance operation, not part of normal benchmark scoring or CI. Use it when curating or auditing specimens:

```sh
cargo benchmark --verify-sources
cargo benchmark --verify-sources kubernetes/go-protobuf-openapi/core-v1-pod
cargo benchmark --refresh SELECTOR --revision EXACT_SHA
```

Source verification fetches the exact pinned revision and requires byte-for-byte equality with every committed fixture and its declared SHA-256. Refresh requires an exact revision and updates fixture/provenance data; golden truth remains a separate review decision.

## Reproducibility

A reproducible result is bound by:

- the Chirograph revision;
- the benchmark corpus revision;
- each case's exact upstream revision and fixture digests;
- the `specimen.yaml` and `golden.yaml` digests recorded in the baseline;
- the public `chirograph analyze <fixture> --format graph-json` process boundary;
- the benchmark scoring and baseline schema versions.

The benchmark intentionally separates retrieval correctness, contract analysis, scoring, and regression policy so a failure in one layer is not mislabeled as another.
