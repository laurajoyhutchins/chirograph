# Pydantic specimen

This adapter is a read-only Chirograph specimen, not a general Pydantic integration.

It pins Pydantic release `2.13.5` at exact upstream commit `001dea020e0809844e5b17666432c9135a976f46` and observes only public behavior. The adapter does not modify Pydantic and does not add Pydantic-specific concepts to `chirograph-core`.

The specimen exercises two cases:

1. A `str` field with a `before` validator whose declared validation input is `int | str`. The validation JSON Schema and runtime acceptance cover integers and strings, while the validated state and serialization perspective are strings. Chirograph records these as different clauses/perspectives of one logical contract rather than false drift.
2. A computed `area` field that is absent from validation input schema but present in serialization schema and runtime output. Again, the difference is intentional rather than contested.

`fixtures/stale-validation-schema.json` supplies a deliberately stale same-perspective schema that claims the validator accepts only strings. Adding that evidence causes the existing validation-input clause to become `CONTESTED` without changing Chirograph production code.

Run the specimen after installing the pinned Pydantic revision:

```sh
python -m pip install 'pydantic @ git+https://github.com/pydantic/pydantic.git@001dea020e0809844e5b17666432c9135a976f46'
python -m unittest adapters/pydantic/test_adapter.py -v
```

The adapter itself emits ordinary Chirograph evidence:

```sh
python adapters/pydantic/extract.py \
  --upstream-revision 001dea020e0809844e5b17666432c9135a976f46 \
  > /tmp/pydantic-evidence.json
cargo run --quiet -p chirograph-cli -- inspect /tmp/pydantic-evidence.json
```
