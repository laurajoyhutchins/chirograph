# Licensing and provenance policy

Chirograph's project-wide default license is Apache-2.0. That default applies to Chirograph-authored source code, documentation, schemas, adapters, tests, and benchmark infrastructure unless a file or accompanying provenance record explicitly says otherwise.

The root [`LICENSE`](../LICENSE) contains the Apache License 2.0 terms. Rust packages inherit the SPDX expression `Apache-2.0` from workspace metadata.

## Third-party material is not relicensed

Putting upstream source in this repository does not convert it to Apache-2.0. Third-party files retain their original copyright, license, attribution, and NOTICE obligations.

Before bundling upstream bytes:

1. Establish the exact upstream origin and revision.
2. Establish the applicable upstream license for those bytes.
3. Confirm redistribution is allowed for the intended use.
4. Preserve required copyright, license, and attribution notices.
5. Record the exact bundled-file digest.
6. Prefer a reference-only fixture when redistribution rights are unclear.

If these facts cannot be established, do not copy the material into the repository.

## Benchmark provenance

The canonical benchmark root is [`../benchmark/`](../benchmark/). Each case contains `specimen.yaml`, `golden.yaml`, and `fixture/`; there is no parallel `benchmarks/` format. `specimen.yaml` records the upstream repository, exact revision, each bundled fixture's upstream path, and its SHA-256 digest. The benchmark corpus loader validates those identities before scoring.

Third-party license and NOTICE obligations remain properties of the upstream material, not of the benchmark metadata schema. Preserve required notices in the distributed corpus, with repository-level attribution summarized in [`../benchmark/README.md`](../benchmark/README.md). If redistribution rights are unclear, do not commit the bytes merely to make a benchmark convenient.

## NOTICE files

Apache-2.0 does not require Chirograph to invent a NOTICE file that merely restates the license. If a distributed third-party work includes attribution notices that must be propagated, preserve those notices as required and add a project-level `NOTICE` only when there is something that downstream redistributors actually need to carry.

## Contributions

Contributions intentionally submitted for inclusion in Chirograph are accepted under Apache-2.0 unless explicitly agreed otherwise before submission. Chirograph uses Developer Certificate of Origin sign-off rather than a project-specific contributor license agreement. See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## Automated checks

`python tools/check_release_metadata.py` verifies Apache-2.0 Cargo metadata, requires the single canonical `benchmark/` root, and rejects a revived legacy `benchmarks/` directory. The Rust benchmark corpus loader performs the deeper case validation, including exact revisions and fixture SHA-256 identities, and the reviewed baseline binds each case's `specimen.yaml` and `golden.yaml` digests. These are consistency checks, not legal advice and not substitutes for reviewing upstream license obligations before copying third-party material.
