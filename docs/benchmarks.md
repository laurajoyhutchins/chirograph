# Benchmark methodology

Chirograph's benchmark is a curated corpus of known contract situations. Its job is to measure general analysis behavior against pinned evidence, not to reward repository-specific recognition.

## Two independent dimensions

Every case names both a **specimen** and a **phenomenon**.

```text
benchmarks/
  <specimen>/
    <case>/
      provenance.json
      expected.json
      fixture/...
```

The runner should discover cases generically and support aggregation in either direction:

```text
by phenomenon: schema-enum-drift -> every specimen exercising that phenomenon
by specimen:   cargo             -> every Cargo case
```

That means adding another `schema-enum-drift` case does not require changing a Cargo-specific registry, and adding another Cargo case does not require a new invocation path.

## What belongs in a case

A case may contain pinned source excerpts or generated artifacts when their upstream terms permit redistribution, expected contract outcomes, and metadata describing the phenomenon. Specimen-specific facts and expected stances belong in the case data. Specimen-specific production code does not belong in the benchmark harness or language adapters.

Each case that uses third-party material must include `provenance.json` conforming to [`../benchmarks/provenance.schema.json`](../benchmarks/provenance.schema.json). Source revisions and bundled-file digests are part of benchmark identity.

## Ground truth

Ground truth should describe the contract situation being tested and the evidence needed to recognize it. It should be reviewable without trusting the current Chirograph output. When a case is derived from an upstream bug, schema mismatch, documented invariant, or executable behavior, record the exact upstream revision and the relevant source locations.

A benchmark should preserve genuine contradiction. Do not rewrite a fixture until Chirograph agrees with it.

## Scoring

Score atomic expectations first, then aggregate. At minimum, keep these separable:

- acquisition: was the relevant evidence found with correct provenance?
- representation: were the intended contract representations modeled?
- assessment: was the expected consistent/contested relationship derived?
- diagnostic fidelity: were unknown or unsupported conditions reported without invented certainty?

Report counts as well as percentages so small categories are not visually inflated. Aggregate by phenomenon and by specimen independently. A global score may be useful as a summary, but it must not hide a regression in a specific phenomenon.

## Retrieval is not implicitly scored

Pinned fixture content is the benchmark input by default. Fetching source live at runtime can be a useful demo and an integration test, but network retrieval is not part of benchmark scoring unless a benchmark explicitly targets retrieval correctness. This keeps benchmark results reproducible and separates Chirograph analysis quality from network availability.

## Reproducibility

A published result should identify:

- Chirograph revision
- benchmark corpus revision
- each upstream specimen revision
- content digests for bundled third-party files
- adapter versions or workspace revision
- exact command used
- scoring schema version

The corpus should make it possible to reproduce a historical score without silently following a moving upstream branch.
