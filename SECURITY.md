# Security policy

Chirograph consumes evidence and, through adapters, may inspect source from repositories that should be treated as untrusted input. Security boundaries must therefore be explicit rather than inferred from the word "read-only."

## Supported versions

Chirograph is pre-1.0. Security fixes are made on the current development line; older revisions are not promised security backports.

## Reporting a vulnerability

Please report vulnerabilities privately. Prefer GitHub's private vulnerability reporting or Security Advisory flow when it is available for this repository. If that facility is unavailable, contact the repository maintainer privately through GitHub rather than publishing exploit details in an issue.

Include the affected revision, the input required to reproduce the problem, the observed impact, and any known constraints on exploitation. Do not include secrets or third-party private source in a report unless the reporting channel is appropriate for that material.

## Current trust boundary

At the current CLI surface, `chirograph inspect <evidence.json>` reads the named local file, parses `chirograph-evidence-v1`, validates the graph, and renders an assessment. That path does not execute the analyzed program and does not itself make network requests.

That guarantee is scoped to the current `inspect` path, not to every adapter or future acquisition mechanism. Adapters are separate trust boundaries. An adapter that launches a runtime, executes tests, invokes a package manager, follows symlinks, reads outside an analysis root, or uses the network must make that behavior explicit in its own documentation and interface.

Chirograph is not a sandbox. Parsing untrusted source safely and executing untrusted source safely are different problems.

## Data and remote services

The core evidence model and current `inspect` command do not require sending source or evidence to a remote model or API. Any future feature that transmits source, evidence, prompts, or derived artifacts off-machine must be explicit and documented, including what is sent, where it is sent, and whether it is retained.

Repository retrieval performed by tests, demos, or adapters is not an implicit permission to transmit unrelated local source.

## Security expectations for adapters

Adapters should default to the least capability necessary. Prefer parsing caller-supplied bytes over executing project hooks. Do not silently run build scripts, repository-local plugins, shell commands, or dependency installation as a side effect of parsing. Preserve exact source provenance so a security report can identify the bytes that were analyzed.
