# Chirograph model

Chirograph models logical software contracts without turning interpretations into source facts.

The core question is not merely whether two files contain similar text. It is:

> What logical contract is represented here, what does each representation claim about that contract, what evidence supports those claims, where do the representations disagree, and which representation appears to govern each facet?

## Core graph

```text
Source ──contains──> Representation ──represents──> Contract ──contains──> Clause
   │                       │                                             ▲
   │                       └────── ClauseAssertion ──────────────────────┘
   │                                supports | contradicts
   │
   └──yields──> Observation ──evidence for──> Relation
                         ├──evidence for──> AuthorityClaim
                         └──evidence for──> ClauseAssertion
```

The durable concepts are:

1. **Contract** — one logical software contract, independent of any single syntax or file.
2. **Representation** — a concrete manifestation of a contract: executable surface, source code, schema, type, validator, test, documentation, configuration, or generated artifact.
3. **Source** — the physical or observable origin that contains a representation or yields observations.
4. **Observation** — a source-backed fact observed at an explicit revision state. Observations are evidence, not interpretation.
5. **Clause** — one atomic proposition about one facet of one logical contract.
6. **Clause assertion** — an evidence-backed interpretation that one representation supports or contradicts a clause.
7. **Relation / authority claim** — other evidence-backed interpretations connecting graph entities or nominating facet authority.

## Contract facets

A logical contract can span seven facets:

- **Structural** — shape, fields, types, constraints, and representation-level schema meaning.
- **Executable** — invocation surface, accepted inputs, outputs, environment, and observable process/API behavior.
- **Semantic** — what state or meaning the operation establishes.
- **Failure** — failure classes and what can safely be concluded after each failure.
- **Concurrency** — behavior under races, locking, compare-and-swap, fencing, and concurrent mutation.
- **Recovery** — retry, idempotency, reconciliation, compensation, and interrupted-work behavior.
- **Verification** — how a caller can establish the authoritative outcome instead of trusting an invocation result alone.

A representation may cover only a subset of the facets of its logical contract.

## Contract clauses

A clause is the smallest contract proposition Chirograph currently persists. It belongs to exactly one logical contract and one contract facet.

The v0.1 logical roles are deliberately small:

- `Requirement` — a condition that must hold for the contract behavior in question to apply, including accepted-input and precondition semantics.
- `Guarantee` — behavior or an outcome the contract claims under its applicable requirements, including failure, recovery, and verification guarantees.
- `Invariant` — a proposition claimed to remain true across the relevant states or transitions.

The facet and clause kind are orthogonal. For example, a retry-safety rule can be a `Guarantee` in the `Recovery` facet, while an expected-old-value rule can be a `Requirement` in the `Concurrency` facet.

Chirograph does not yet encode clauses as a formal predicate language. `statement` is a stable atomic proposition for comparison and explanation, not an executable proof term.

### Representation stance

A representation can make one of two explicit evidence-backed assertions about a clause:

- `Supports` — observations from that representation are consistent with the clause.
- `Contradicts` — observations from that representation conflict with the clause.

Silence is neither. If Chirograph has no clause assertion from a representation, it must not manufacture agreement, disagreement, or an `Unknown` assertion.

Every persisted clause requires at least one supporting representation. Otherwise there is no evidence-backed reason for that proposition to exist in the graph.

Contradiction is valid graph evidence. It does not make the graph invalid. This distinction is what lets Chirograph preserve drift instead of deleting whichever side loses an authority heuristic.

### Clause assessment

Clause status is derived from the current evidence:

```text
one or more Supports + no Contradicts  -> Consistent
one or more Supports + any Contradict -> Contested
```

`Consistent` does **not** mean proven true, universally implemented, or fully verified. It means only that the assertions currently represented in the graph contain no contradiction for that clause.

`Contested` does **not** choose a winner. Chirograph reports the supporting and contradicting representations deterministically and leaves authority policy, human judgment, or stronger mechanical evidence to resolve the disagreement.

There is deliberately no truth-by-majority rule. Ten documentation copies do not mechanically defeat one executable behavior observation.

## Relations

Relations are typed edges and always remain distinct from observations. Initial relation kinds are:

- `Defines`
- `Implements`
- `Documents`
- `Validates`
- `Generates`
- `Projects`
- `EquivalentTo`
- `ConflictsWith`
- `DependsOn`

Every relation carries observation IDs as its evidence basis. A relation without a source-backed basis is not a valid persisted graph fact.

## Authority

Authority is not inferred from filenames or prose alone and is never stored as an unqualified fact. Chirograph records a **facet-scoped authority claim** connecting one representation to one facet of the logical contract it appears to govern. This allows different representations to govern different parts of the same contract without forcing a single global authority.

The basis is explicit:

- `ExplicitDeclaration`
- `MechanicalEnforcement`
- `ObservedBehavior`
- `Documentation`
- `Inference`

The basis describes why the claim exists. It is not a universal ranking. Different domains can apply different authority policies later without changing the evidence model.

An authority claim must:

- point to an existing contract;
- point to a representation of that same contract;
- name a facet that representation actually covers; and
- cite existing source-backed observations.

Authority and clause assessment are separate questions. A clause may be contested even when one representation has a stronger authority claim. Chirograph should preserve the disagreement rather than rewrite the evidence graph to match the authority decision.

## Revision truth

Every observation records revision status explicitly:

- `Exact(value)` — the source was observed at an exact version/revision/digest coordinate;
- `Unversioned` — the source is known not to expose a revision coordinate;
- `Unknown` — revision identity could not be established.

`Unknown` is data. Chirograph must not manufacture precision to make a graph look complete.

## Invariants

The v0.1 model enforces these mechanical invariants:

1. Source, contract, representation, observation, and clause IDs are unique.
2. Representations reference existing sources and contracts.
3. A representation cannot claim a facet its contract does not declare.
4. Observations reference existing sources.
5. Every clause references an existing contract and a facet that contract declares.
6. Clause statements are non-empty and are never silently whitespace-normalized.
7. Clause assertions reference existing clauses and representations from the same logical contract.
8. A clause assertion can only come from a representation that covers the clause facet.
9. Every clause assertion cites existing observations.
10. Every persisted clause has at least one supporting assertion.
11. Contradicting assertions are preserved as valid evidence and derive a contested assessment.
12. Relation endpoints exist and every relation cites existing observations.
13. Every authority claim cites existing observations.
14. An authority claim cannot nominate a representation belonging to another logical contract.
15. An authority claim is scoped to a facet the nominated representation actually covers.
16. Identifiers are never silently normalized.

The model intentionally does **not** yet decide extraction APIs, serialization format, authority-ranking policy, clause predicate syntax, similarity heuristics, or repository-specific adapters. Those belong above this kernel.

## Epistemic boundary

Chirograph keeps source facts and derived interpretations separate:

```text
source observation                         derived interpretation
------------------                         ----------------------
"help says --old must match"       ───>   clause: old value must match current value
                                       └─> help representation Supports clause
implementation accepts a mismatch   ───>   implementation representation Contradicts clause
validator rejects bad schema        ───>   validator mechanically enforces a contract facet
file says "generated from X"        ───>   X is an authority candidate
```

A clause, assertion, relation, or authority claim may be wrong. Preserving the exact observations that justify each interpretation makes the graph inspectable, correctable, and explainable.
