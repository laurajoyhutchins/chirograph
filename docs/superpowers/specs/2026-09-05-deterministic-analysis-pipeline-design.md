# Deterministic Analysis Pipeline Design

**Date:** 2026-09-05  
**Status:** Approved in chat; written design awaiting owner review  
**Extends:** `docs/superpowers/specs/2026-09-03-tree-sitter-adapter-architecture-design.md`

## Summary

Chirograph already has a strict benchmark corpus, canonical graph JSON, directional baseline comparison, and a public benchmark boundary through `chirograph analyze`. The reviewed eight-case baseline currently scores zero contract recall because `analyze` intentionally returns an empty `ContractGraph` rather than inventing logical contracts from file existence alone.

This design fills the missing production seam between source acquisition and semantic graph output. It does not weaken the epistemic boundary. Deterministic software may promote source observations into a contract graph only when the promotion is mechanically justified by explicit, provenance-bearing evidence. Ambiguous correspondence remains unresolved and therefore does not become a contract, authority claim, relationship, clause stance, or finding merely because multiple files look similar.

The first implementation slice targets one honest score increase on the Cargo schema-enum-drift case while preserving every existing benchmark invariant and every non-regression guarantee across the full corpus.

## Problem

The current public analysis path has two separate gaps.

First, acquisition is not wired into `analyze`. The existing Tree-sitter architecture deliberately stops at deterministic source-local facts. That is correct: a parser can observe a Rust enum, field, attribute, call, or assertion, but syntax alone cannot decide which observations are representations of the same logical contract.

Second, the current `analyze <source-tree>` boundary lacks explicit source identity and revision. The benchmark runner knows the pinned upstream repository and exact revision from `specimen.yaml`, but the analyzer receives only a filesystem path. Strict contract identity and exact provenance cannot always be reconstructed honestly from fixture bytes or directory names. Reading benchmark parent directories, `specimen.yaml`, or `golden.yaml` from inside the analyzer would turn corpus layout into an answer channel and is forbidden.

The missing layer is therefore not another parser. It is a deterministic, provenance-bound analysis pipeline with an explicit source context.

## Goals

1. Make the public `chirograph analyze` path capable of producing non-empty canonical graphs from ordinary source trees when the evidence is sufficient.
2. Preserve the distinction between an observed representation and a logical contract.
3. Make every automatic alignment explainable by concrete source evidence and exact provenance.
4. Keep unresolved or ambiguous alignment unresolved.
5. Preserve facet-scoped authority instead of introducing global authority ranking.
6. Keep benchmark scoring hermetic, data-only, and independent of golden truth.
7. Derive stable contract identities from explicit source context plus evidenced semantic paths, not benchmark case names.
8. Improve at least one reviewed benchmark metric without regressing any existing case.
9. Keep `chirograph-core` language-agnostic and keep parsing responsibilities in adapters/substrates.

## Non-goals

This design does not add fuzzy matching, embeddings, LLM judgment, confidence thresholds, majority voting, whole-program compiler semantics, global authority ranking, repository-specific recognizers, benchmark-case recognizers, or golden-derived hints.

It does not attempt to solve all eight benchmark cases in the first slice. It does not replace the explicit semantic-alignment model or semantic-query API. It does not make Tree-sitter responsible for contract truth. It does not fetch source from the network during normal analysis or benchmark scoring.

## Governing invariant

> A source observation may enter the semantic graph only to the extent that deterministic evidence justifies the identity, relationship, facet, and authority claim being emitted.

An observation can be perfectly valid while its cross-representation alignment is unknown. In that state Chirograph preserves the observation and diagnostic evidence but does not inflate the contract graph.

## Architecture

```text
explicit source context + source tree
                |
                v
     deterministic discovery
                |
                v
       language/data adapters
                |
                v
   source-local observations/facts
                |
                v
   representation candidates
                |
                v
  evidenced relation + identity rules
                |
        +-------+--------+
        |                |
   confirmed         unresolved
   alignment         alignment
        |                |
        v                +--> diagnostics / inspectable evidence
 semantic graph
        |
        v
 validate + canonical graph JSON
```

### 1. Explicit source context

The analysis kernel receives source context separately from source bytes. At minimum the context carries:

