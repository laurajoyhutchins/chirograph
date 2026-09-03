# Java adapter

The Java adapter is Chirograph's read-only acquisition layer for Java source. It uses [`tree-sitter-java`](https://github.com/tree-sitter/tree-sitter-java) rather than implementing a Java parser.

## Boundary

`chirograph-java-adapter` parses one Java source file at a time and emits deterministic, provenance-rich language facts plus ordinary Chirograph `Observation` values at a caller-supplied revision.

The initial fact vocabulary is deliberately small:

- field declarations, including constants and their source text
- method invocations
- conditionals that contain a thrown exception
- line and block comments
- assertion-style method calls such as `assertThrows`

Every fact records the source path, exact 1-based line/column range, relevant syntactic name or condition when available, and the original source text. `observe_java_source` carries those facts into Chirograph observations without assigning contract meaning to them.

This adapter is framework-agnostic. It does not contain Kafka concepts, Pydantic-style framework rules, or project-specific contract names.

## Evidence candidates

`rank_java_evidence` is a deterministic retrieval layer over acquired Java facts. A caller supplies a semantic query and, optionally, the Java fact kinds that are useful for that query. The ranker:

- splits dotted, underscored, and camel-case text into normalized lexical terms
- removes small connective stop words
- matches whole normalized terms rather than substrings
- ranks matching facts primarily by term overlap and secondarily by a stable fact-kind weight
- gives unmatched facts a neutral score instead of manufacturing relevance
- returns the original Java fact and its exact-revision Chirograph observation together

This is candidate generation, not semantic adjudication. A high score means that an observation is mechanically relevant to the query terms. It does **not** mean the observation supports, contradicts, defines, or governs a contract clause. Clause identity, stance, facet meaning, and authority still belong to higher-level Chirograph logic or agent judgment.

That distinction is intentional: software can cheaply remove the search bookkeeping without laundering lexical similarity into contract truth.

## What it does not do

This is not a Java compiler or whole-program analyzer. The v0 adapter does not resolve types across files, construct a project classpath, follow dynamic dispatch, infer data flow, or automatically decide that two observations express the same logical contract clause. Those are separate semantic-resolution problems and should not be hidden inside a parser adapter.

Parsing fails closed when Tree-sitter reports syntax errors.

## Kafka specimen

`examples/kafka_idempotence.rs` is an external specimen, not Kafka logic in the adapter. CI downloads Apache Kafka's `ProducerConfig.java` at exact upstream revision `5e3bc31a7dbc354155932e38ab35d11dd71b97bf`, acquires Java observations through Tree-sitter, and asks the generic ranker for documentation and validator candidates using semantic query terms. The specimen no longer locates those observations by the exact `ENABLE_IDEMPOTENCE_DOC` constant name or Kafka validator-local variable names.

The selected observations are emitted as ordinary `chirograph-evidence-v1` and passed to the normal Chirograph inspector. Higher-level specimen logic still makes the explicit judgment that the documentation supports the fallback clause while the validator contradicts it.

The acceptance result remains `CONTESTED` for the implicit `max.in.flight.requests.per.connection > 5` fallback clause, with the mechanically enforced validator authoritative for the failure facet.

Verified on Chirograph head `39288bc72d5f6eac09731a4eb96c029d270a78a6` by GitHub Actions run `33721735862`.
