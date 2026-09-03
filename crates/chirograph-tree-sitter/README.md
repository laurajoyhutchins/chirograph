# chirograph-tree-sitter

`chirograph-tree-sitter` is Chirograph's shared, language-neutral acquisition substrate for Tree-sitter grammars.

The caller supplies source bytes, a Tree-sitter language, and `SourceProvenance`. The crate parses those bytes read-only and preserves the supplied source identity, revision, locator, path, byte offsets, and row/column coordinates. It never infers an exact revision that the caller did not provide.

This crate owns parser lifecycle, deterministic child-index preorder traversal, source text/span helpers, and syntax diagnostics. Tree-sitter `ERROR` and missing nodes are retained as diagnostics rather than being silently treated as a clean parse. Invalid UTF-8 is rejected before parser invocation.

The crate deliberately does **not** perform filesystem discovery, Git operations, repository retrieval, or network access. It also does not assign contract truth, authority, `supports`, or `contradicts` stance, and it contains no language-specific or project-specific semantics. Language adapters build source-local facts on top of this substrate; higher layers decide how those facts relate to contract clauses.
