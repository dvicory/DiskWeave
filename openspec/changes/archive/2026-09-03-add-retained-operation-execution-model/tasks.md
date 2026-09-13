## 1. Reviewed model and delegation

- [x] 1.1 Build the bounded retained-operation relation and compact driver/fan-in profiles; verify canonical/profile typecheck and deterministic Quint scenarios pass.
- [x] 1.2 Obtain adversarial review of the model and repair execution, ownership, identity, uncertainty, abandonment, aggregate-state, duplicate, and explicit-release findings; verify compact bounded checks pass.
- [x] 1.3 Obtain adversarial review of `proposal.md`, `design.md`, and the `store-operation-contracts` delta; verify the delegation is no broader than the reviewed model and no redundant canonical prose is removed.
- [x] 1.4 Apply only the reviewed delegation delta after the model and artifact review return `GO`; verify strict OpenSpec validation passes.

## 2. Production correspondence

- [x] 2.1 Add a Connect model wrapper with deterministic six-child retained-driver paths through `Reclaimable`, exact identity inputs, failure/uncertainty/abandonment and owner-reconciliation probes; keep fan-in/release model profiles as separate evidence-only checks; verify Quint typecheck and model-run traces.
- [x] 2.2 Add the Rust correspondence driver and focused service tests; verify real `HealthyPortableService`, retained `WriteDriver`, `OperationSlotTable`, and `TransactionMachine` observations match model states without fabricated evidence.
- [x] 2.3 Obtain adversarial review of the Connect wrapper, Rust driver, focused tests, and all model-to-Rust mappings; repair every blocking finding and preserve explicit non-claims.

## 3. Evidence and completion

- [x] 3.1 Run bounded core and correspondence model checks, deterministic Connect seeds, focused `dwv-service` tests, and affected Rust quality checks; record exact commands, bounds, and results.
- [x] 3.2 Update the verification manifest, portable verification evidence, and reviewed endpoint state with the exact core/Connect scope, owner boundaries, non-claims, and evidence digests.
- [x] 3.3 Run strict OpenSpec validation and pre-currentization documentation knowledge readiness/checks; verify the active delegation, implementation correspondence, and maintained evidence are coherent.
- [x] 3.4 At the atomic currentization of this delegated requirement, add the source-local `// dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` marker to `models/quint/PortableOperationExecutionCore.qnt`, then run the post-currentization knowledge readiness/check against the resulting requirement/model relationship. Synchronize and archive the completed OpenSpec change only after all implementation, review, verification, evidence, and knowledge checks pass; record the final revision and durable decisions.
