# Guarantees and limits

Chirograph is an evidence-backed contract analysis tool. Its output should be read as a reproducible assessment of represented evidence, not as an oracle about all possible program behavior.

The canonical concepts and graph invariants live in [`model.md`](model.md). This document states the user-facing boundary around those mechanics.

## What Chirograph can establish

For a valid `chirograph-evidence-v1` document, Chirograph can mechanically establish which sources, representations, observations, clauses, assertions, relations, and authority claims are represented; whether the graph satisfies the model invariants; and whether each represented clause is currently `CONSISTENT` or `CONTESTED` under the evidence in that graph.

A `CONTESTED` clause has at least one supporting assertion and at least one contradicting assertion. Chirograph preserves both sides. It does not resolve disagreement by counting files or assertions.

For an explicit semantic alignment catalog, Chirograph can validate that observed representations, candidate contracts, facets, and cited observations exist; that every alignment claim has evidence; and that each representation/contract/facet identity has at most one recorded state. The states `Confirmed`, `Rejected`, and `Unresolved` are preserved exactly as supplied.

An exact revision value means the observation claims an explicit source revision coordinate. `Unversioned` and `Unknown` remain distinct and are not upgraded to false precision.

For the same Chirograph version and the same valid evidence input, the current `inspect` renderer orders reported contracts, clauses, and authority claims deterministically. Alignment query helpers likewise return stable ordering without using input order as a semantic signal.

## What Chirograph does not establish

`CONSISTENT` does not mean "proved true." It means that the evidence represented in the graph contains support and no represented contradiction for that clause.

An `Unresolved` alignment does not become confirmed because other representations align with the same contract, because similar text appears repeatedly, or because one interpretation has more supporting files. A `Rejected` alignment is not overridden by peer consensus. Chirograph's core does not perform fuzzy matching, confidence scoring, or truth-by-majority to fill these gaps.

No observation from a representation means no observation. Chirograph must not convert silence, unsupported syntax, parser failure, missing files, or an adapter's limited coverage into evidence that a contract is absent or satisfied.

An authority claim is evidence-backed metadata about apparent authority for one facet. It does not erase contradictory evidence and is not a universal ranking policy.

Static source evidence does not by itself prove runtime behavior. Runtime observations do not by themselves prove every possible execution. Documentation does not automatically outrank implementation, and executable behavior does not automatically answer a specification question. Those relationships require explicit evidence and policy.

Chirograph does not promise complete discovery of every contract in a repository. Coverage is bounded by the acquisition adapters and evidence supplied.

## Failure should remain legible

When Chirograph cannot establish a revision, parse an evidence format, validate a graph, establish semantic alignment, or support a syntax construct, the preferred result is an explicit error, diagnostic, `Unknown`, `Unresolved`, or absence of an assertion. Invented agreement is not a recovery strategy.

## Version boundary

The evidence document starts with an explicit schema identifier such as `chirograph-evidence-v1`. Compatibility is tied to that identifier, not to a guess based on field shape. The current v1 representation records already-aligned contract membership; the new pre-alignment model does not silently change that wire contract. See [`evidence-interchange.md`](evidence-interchange.md).
