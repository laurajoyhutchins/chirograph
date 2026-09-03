# Python acquisition adapter

`chirograph-python-adapter` is Chirograph's generic, read-only Python source acquisition layer. It uses `tree-sitter-python` from Rust and does not import or execute the Python source it observes.

The adapter deliberately stops at source-backed facts. Framework meaning belongs in later interpretation or in independent executable observations. A decorator named `field_validator`, for example, is observed as Python syntax with its exact arguments; the parser does not infer Pydantic semantics from the name.

## v0 facts

The adapter currently observes:

- modules;
- class and function definitions, including function return annotations;
- annotated assignments and their type expressions;
- decorators with their source arguments;
- calls;
- `return` and `raise` statements;
- `if` / `elif` conditions;
- comments;
- module, class, and function docstrings when they are mechanically identifiable as the first statement in that scope; and
- `assert` statements as test assertions.

Every fact records the source path, zero-based half-open byte range, one-based line/column range, and original source text. `observe_python_source` projects those facts into ordinary `chirograph_core::model::Observation` values while preserving the caller-supplied revision exactly.

Syntax errors fail closed. Chirograph does not emit a partial set of Python source facts from a Tree-sitter tree containing error nodes.

## Boundary

This adapter is not a Python compiler, type checker, import resolver, runtime, or framework adapter. It does not establish that an annotation is enforced, that a decorator changes behavior, or that a call succeeds. Those claims require other evidence sources such as runtime probes, generated schemas, tests, or external analyzers.

That separation is intentional: Tree-sitter answers what Python source contains. It does not quietly promote syntax into contract truth.
