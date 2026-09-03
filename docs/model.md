# Chirograph model

Chirograph models logical software contracts without turning interpretations into source facts.

The core question is not merely whether two files contain similar text. It is:

> What logical contract is represented here, how are its representations related, what evidence supports that relationship, and which representation appears to govern each facet of the contract?

## Core graph

```text
Source ──contains──> Representation ──represents──> Contract
   │                       │                           │
   └── Observation ────────┴── evidence for ─────────┤
                                                     │
Representation ── typed relation ──> Representation │
                                                     │
Representation ── authority claim + evidence ───────┘
```

The graph has five durable concepts:

1. **Contract** — one logical software contract, independent of any single syntax or file.
2. **Representation** — a concrete manifestation of a contract: executable surface, source code, schema, type, validator, test, documentation, configuration, or generated artifact.
3. **Source** — the physical or observable origin that contains a representation or yields observations.
4. **Observation** — a source-backed fact observed at an explicit revision state. Observations are evidence, not interpretation.
5. **Relation / authority claim** — an interpretation justified by observations. These may be revised as stronger evidence appears.

## Contract facets

A logical contract can span six facets:

- **Executable** — invocation surface, accepted inputs, outputs, environment, and observable process/API behavior.
- **Semantic** — what state or meaning the operation establishes.
- **Failure** — failure classes and what can safely be concluded after each failure.
- **Concurrency** — behavior under races, locking, compare-and-swap, fencing, and concurrent mutation.
- **Recovery** — retry, idempotency, reconciliation, compensation, and interrupted-work behavior.
- **Verification** — how a caller can establish the authoritative outcome instead of trusting an invocation result alone.

A representation may cover only a subset of the facets of its logical contract.

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

## Revision truth

Every observation records revision status explicitly:

- `Exact(value)` — the source was observed at an exact version/revision/digest coordinate;
- `Unversioned` — the source is known not to expose a revision coordinate;
- `Unknown` — revision identity could not be established.

`Unknown` is data. Chirograph must not manufacture precision to make a graph look complete.

## Invariants

The v0.1 model enforces these mechanical invariants:

1. Source, contract, representation, and observation IDs are unique.
2. Representations reference existing sources and contracts.
3. A representation cannot claim a facet its contract does not declare.
4. Observations reference existing sources.
5. Relation endpoints exist.
6. Every relation evidence reference resolves to an observation.
7. Every authority claim cites existing observations.
8. An authority claim cannot nominate a representation belonging to another logical contract.
9. An authority claim is scoped to a facet the nominated representation actually covers.
10. Identifiers are never silently normalized.

The model intentionally does **not** yet decide extraction APIs, serialization format, authority-ranking policy, similarity heuristics, or repository-specific adapters. Those belong above this kernel.

## Epistemic boundary

Chirograph keeps two categories separate:

```text
source observation                     derived interpretation
------------------                     ----------------------
"help says --old must match"   ───>   "this representation documents CAS semantics"
validator rejects bad schema    ───>   "validator mechanically enforces this contract"
file says "generated from X"    ───>   "X is an authority candidate"
```

A derived interpretation may be wrong. Preserving the observations that justify it makes the graph inspectable, correctable, and explainable.
