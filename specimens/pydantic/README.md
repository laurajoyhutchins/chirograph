# Pydantic specimen

Pydantic is a Chirograph specimen, not a Chirograph adapter.

The specimen pins Pydantic `2.13.5` at exact upstream commit `001dea020e0809844e5b17666432c9135a976f46` and exercises generic Chirograph acquisition boundaries:

- `adapters/python` parses `models.py` with Tree-sitter and emits exact-revision source observations. Pydantic-specific code does not manufacture static Python facts.
- `specimens/python_runtime.py` provides the framework-agnostic executable observation boundary.
- `probe.py` contains only the Pydantic-specific runtime experiment: `model_validate`, validation and serialization JSON Schema perspectives, `model_dump`, and pinned package/runtime identity.
- `evidence.py` assembles those observations into ordinary `chirograph-evidence-v1` contracts and clauses.

The specimen covers two perspective-sensitive cases:

1. `ValidatorModel.value` is declared as `str`, while a `before` validator accepts `int | str`. Validation schema and runtime acceptance cover integers and strings; normalized state and serialization remain strings.
2. `Rectangle.area` is a computed field. It is absent from validation input schema but present in serialization schema and runtime output.

Those intentional perspective differences remain `CONSISTENT`. `fixtures/stale-validation-schema.json` deliberately claims string-only validation input for the same validation perspective; adding it makes the accepted-input clause `CONTESTED`.

Run the specimen from the repository root:

```sh
python -m pip install 'pydantic @ git+https://github.com/pydantic/pydantic.git@001dea020e0809844e5b17666432c9135a976f46'
python -m unittest specimens/pydantic/test_specimen.py -v
```

Emit evidence directly at the exact Chirograph source revision:

```sh
python specimens/pydantic/evidence.py \
  --repository-revision "$(git rev-parse HEAD)" \
  --upstream-revision 001dea020e0809844e5b17666432c9135a976f46 \
  > /tmp/pydantic-evidence.json
cargo run --quiet -p chirograph-cli -- inspect /tmp/pydantic-evidence.json
```
