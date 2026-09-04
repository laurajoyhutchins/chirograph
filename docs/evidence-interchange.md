# Evidence interchange

`chirograph-evidence-v1` is the versioned, language-neutral interchange between acquisition and Chirograph's contract graph.

The executable parser in `crates/chirograph-core/src/evidence.rs` is authoritative for the exact wire shape. This document describes the compatibility contract around it rather than duplicating every Rust type.

## Document shape

A v1 evidence document contains these collections:

```json
{
  "schema": "chirograph-evidence-v1",
  "sources": [],
  "contracts": [],
  "representations": [],
  "observations": [],
  "clauses": [],
  "clause_assertions": [],
  "relations": [],
  "authority_claims": []
}
```

The parser rejects unsupported schema identifiers, invalid enum values, malformed identifiers, unknown fields in the v1 wire objects, and graphs that violate the model invariants.

## Evidence versus interpretation

`Source` and `Observation` are the provenance-bearing factual layer. Contracts and representations organize those facts. Clause assertions, relations, and authority claims are interpretations and must cite the observations that justify them.

Adapters must not hide interpretation inside an observation merely to make it appear mechanically certain. For example, an observation can record that a validator rejects a concrete input. A separate assertion can use that observation as evidence for a contract clause.

See [`model.md`](model.md) for the full epistemic boundary and graph invariants.

## Pre-alignment state is separate

`chirograph-evidence-v1` remains a post-alignment format: every `representation` in it is already assigned to a logical contract. Observed artifacts whose contract membership is still being evaluated are represented separately in `chirograph-alignments-v1`.

That separation is intentional. Adding candidate representations or unresolved alignment claims to evidence-v1 would silently change the meaning of its `representations` collection. See [`alignment-interchange.md`](alignment-interchange.md) for the pre-alignment wire contract.

## Revision identity

Every observation carries one of three revision states:

- `exact` with an explicit value
- `unversioned`
- `unknown`

An adapter should use `exact` only when it can identify the observed source bytes or behavior at that coordinate. A branch name such as `main` is not an exact revision unless the evidence also records the exact resolved revision used for observation.

## Compatibility

Consumers should dispatch on the `schema` value before interpreting the rest of the document. A change that cannot be represented without changing the meaning or accepted shape of v1 should introduce a new schema identifier rather than silently changing v1 semantics.

Producers should emit only fields defined by the target schema. V1 deliberately rejects unknown wire fields so a consumer cannot silently ignore evidence that the producer considered semantically important.

## Determinism

Given the same source bytes, acquisition configuration, revision coordinates, and adapter version, adapters should produce semantically identical evidence. Stable ordering is preferred so evidence can be diffed and hashed without incidental churn.

Network retrieval, repository discovery, model calls, and runtime execution are acquisition concerns. They should not be required merely to parse an already-materialized evidence document.
