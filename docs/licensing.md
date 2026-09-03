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

Every benchmark case that incorporates or references third-party material must contain a machine-readable `provenance.json` conforming to [`../benchmarks/provenance.schema.json`](../benchmarks/provenance.schema.json).

The record distinguishes the source repository and exact revision from the way the case is materialized. `bundled` means third-party bytes are committed in the case. `reference-only` means the case records upstream coordinates without committing those bytes. `generated` is reserved for artifacts generated from identified inputs and must still record the source coordinates needed to reproduce them.

SPDX license expressions identify upstream license terms. A per-file license expression may override the source-level expression when an upstream repository contains mixed licensing.

## NOTICE files

Apache-2.0 does not require Chirograph to invent a NOTICE file that merely restates the license. If a distributed third-party work includes attribution notices that must be propagated, preserve those notices as required and add a project-level `NOTICE` only when there is something that downstream redistributors actually need to carry.

## Contributions

Contributions intentionally submitted for inclusion in Chirograph are accepted under Apache-2.0 unless explicitly agreed otherwise before submission. Chirograph uses Developer Certificate of Origin sign-off rather than a project-specific contributor license agreement. See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## Automated checks

`python tools/check_release_metadata.py` verifies the repository's Apache-2.0 Cargo metadata and validates the required shape of any benchmark `provenance.json` records. It is a consistency check, not legal advice and not a substitute for reviewing an upstream license before copying third-party material.
