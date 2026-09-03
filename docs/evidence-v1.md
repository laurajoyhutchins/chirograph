# Chirograph evidence v1

`chirograph-evidence-v1` is the first language-neutral interchange format for feeding observed software-contract evidence into Chirograph.

The format is read-only. Producing or consuming a document does not authorize Chirograph to mutate the analyzed repository, runtime, schema registry, database, or API.

## Boundary

```text
extractor / adapter
       │
       ▼
chirograph-evidence-v1
       │
       ▼
validated ContractGraph
       │
       ├── clause assessment
       ├── authority explanation
       └── relationship analysis
```

The interchange carries observations and interpretations already represented by the core model. It deliberately contains no language-specific AST nodes, Postgres-specific metadata, TypeScript compiler objects, or Overcenter-specific lifecycle vocabulary.

## Top-level document

A v1 document contains exactly these top-level fields:

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

All identifier fields use the same non-empty, non-whitespace-normalizing identifiers as the Rust model. Invalid identifiers are rejected during decoding.

## Stable vocabulary

Enum-like values use lower snake case. Examples include:

- facets: `structural`, `executable`, `semantic`, `failure`, `concurrency`, `recovery`, `verification`
- representation kinds: `source_code`, `schema`, `type_definition`, `validator`, `test`, `documentation`, `configuration`, `generated_artifact`
- clause kinds: `requirement`, `guarantee`, `invariant`
- clause stances: `supports`, `contradicts`
- authority bases: `explicit_declaration`, `mechanical_enforcement`, `observed_behavior`, `documentation`, `inference`

Revision identity remains explicit:

```json
{ "kind": "exact", "value": "abc123" }
{ "kind": "unversioned" }
{ "kind": "unknown" }
```

`unknown` is preserved as uncertainty. The interchange never manufactures an exact revision.

Relation endpoints are tagged so contract and representation identities cannot be confused:

```json
{ "kind": "contract", "id": "billing.invoice" }
{ "kind": "representation", "id": "billing.invoice.proto" }
```

## Validation

Decoding has two gates:

1. The document must use the exact supported schema identifier `chirograph-evidence-v1` and conform to the v1 JSON shape.
2. The resulting `ContractGraph` must pass all core model invariants.

A syntactically valid JSON document that names a missing source, cross-contract clause assertion, unsupported facet, evidence-free relation, or otherwise invalid graph is rejected rather than partially accepted.

## Epistemic rule

The wire format does not upgrade evidence quality.

```text
observation             remains an observation
supports / contradicts remains an interpretation
facet authority claim  remains an evidence-backed claim
```

Serializing a claim does not make it true. Chirograph preserves the evidence references needed to inspect why the claim exists.

## Versioning

Readers fail closed on unknown schema identifiers. A future incompatible wire shape receives a new schema identifier instead of silently changing `chirograph-evidence-v1`.