- repository identity, such as `github:rust-lang/cargo`;
- revision as an exact commit when known, otherwise the existing explicit unversioned/unknown state;
- the local source-root path used only to locate bytes.

Repository identity is provenance, not semantic truth. The default contract namespace is derived mechanically from the repository name component, for example `github:rust-lang/cargo` -> `cargo`. The benchmark runner may pass the upstream repository and exact revision already declared in `specimen.yaml`; it must not pass scenario, case, golden contract IDs, expected findings, authority answers, or any other reviewed truth.

The public CLI remains the only benchmark execution boundary, but it gains generic provenance arguments. Conceptually:

```text
chirograph analyze <fixture-dir> \
  --source-repository rust-lang/cargo \
  --revision 2ceefa0090080354b80cc2f5415039bdb0d2bf0b \
  --format graph-json
```

The exact flag spelling is an implementation detail. The contract is that source identity and revision are explicit inputs to the same public analyzer used outside the benchmark.

The analyzer must not walk above the supplied source root to discover benchmark metadata or infer identity from `benchmark/<repository>/<scenario>/<case>` directory names.

### 2. Deterministic source discovery

Discovery enumerates supported regular files beneath the supplied root in stable repository-relative path order. File classification is based on reusable language/data-format rules, not repository names or benchmark scenarios.

Unsupported files are omitted with deterministic diagnostics where useful. Discovery performs no network access and does not mutate the analyzed tree.

### 3. Acquisition adapters remain source-local

The 2026-09-03 Tree-sitter architecture remains authoritative for parser responsibilities. Tree-sitter adapters receive caller-supplied bytes and provenance, preserve exact spans, and emit language-aware source-local facts without assigning contract truth.

Structured data formats use semantic parsers where appropriate. A JSON schema parser, for example, may observe object/property paths, enum values, references, and schema metadata. A Rust adapter may observe declarations, fields, type references, serde/schemars attributes, literals, calls, and assertions. Neither may declare two artifacts to be the same contract simply because their names resemble each other.

Adapters may expose explicit mechanism evidence that is visible in source, such as serialization names, generation annotations, schema references, import/type references, or a declared generated-from relationship. Such evidence is still an observation. Promotion into a semantic relationship happens later.

### 4. Representation candidates

A representation candidate is the analysis-layer form of an observed source artifact before contract membership is known. It carries only mechanically observed data:

- source/revision and exact locator;
- representation kind and supported facets;
- qualified local identity;
- zero or more evidenced semantic paths;
- zero or more evidenced generation, projection, implementation, validation, or dependency links;
- the observations that justify each field.

A semantic path is stronger than a tokenized name. It is a path the source representation explicitly exposes to its consumer, such as a serialized manifest key path, protocol message name, schema definition path, package-qualified API type, or other externally meaningful identity. Adapters must not fabricate semantic paths from benchmark expectations.

Candidates are deterministic and stably ordered. Candidate count alone has no semantic force.

### 5. Conservative deterministic alignment

Alignment is a separate policy layer. It consumes candidates and emits the existing confirmed/rejected/unresolved alignment states with provenance.

Automatic confirmation requires a mechanical identity bridge. Examples include:

- an explicit serialized field path on an implementation matching an explicit property path in a schema;
- an explicit generator/projection declaration connecting a source declaration to a generated artifact;
- an explicit type/reference edge connecting a wrapper to a generated representation;
- the same externally declared qualified identity expressed by two supported representations when that identity is unambiguous in the supplied source context.

Name equality, token overlap, directory proximity, file count, or repeated agreement are insufficient by themselves. If two possible targets satisfy the same weak evidence, alignment remains unresolved rather than choosing one by rank.

Rejected alignment is used only when evidence establishes that two candidates are not representations of the same logical contract. Absence of evidence is unresolved, not rejected.

### 6. Contract seeds and stable identity

A logical contract is created only after at least one representation has an evidenced semantic path strong enough to serve as a contract seed.

The stable contract identifier is derived from:

```text
<source namespace> + <canonical evidenced semantic path>
```

The canonical path comes from the strongest explicit consumer-facing identity available for that contract, not from the benchmark case ID and not from whichever representation has the most files. Other confirmed representations attach to that seed through alignment evidence.

