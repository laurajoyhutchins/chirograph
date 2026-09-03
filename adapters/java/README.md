# Java adapter

The Java adapter is Chirograph's read-only syntax acquisition layer for Java source. It uses [`tree-sitter-java`](https://github.com/tree-sitter/tree-sitter-java) rather than implementing a Java parser.

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

## What it does not do

This is not a Java compiler or whole-program analyzer. The v0 adapter does not resolve types across files, construct a project classpath, follow dynamic dispatch, infer data flow, or decide that two observations express the same logical contract clause. Those are separate semantic-resolution problems and should not be hidden inside a parser adapter.

Parsing fails closed when Tree-sitter reports syntax errors.

## Kafka specimen

`examples/kafka_idempotence.rs` is an external specimen, not Kafka logic in the adapter. CI downloads Apache Kafka's `ProducerConfig.java` at exact upstream revision `5e3bc31a7dbc354155932e38ab35d11dd71b97bf`, acquires the relevant documentation and validator observations through Tree-sitter, emits `chirograph-evidence-v1`, and runs the ordinary Chirograph inspector.

The specimen must reproduce the previously hand-assembled result that the implicit `max.in.flight.requests.per.connection > 5` fallback clause is `CONTESTED`: the Java documentation supports fallback while the mechanically enforced validator throws `ConfigException`.
