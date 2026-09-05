# Adapter contract

Adapters acquire source-backed facts and translate them into Chirograph's language-neutral evidence model. They are how Chirograph learns new syntaxes and acquisition surfaces without making the core know about individual repositories.

## Required properties

A production adapter should satisfy these constraints:

1. **General-purpose scope.** A language adapter may know Rust, Java, Python, OpenAPI, protobuf, or another representation. It must not contain branches whose purpose is to recognize Cargo, Kafka, Pydantic, or another benchmark specimen.
2. **Deterministic acquisition.** The same source bytes, configuration, revision coordinates, and adapter version should produce semantically identical facts and evidence.
3. **Exact provenance.** Preserve source identity, source locations, and revision status. Do not replace an unknown revision with a branch name or inferred version.
4. **Bounded interpretation.** Language-aware parsing is appropriate. Contract truth, authority, and cross-representation agreement should remain explicit derived layers with evidence references.
5. **Partial coverage is explicit.** Unsupported syntax or semantic analysis is a limitation, not evidence of absence.
6. **Ordinary evidence.** Adapters should emit or support the public versioned Chirograph evidence model instead of extending `chirograph-core` with specimen-specific concepts.
7. **Declared capabilities.** Document whether the adapter reads files, follows symlinks, discovers repositories, makes network requests, launches runtimes, installs dependencies, executes project code, or calls remote services.
8. **Tests at the adapter boundary.** Test generic language behavior first. Use real repositories as acceptance specimens, not as the implementation specification.

## Recommended layering

```text
caller-supplied source / observed behavior
                |
                v
       acquisition adapter
                |
                v
     provenance-rich facts
                |
                v
       evidence construction
                |
                v
     chirograph-evidence-v1
                |
                v
       contract analysis
```

A shared parser substrate may provide byte spans, deterministic traversal, and diagnostics. Language adapters may map syntax nodes into useful source-local facts. Higher layers may rank evidence candidates or construct clause assertions. Keeping these layers visible prevents a benchmark-specific heuristic from quietly becoming a "parser feature."

## Production acquisition runtime

`chirograph-acquisition` is the common production boundary used by `chirograph analyze` before semantic graph assembly. It owns deterministic repository-relative discovery, adapter registration and selection, caller-supplied source/revision context, provenance/span transport, bounded diagnostics, and stable fact ordering. It deliberately does not own logical contract identity, cross-representation alignment, authority, or clause truth.

Adapters implement the public `SourceAdapter` contract by declaring a stable capability record and extracting source-local `AcquiredFact` values from caller-supplied bytes. Tree-sitter-backed languages and semantic structured-data parsers share this runtime envelope without sharing a fake universal syntax vocabulary. The built-in runtime currently proves both paths with Rust and semantic JSON/JSON-with-comments acquisition; additional language and schema adapters should plug into the same registry instead of copying file discovery, parser lifecycle, or provenance machinery.

Adapter identity must be unique. If more than one adapter claims a discovered file, selection fails closed rather than choosing by registration order. Unsupported regular files produce bounded diagnostics instead of evidence of absence. Malformed supported input is an acquisition failure, not partial semantic evidence. Discovery does not follow symlinks and skips common generated/dependency directories.

The acquisition runtime performs no network access, repository checkout, analyzed-project execution, dependency installation, fuzzy matching, benchmark/case dispatch, golden access, or authority inference. Those capabilities, when needed, belong behind separate explicit trust and reasoning boundaries.

## Source retrieval

Source retrieval and source parsing are separate capabilities. An adapter should be usable with caller-supplied bytes when practical. Live remote retrieval can be useful in demos and tooling, but a benchmark should not score retrieval unless retrieval itself is the phenomenon being measured.

## Security

Follow [`../SECURITY.md`](../SECURITY.md). Parsing source should not silently execute it. If execution is necessary for a class of evidence, expose that as a distinct capability and make the trust boundary obvious to callers.