For the first Cargo slice, the relevant identity is the Cargo manifest semantic path for profile debug information. The source namespace is mechanically derived from `rust-lang/cargo`; the manifest path must be obtained from serialization/schema evidence. The implementation may not contain a literal check for the Cargo benchmark case, its fixture path, or its golden contract ID.

If a source set contains sufficient representations to establish correspondence but no defensible stable contract path, Chirograph preserves the candidates/alignment evidence without emitting a guessed contract ID.

### 7. Relationship assembly

Relationships enter the graph only when the underlying relation is directly evidenced. The assembler maps supported evidence classes onto existing graph relationship kinds such as `projects`, `generates`, `implements`, `validates`, or `depends-on`.

A confirmed common contract does not imply every possible pairwise relationship. For example, two representations can be aligned to the same contract while the direction or mechanism between them remains unknown.

### 8. Facet-scoped authority

Authority remains a property of a representation for a specific contract facet. The assembler may emit an authority claim only when its basis is source-backed:

- explicit declaration;
- mechanical enforcement;
- observed behavior;
- documentation;
- inference only where the existing model explicitly permits an inference claim and the evidence remains visible.

There is no global winner. Structural authority does not silently become executable or semantic authority.

Generation direction can support authority reasoning without becoming an authority shortcut. A generated projection is not authoritative merely because it is generated, and a source declaration is not authoritative merely because it is handwritten.

### 9. Clause findings

The initial analysis pipeline does not invent prose clauses from arbitrary code. A contested finding is emitted only when a supported deterministic rule can construct the clause identity and compare representations over the same aligned semantic subject.

For the Cargo first slice, enum-spelling comparison is eligible because both sides expose a mechanically aligned serialized value set for one semantic field. The clause/finding logic must operate on generic aligned enum/value-set observations and must be reusable outside Cargo.

If the analyzer can establish the Cargo contract and representations but not yet justify the contested clause, contract/authority/relationship score may improve while finding recall remains unchanged. Partial honest improvement is preferable to manufacturing a finding.

### 10. Graph assembly and validation

Only confirmed contract seeds, confirmed representation membership, evidenced relationships, supported authority claims, and mechanically justified findings are projected into `ContractGraph`.

Unresolved candidates do not become placeholder contracts in graph JSON. Their existence may be exposed through analysis diagnostics or a future observation/alignment output surface, but the benchmark graph remains a statement of what Chirograph can justify.

The completed graph is validated before canonical `chirograph-graph-v1` encoding. Ordering must be independent of filesystem enumeration order, adapter iteration order, hash-map order, and equivalent input permutation.

## False-contract restraint

Precision is a first-class safety property, not a secondary optimization. The analyzer must prefer zero output over a guessed contract.

Required negative behavior includes:

- neighboring types are not contracts merely because they share a prefix;
- dependencies are not second representations of the enclosing contract;
- generated helpers are not automatically independent contracts;
- multiple files agreeing on a name do not create authority by majority;
- an unresolved semantic identity never becomes confirmed because more weak matches appear.

The Kubernetes benchmark's PodSpec/PodStatus/PodList negatives remain a useful later acceptance test for this property, but Kubernetes-specific names must never appear in production recognition code.

## Benchmark boundary

The benchmark remains an evaluator of production behavior, not a participant in analysis.

Allowed benchmark-to-analyzer inputs:

- fixture bytes;
- upstream repository identity from `specimen.yaml`;
- exact upstream revision from `specimen.yaml`;
- ordinary public CLI options.

Forbidden analyzer inputs or observations:

- `golden.yaml` or any values derived from it;
- benchmark scenario or case ID as semantic hints;
- expected contract IDs, authority claims, relationships, clauses, findings, lifecycle, or non-contracts;
- parent-directory inspection of the benchmark corpus;
- benchmark-only adapter dispatch.

A required anti-leakage regression copies the same fixture bytes to an unrelated temporary path and invokes the public analyzer with the same explicit source context. Canonical graph output must be byte-identical. This proves that corpus directory names are not functioning as an answer key.

The runner continues to launch only the public Chirograph CLI. It may enrich that invocation with generic source provenance from `specimen.yaml`; it must not privately call adapters or semantic assembly internals.

## Error and diagnostic behavior

Analysis fails explicitly when source context required for an exact operation is malformed. Parse uncertainty remains a diagnostic rather than being upgraded into semantic certainty.

