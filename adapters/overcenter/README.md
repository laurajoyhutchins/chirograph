# Overcenter adapter

This adapter converts Overcenter `contract-evidence-catalog-v1` plus `contract-evidence-classifications-v1` documents into the neutral `chirograph-evidence-v1` interchange.

```sh
node adapters/overcenter/convert.mjs catalog.json classifications.json > evidence.json
cargo run --quiet -p chirograph-cli -- inspect evidence.json
```

The adapter treats Overcenter's logical contract authority as **structural** authority. Candidate source identities become Chirograph representations. Non-projection classifications become facet-scoped authority claims backed by observations of the classification metadata. Projection classifications become `Projects` relations.

Overcenter relationship kinds map into the closest generic Chirograph relation:

| Overcenter | Chirograph |
| --- | --- |
| `consumes` | `DependsOn` |
| `produces` | `Defines` |
| `persists-as` | `Projects` |
| `derives-from` | `DependsOn` |
| `verified-by` | `Validates` |
| `compatibility-for` | `DependsOn` |

The adapter intentionally does not copy Overcenter significance classes, lifecycle labels, or SemVer policy into the Chirograph kernel. It also does not invent contract clauses when the source catalog contains no clause-level evidence.
