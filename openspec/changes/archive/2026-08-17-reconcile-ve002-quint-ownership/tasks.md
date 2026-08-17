## 1. Ownership repair

- [x] 1.1 Review all four existing transaction-machine requirement IDs and record why only normalized actions and reference traces retain independent responsibilities.
- [x] 1.2 Remove the durable-intent, clean/checkpoint, and failure/abandonment requirement blocks and their stale relationship edges.
- [x] 1.3 Remove exact action-order wording from normalized action emission while preserving semantic action vocabulary, evidence fields, and backend-fanout normalization.
- [x] 1.4 Name the canonical `RecoveryProtocol` module and separate its protocol semantics from the finite `RecoveryProtocolAnalysis` evidence instance.

## 2. Dependent rewiring

- [x] 2.1 Rewire current healthy-portable-io consumers to the retained normalized-action and reference-trace owners.
- [x] 2.2 Update the active portable-shutdown change so its target names the retained reference-trace owner and not a removed transaction-outcome requirement.
- [x] 2.3 Rewire the evidence manifest, curriculum, maintained guide, and current campaign references; preserve historical records.
- [x] 2.4 State in design and maintained evidence that the model source is semantic authority while runs and evidence are verification only.

## 3. Boundary verification and sync

- [x] 3.1 Validate the corrective change and inspect all affected dependents before canonical sync.
- [x] 3.2 Run focused Quint, Rust, OpenSpec, and documentation checks; confirm no retained scenario restates delegated states, outcomes, order, or release sequencing.
- [x] 3.3 Sync the verified delta into current specs and archive the completed change without rewriting historical references.
