# Cross-representation drift fixture

`retry-safety.json` is Chirograph's first deliberately heterogeneous drift specimen. It models one logical recovery guarantee across documentation, a test, and runtime source code:

> retry after an indeterminate transport failure does not duplicate the mutation

The fixture exists to prove that Chirograph derives contract status only from recorded evidence. A representation being present in the graph is not itself support or contradiction. The completed specimen must preserve supporting evidence from documentation and tests alongside contradictory runtime evidence and report the clause as `CONTESTED`.

No fixture-specific logic belongs in the core or CLI.
