# Chirograph benchmark corpus

This directory contains generic benchmark infrastructure and, as the corpus grows, curated cases representing known software-contract situations.

Cases are organized by specimen and case name, but each case also declares a phenomenon so results can be aggregated independently by either dimension. See [`../docs/benchmarks.md`](../docs/benchmarks.md) for methodology.

A third-party case must include `provenance.json` conforming to [`provenance.schema.json`](provenance.schema.json). Do not add upstream source bytes until their redistribution terms have been established and recorded.

The benchmark harness must remain specimen-agnostic. It may discover metadata, execute registered general adapters, compare ordinary Chirograph evidence with expected outcomes, and aggregate scores. It must not contain code paths that recognize a repository merely to produce its expected answer.