Adapter failure for one supported file must be represented deterministically. The first implementation slice should fail the analysis when a required adapter cannot parse bytes safely enough to support emitted semantics; silently dropping contradictory evidence would produce a stronger and therefore misleading graph.

Unresolved alignment is not an execution error. It is a valid epistemic result and may yield an empty or partial semantic graph.

## Determinism and provenance invariants

For identical bytes, source context, Chirograph revision, and analyzer configuration:

1. discovered source order is stable;
2. adapter observations are stable;
3. candidate identities and evidence references are stable;
4. alignment decisions are stable;
5. graph entities and relationships are stable;
6. encoded graph JSON is byte-stable;
7. every emitted semantic claim is traceable to exact source provenance;
8. changing the exact revision changes provenance even when bytes happen to be identical.

No clock, network response, environment-dependent repository discovery, or model call may affect the scored path.

## First implementation slice: Cargo

The first slice proves the architecture with the existing Cargo case rather than attempting broad corpus coverage.

It should compose:

- the shared Tree-sitter substrate already designed;
- the generic Rust source-fact adapter already designed;
- semantic JSON/schema observations;
- explicit source context at the public analysis boundary;
- generic representation-candidate construction;
- generic serialized-path/generation alignment sufficient to connect implementation and schema when the evidence supports it;
- conservative graph assembly.

The slice must not introduce Cargo-specific production symbols, branch on `repository == cargo`, inspect benchmark paths, or alter golden truth to fit analyzer output.

A useful implementation may initially improve only contract recall. Further authority, relationship, and finding recall should be added only when each claim has an independently defensible evidence path.

## Testing strategy

### Synthetic invariant tests

Use tiny invented fixtures to prove:

- exact source context survives acquisition and assembly;
- explicit projection/generation evidence can confirm alignment;
- same-name unrelated representations remain unresolved;
- ambiguous one-to-many correspondence remains unresolved;
- unresolved repetition does not promote to confirmed;
- facet-scoped authority does not spill across facets;
- deterministic ordering survives input permutation;
- malformed provenance or unsupported semantic claims fail closed.

### Public CLI tests

Process-level tests exercise the built `chirograph analyze` binary with explicit source context and verify canonical graph JSON, invalid context failures, and path-independent output.

### Cargo acceptance test

The existing reviewed Cargo fixture is analyzed through the public CLI and scored by the existing benchmark scorer with its existing `golden.yaml` unchanged.

The first successful slice must make `cargo/schema-enum-drift/toml-debug-info-spellings` contract recall greater than zero without increasing false-contract rate or moving contract inflation farther from 1.0.

### Full-corpus non-regression

Run the full reviewed baseline after every score-improving slice. Existing directional baseline semantics remain authoritative: higher-is-better metrics may not decrease, false-contract rate may not increase, inflation may not move farther from 1.0, and scored cases may not become execution failures.

Do not rewrite `benchmark/baseline.json` merely because a metric improves. The old baseline is the regression floor and can validate an improvement directly.

## Acceptance criteria

This design is ready for implementation planning when all of the following are agreed:

1. source identity and revision are explicit analyzer inputs rather than inferred from benchmark layout;
2. acquisition remains source-local and does not assign contract truth;
3. representation candidates preserve observations before semantic commitment;
4. automatic alignment requires mechanical, provenance-bearing evidence;
5. ambiguous alignment stays unresolved;
6. contract IDs derive from source namespace plus an evidenced semantic path;
7. authority remains facet-scoped and evidence-backed;
8. the scored path is deterministic, offline, and model-free;
9. the analyzer cannot read golden truth or benchmark case metadata;
10. the first implementation slice targets Cargo with unchanged golden truth;
11. full-corpus baseline comparison must pass with no regressions;
12. at least one reviewed higher-is-better benchmark metric must improve on the exact final revision.

## Design boundary with future agent judgment

This deterministic path is intentionally conservative. A reasoning agent may later inspect unresolved observations and propose explicit alignment claims through a separate provenance-bearing interface. Such judgment must remain distinguishable from deterministic analysis and must not be smuggled into the hermetic benchmark path.

That separation preserves Chirograph's central value: deterministic software records what can be established mechanically, while reasoning remains available for the genuinely semantic decisions that software cannot justify on its own.