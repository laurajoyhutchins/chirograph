# Semantic query API

`chirograph-core::query::SemanticQuery` is a deterministic, read-only view over validated Chirograph state. It exists to let agents, CLIs, and later integrations inspect the semantic graph without reimplementing graph traversal or silently adding interpretation.

The query layer does not mutate graph state. Construction validates the supplied `ContractGraph`; `with_alignments` additionally validates the supplied `AlignmentCatalog` against that graph.

## Operations

The initial API exposes:

- `contracts()` — all represented logical contracts, ordered by `ContractId`.
- `representations_for(contract)` — post-alignment representations explicitly assigned to one contract, ordered by `RepresentationId`.
- `clauses_for(contract)` — clauses explicitly belonging to one contract, ordered by `ClauseId`.
- `evidence_for(contract)` — source observations mechanically reachable through explicit provenance-bearing interpretations associated with the contract.
- `contestations()` — every clause whose represented assertions include at least one supporter and at least one contradictor.
- `authority_for(contract, facet)` — every recorded authority claim for that contract facet, returned deterministically without selecting a winner.
- `alignments_for(observed_representation)` — alignment claims recorded for one pre-alignment representation when the query was constructed with an alignment catalog.

Queries naming an unknown contract fail with `QueryError::UnknownContract` rather than silently returning an empty result.

## Evidence closure

`evidence_for(contract)` intentionally does not mean "all observations near files that appear related to this contract." It returns only observations cited by explicit semantic edges already represented in Chirograph:

1. clause assertions for clauses belonging to the contract;
2. relations whose endpoint is the contract or one of its post-alignment representations;
3. authority claims for the contract; and
4. alignment claims for the contract when an alignment catalog was supplied.

The resulting observation IDs are deduplicated and ordered deterministically. An observation that merely comes from the same source, file, or repository is excluded unless one of those explicit interpretations cites it.

This boundary is deliberate. Query traversal must not become a second, hidden semantic-alignment engine.

## Determinism

For the same validated semantic state, query results do not depend on vector insertion order in the input graph. Results use stable semantic identities for ordering. Deterministic ordering is presentation and reproducibility machinery, not an authority ranking.

In particular, `authority_for` may return several claims. Their order does not indicate strength, precedence, confidence, or truth.

## Contestation

`contestations()` delegates to Chirograph's existing clause assessment semantics. It preserves both supporting and contradicting representation IDs. More supporters do not erase a contradictor, and the query layer does not choose which side should govern.

## Alignment

`alignments_for` exposes only claims already present in the supplied `AlignmentCatalog`. `Confirmed`, `Rejected`, and `Unresolved` are returned as recorded. The query layer performs no fuzzy matching, confidence scoring, peer voting, or promotion of unresolved state.

A `SemanticQuery` constructed without an alignment catalog has no pre-alignment claims to expose. An empty alignment result is therefore not evidence that an artifact cannot align to a contract; it means only that this query view has no recorded claim for that observed representation.

## CLI

The command-line interface exposes the same read-only semantics without adding a second interpretation layer:

```sh
chirograph contestations evidence.json
chirograph evidence evidence.json review-status
chirograph authority evidence.json review-status semantic
chirograph alignment evidence.json alignments.json candidate-example
```

`contestations` preserves both sides of every disagreement. `evidence` prints only the explicit provenance closure for the named contract. `authority` prints every recorded claim for the exact contract facet without ranking them. `alignment` additionally reads a separate `chirograph-alignments-v1` document and reports the recorded pre-alignment states exactly as supplied.

The CLI validates identifiers and documents before querying. Output ordering follows the deterministic query layer, so rearranging collections in an otherwise identical input document does not change the semantic output.

## Non-guarantees

The query layer does not:

- discover new contracts;
- decide that two artifacts express the same contract;
- infer authority from filenames, source kinds, or repetition;
- rank authority claims;
- resolve contested clauses;
- expand evidence by textual similarity or source proximity;
- implement benchmark scoring or benchmark-specific matching;
- add parser, language, framework, or repository-specific semantics; or
- prove that represented evidence is complete or correct.

Those boundaries preserve the distinction between acquisition, interpretation, and deterministic inspection. Higher layers may propose new evidence-backed interpretations, but the query layer only reports semantic state that Chirograph already has grounds to represent.
