# Alignment interchange

`chirograph-alignments-v1` is the versioned interchange for pre-alignment representation claims. It is deliberately separate from `chirograph-evidence-v1`, which remains the post-alignment contract-graph format.

## Document shape

```json
{
  "schema": "chirograph-alignments-v1",
  "representations": [
    {
      "id": "candidate-example",
      "source": "docs",
      "kind": "documentation",
      "locator": "Candidate example"
    }
  ],
  "claims": [
    {
      "representation": "candidate-example",
      "contract": "review-status",
      "facet": "semantic",
      "state": "unresolved",
      "evidence": ["obs-docs"]
    }
  ]
}
```

The v1 parser rejects unsupported schema identifiers, unknown fields, malformed identifiers, invalid representation kinds, facets, or states, and catalogs that fail validation against the supplied contract graph.

## Validation boundary

An alignment document is not meaningful by itself. Chirograph validates it against the exact `ContractGraph` supplied by the caller. Every observed representation must name a known source. Every claim must name a represented pre-alignment artifact, a known contract and contract facet, and at least one known observation as evidence. Duplicate representation identities and duplicate representation/contract/facet claims are rejected.

The three states are exact:

- `confirmed`
- `rejected`
- `unresolved`

Parsing does not promote, rank, merge, or vote on those states. Repetition does not convert `unresolved` into `confirmed`, and confirmed peers do not override a rejected claim.

## Relationship to evidence-v1

`chirograph-evidence-v1` continues to carry sources, observations, logical contracts, post-alignment representations, clauses, assertions, relations, and authority claims. Pre-alignment candidates and their claims do not become fields in evidence-v1.

Keeping the documents separate prevents acquisition or query code from silently treating a candidate representation as a member of a logical contract before an evidence-backed alignment decision exists.

## CLI

The CLI combines the two documents only for the alignment query:

```sh
chirograph alignment evidence.json alignments.json candidate-example
```

Both documents are validated before the query runs. The command reports the recorded claims in deterministic order and performs no semantic matching of its own.
