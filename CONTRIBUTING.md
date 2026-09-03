# Contributing to Chirograph

Chirograph welcomes bug reports, benchmark cases, language adapters, documentation improvements, and implementation changes that make software-contract analysis more general, reproducible, and inspectable.

## Development workflow

1. Work from the current repository revision and keep changes narrowly scoped.
2. Add or update tests for behavioral changes.
3. Run the repository verification suite:

   ```sh
   cargo fmt --all -- --check
   cargo check --workspace --all-targets
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   node --test adapters/overcenter/convert.test.mjs
   python -m unittest adapters/pydantic/test_adapter.py -v
   python tools/check_release_metadata.py
   ```

4. Explain the contract or evidence behavior being changed and cite exact test or benchmark evidence in the pull request.

## Design constraints

Keep `chirograph-core` language-agnostic. Language adapters may understand syntax and language semantics, but production adapters must not contain code that recognizes a particular specimen such as Cargo, Kafka, or Pydantic in order to make a benchmark pass.

Keep source-backed observations separate from interpretations. A parser observation is evidence. A clause assertion, relation, authority claim, or ranking is a derived interpretation and must retain the observations that justify it.

Do not turn missing evidence into agreement. Unsupported syntax, an absent observation, and proof that a contract is absent are different states.

## Adapters

New adapters should follow [`docs/adapters.md`](docs/adapters.md). At minimum they must preserve source locations and revision status, be deterministic for the same inputs, document side effects and trust boundaries, and emit or support ordinary versioned Chirograph evidence.

## Benchmarks and third-party material

Benchmark machinery must remain generic. A benchmark case may contain specimen-specific data and expected outcomes, but the benchmark runner and production analysis path must not grow specimen-specific branches.

Before adding third-party bytes, read [`docs/licensing.md`](docs/licensing.md). Every benchmark case that bundles or references upstream material must include `provenance.json` conforming to [`benchmarks/provenance.schema.json`](benchmarks/provenance.schema.json). Preserve upstream copyright, license, and NOTICE requirements. Do not assume that public source code is redistributable under Chirograph's license.

## Developer Certificate of Origin

Chirograph uses the Developer Certificate of Origin 1.1 rather than a project-specific contributor license agreement. By signing off a commit, you certify that you have the right to submit the contribution under the project's license and the other terms in the DCO.

Add a sign-off with:

```sh
git commit -s
```

Each contributed commit should contain a line of the form:

```text
Signed-off-by: Your Name <you@example.com>
```

The DCO text is maintained at <https://developercertificate.org/>.

## License of contributions

Unless explicitly stated otherwise before submission, contributions intentionally submitted for inclusion in Chirograph are accepted under Apache-2.0, consistent with the project's [`LICENSE`](LICENSE).
