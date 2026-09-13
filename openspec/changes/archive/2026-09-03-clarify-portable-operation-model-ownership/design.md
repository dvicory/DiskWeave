## Context

See `proposal.md` for motivation. The current requirement `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` delegates only the fixed six-child retained-operation correlation relation through `Reclaimable` to `models/quint/PortableOperationExecutionCore.qnt`.

`models/quint/PortableOperationExecution.qnt` is evidence-only, but it currently repeats the core's slot, child, work, identity, completion, abandonment, and reconciliation state. It also carries a local `TransactionState`, a local `ReleaseAuthorization`, and transitions that resemble transaction release and lifecycle `ReleaseAllowed`. The canonical delegated owners already exist:

- `models/quint/PortableOperationExecutionCore.qnt` owns retained-child correlation through `Reclaimable`.
- `models/quint/RecoveryProtocol.qnt`, delegated by `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent`, owns the bounded write and transaction lifecycle, including `releaseRange`.
- `models/quint/LifecycleRelease.qnt`, delegated by `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts`, owns the bounded composition of seven owner facts into generation-qualified `ReleaseAllowed` and distinguishes later cleanup.

The wrapper is consumed only by `verification/quint/PortableOperationExecutionAnalysis.qnt` and `verification/quint/PortableOperationExecutionFanInAnalysis.qnt`. Independent core consumers are `PortableOperationExecutionCoreAnalysis.qnt`, `PortableOperationExecutionCoreMutants.qnt`, `PortableOperationExecutionCoreConnect.qnt`, and `crates/dwv-service/src/service/tests/retained_operation_core_connect.rs`. The registered evidence is `verification/manifest.toml` plus `docs/verification/retained-operation-execution-core.md`.

Three active changes touch surrounding composition but are not current authority for this repair: `define-portable-writable-session-lifecycle`, `add-portable-shutdown-claim-release`, and `add-scan-independent-writable-startup`.

## Goals / Non-Goals

**Goals:**

- Leave one discoverable delegated owner for each modeled decision and keep the operation-slot requirement ID and product behavior unchanged.
- Make the evidence wrapper visibly non-canonical by locating it under `verification/quint` and composing existing owners through qualified namespaces.
- Remove copied owner state and transitions while retaining evidence-local driver order and cross-owner correspondence checks.
- Make finite identity fixtures explicit trusted inputs and distinguish their suppliers from the transitions each model owns.
- Preserve or strengthen every current bounded, driver, fan-in, mutant, Connect, and implementation-correspondence check and its claim ceiling.

**Non-Goals:**

- No new delegated model, requirement identity, product behavior, operator result, persisted format, service API, scheduling policy, physical-store behavior, or recovery policy.
- No redesign of `PortableOperationExecutionCore`, `RecoveryProtocol`, or `LifecycleRelease` semantics.
- No implementation of active writable-session, shutdown, or startup targets and no dependency on their unsynced semantics.
- No arbitrary-width concurrency, liveness, physical durability, deployed-backend certification, or model-completeness claim.
- No compatibility shim, duplicate model path, re-export, alias module, or temporary second source of truth.

## Decisions

### 1. Source-to-target ownership map

| Current source or construct | Current meaning | Target owner and location | Cutover action |
|---|---|---|---|
| `models/quint/PortableOperationExecutionCore.qnt` | Sole delegated retained-child correlation authority through `Reclaimable` | Same module and path | Retain as the only `dwv:req`-marked model for the operation-slot requirement. Change only the fixture-input parameter boundary required below; do not widen or narrow its transitions. |
| `PortableOperationExecution.qnt` copies of `SlotState`, `SlotChildState`, `WorkState`, `CompletionDisposition`, `ReconciliationState`, `Child`, child actions, abandonment, duplicates, and reconciliation | Shadow copy of the core relation | Qualified `core` instance of `PortableOperationExecutionCore` in `verification/quint/PortableOperationExecution.qnt` | Delete the copies and project the imported core's state/actions directly. |
| `PortableOperationExecution.qnt` `TransactionState`, transaction guards, retained marker, and transaction-release observation | Evidence-shaped copy of transaction lifecycle | Qualified `recovery` instance of `RecoveryProtocol` | Replace local transaction state with owner actions and observations. `RecoveryProtocol.releaseRange` remains the transaction release fact; the wrapper does not infer it from slot state. |
| `PortableOperationExecution.qnt` `ReleaseAuthorization`, release observation, and release transition | Evidence-shaped copy of `ReleaseAllowed` and cleanup | Qualified `release` instance of `LifecycleRelease` | Feed the seven independently classified owner facts to the lifecycle owner, observe exact-generation `ReleaseAllowed`, and represent later cleanup only through the owner's cleanup actions or an explicitly external cleanup result. |
| `ExecutionMode`, driver cursor, fixed protected-write child order, and fan-in entry | Finite correspondence fixture; no delegated product semantics | Evidence-local state in `verification/quint/PortableOperationExecution.qnt` | Retain the minimum phase/cursor state needed to drive the imported owners. Do not mirror owner state or turn the fixed order into scheduling policy. |
| Hard-coded operation, store, incarnation, topology, range, and action fixture values | Finite examples currently embedded in model helpers | Trusted parameters supplied by each analysis or Connect fixture | Pass one exact operation generation and a total child-identity map over that fixture's `ChildIds`. Production/core Connect retains the fixed six-child domain; compact wrapper analysis may retain its declared two-child evidence bound. The core checks exact equality and operation binding but does not establish that upstream values are true. |
| `verification/quint/PortableOperationExecutionAnalysis.qnt` and `PortableOperationExecutionFanInAnalysis.qnt` | Evidence-only two-child driver and fan-in profiles | Same filenames under `verification/quint` importing the relocated sibling composition | Update imports and fixtures; retain driver success, fan-in out-of-order delivery, duplicate handling, owner release composition, and bounded invariant checks. |
| Core analysis, mutant, Connect, and Rust Connect consumers | Direct evidence for the delegated core and implementation mapping | Existing files and core import | Retain direct core ownership. Update only the trusted identity-map instantiation and exact mapped values if the core parameter signature changes. |
| `verification/manifest.toml` and `docs/verification/retained-operation-execution-core.md` | Registered claim, commands, source paths, bounds, and non-claims | Existing evidence locations | Replace the wrapper source path, describe qualified owner composition, retain exact checked commands/results after rerun, and do not broaden the claim. |
| `models/quint/PortableOperationExecution.qnt` | Mislocated evidence wrapper | No target at this path | Delete in the same revision that creates the verification composition and updates all importers/references. |

### 2. External facts and model-owned transitions

| Fact or transition | Classification and owner | Treatment in the evidence composition | Explicit non-implication |
|---|---|---|---|
| Operation generation, fixed child IDs, and per-child operation/store/incarnation/topology/range/action identity | Trusted finite inputs supplied by operation, topology, request, and store owners | Instantiate the fixed-domain core and compare every accepted, executed, delivered, and duplicate observation against the supplied identity map | Input equality does not prove discovery, admission, topology validity, store identity, request normalization, or production cardinality. |
| Driver retained after submission and fixed protected-write action order | Production `WriteDriver` observation; evidence-local sequencing | A minimal local phase/cursor drives the matching imported owner action | The fixture does not own scheduling or prove concurrent service calls. |
| Child emit, accept, execute, deliver, duplicate, abandon, refusal, reconciliation, and `Reclaimable` transitions | Model-owned by the `core` namespace | Drive and inspect the imported core directly; keep no copied child or slot projection | The wrapper cannot manufacture child terminality, reconciliation, or `Reclaimable`. |
| Physical result disposition and exact completion envelope | Store/adapter observation admitted at the existing service boundary | Supply the exact correlated disposition to a core delivery action | Delivery does not establish persistence, durability, recovery authority, or safe retry. |
| Write-record, data/parity, uncertainty, recovery, and transaction release transitions | Model-owned by the `recovery` namespace for its declared bounded relation; concrete observations remain external | Drive qualified `RecoveryProtocol` actions and consume its resulting transaction-release fact | Core completion or `Reclaimable` does not imply transaction release or recovery `CLEAN`. |
| Operation/media effect terminal or authoritatively reconciled | Existing store-operation or recovery owner observation | Supply as one distinct lifecycle observation | It is not inferred from child terminality alone. |
| Terminal children, required reconciliation, and safe `Reclaimable` | Core/operation-slot owner outputs for the exact generation | Project each as a separately named observation into `LifecycleRelease` | No projection may weaken or collapse the three predicates. |
| Applicable semantic transaction release requirement satisfied for the exact operation generation | `RecoveryProtocol.releaseRange` or another applicable transaction owner supplies release within its admitted current-write relation; the applicable correlation owner separately binds that observation to the operation generation | Drive the qualified transaction owner, then project its result only with an explicit trusted current-write-to-operation-generation binding | `RecoveryProtocol` does not decide whether a concrete observation belongs to a prior write after release/reuse, and `RangeReleased` alone does not establish `ReleaseAllowed`. |
| Recovery-owned authoritative reconciliation | Recovery owner observation | Supply separately to `LifecycleRelease` | The wrapper does not derive recovery authority or `CLEAN`. |
| Basis consumed, discarded, or authoritatively reconciled | Healthy-service composition owner observation | Supply separately to `LifecycleRelease` | Child completion and transaction release do not imply basis conformance. |
| Applicability to release composition | External admission/owner scope | Instantiate `ApplicableGenerations`; never derive it from termination | Routine read or flush termination does not enter the release relation. |
| `ReleaseAllowed(operation-generation)` | Model-owned by the `release` namespace from all seven facts | Observe the imported authorization set for the exact generation | It is not generic completion, transaction release alone, or physical cleanup. |
| Cleanup request, failure, or success after authorization | Lifecycle model cleanup observation plus operation-slot/resource owner effect | Use the imported cleanup relation or record an external result without copying release state | Cleanup cannot revoke semantic authorization or authorize generation reuse by itself. |

### 3. Parameterize trusted fixture identity without transferring authority

Replace literal identity construction in the core with a required total identity mapping over the instantiated `ChildIds`, alongside the existing operation-generation parameter. The production/core Connect fixtures retain the fixed six-child domain; compact wrapper analyses may retain their declared two-child evidence bound. Each analysis and Connect fixture supplies its finite values. Preserve the invariant that every child identity names the instantiated operation generation and preserve exact equality guards on acceptance, execution, delivery, and duplicate observation.

The input map is assumed data for a finite model instance. The core owns correlation transitions only. It does not validate whether the named stores exist, choose the topology or range, normalize a request, assign a generation, establish an incarnation, or certify upstream evidence. Invalid or mismatched observations remain disabled before state mutation, and existing negative identity probes remain mapped to production rejection paths.

No new general fixture schema or identity owner is introduced. A single required map is the minimum boundary that removes embedded product-looking values and lets all existing profiles share the same explicit input contract.

### 4. Compose owners without shadow state

The relocated module keeps the name `PortableOperationExecution` but imports the three canonical modules with qualified namespaces such as `core`, `recovery`, and `release`; wildcard imports are not used where owner names can collide. Its composite step relation may select among owner actions and evidence-local phase changes, but it does not assign imported owner state directly or cache derived copies of owner states.

Cross-owner projections are one-way evidence adapters:

1. trusted fixture and production observations drive the applicable owner action;
2. the composition observes the resulting owner predicate under its qualified namespace;
3. it supplies that predicate as a separately named input to the next owner relation;
4. invariants assert exact-generation agreement and prevent an evidence-local phase from advancing when the supplying owner predicate is absent.

A projection failure is an ownership or correspondence signal. The implementation must repair the mapping or surface a semantic conflict; it must not add a bypass action, fabricated observation, compatibility alias, or weaker guard to make a trace pass.

### 5. Clean cutover and reference closure

Create `verification/quint/PortableOperationExecution.qnt`, update the two wrapper analysis imports to that sibling, then delete `models/quint/PortableOperationExecution.qnt` in one revision. Keep `PortableOperationExecutionCore.qnt` in `models/quint` and keep all direct core consumers pointed at it. Search the repository for both portable-operation module names and old relative paths; update every result in analyses, Connect/Rust correspondence, mutants, verification registry, evidence, reviewed relationship state, and maintained references. Historical archived OpenSpec artifacts remain immutable history and are not rewritten.

There is no transition period. The target contains no alias, forwarding module, copied type compatibility layer, or deprecated path.

### 6. Verification preservation

| Existing evidence | Required target evidence |
|---|---|
| Core bounded-domain, delayed/out-of-order reconciliation, abandonment, and invariants | Retain the same six-child delegated domain, scenarios, invariant set, and bounded depth; update only explicit fixture inputs where required. |
| Six deterministic core mutation canaries | Retain all six canaries and their exact named defect claims: execution before acceptance, wrong accepted identity, terminal without disposition, unaccepted sibling surviving failure, accepted child refused on abandonment, and premature reclaim. Do not substitute source-text assertions. |
| Core Connect Quint scenarios and nine Rust correspondence probes | Retain success, duplicate disposition, abandonment, failed accepted-sibling retention, short, uncertainty, child-identity rejection, and completion-identity rejection through real production observations. Keep the delegated correspondence endpoint at `Reclaimable`. |
| Wrapper driver analysis | Preserve the successful retained-driver protected-write sequence and add or retain exact qualified checks that transaction progress comes from `RecoveryProtocol` and release authorization comes from `LifecycleRelease`. |
| Wrapper fan-in analysis | Preserve two-child accepted fan-in, reverse execution/delivery, duplicate observation, reconciliation, transaction satisfaction, `ReleaseAllowed`, and cleanup while using imported owner transitions. |
| Existing four-step bounded wrapper checks | Preserve the checked safety predicates or replace each copied predicate with an equivalent or stronger qualified owner predicate. A state-space concern may justify a smaller owner prefix only if the same behavior and negative boundary remain checked elsewhere and the evidence records the split; it may not justify dropping a property, bound disclosure, or failure path. |
| Manifest and retained-operation evidence | Record exact commands, bounds, paths, results, owner projections, and unchanged non-claims after implementation. Do not reuse old results for the relocated composition. |

The implementation phase runs these checks; this planning change intentionally runs none.

### 7. Active-change and affected-dependent boundary

| Change or owner | Review obligation | Exclusion |
|---|---|---|
| Current `store-operation-contracts` requirement and direct dependents | Use the normal affected-owner workflow and review each reported dependent with a concrete reason | Do not bulk-accept reviewed state or allocate a new requirement ID. |
| `explicit-transaction-machine` / `RecoveryProtocol` | Confirm qualified composition consumes the current delegated relation without changing its state, outcomes, ordering, or release meaning | Do not redesign transaction or recovery semantics. |
| `healthy-portable-io` / `LifecycleRelease` | Confirm all seven owner facts, applicability, monotonic exact-generation authorization, and cleanup distinction remain intact | Do not infer `ReleaseAllowed` from core or transaction completion. |
| `define-portable-writable-session-lifecycle` | Review the target's durable begin/close facts as adjacent future composition | Do not import its unsynced session semantics or add a session model here. |
| `add-portable-shutdown-claim-release` | Confirm shutdown remains a downstream consumer of operation `Reclaimable`, settled effects, session close, endpoint withdrawal, and independent release authority | Do not implement shutdown ordering or clean-close policy. |
| `add-scan-independent-writable-startup` | Confirm startup retains topology, stabilization, store/recovery claim, session begin, epoch, and publication authority | Do not make fixture identities or the correspondence wrapper an admission authority. |

### 8. Review gates

1. **Planning-artifact gate before implementation:** an independent reviewer examines the exact proposal, delta, design, and tasks against current canonical ownership, all three delegated models, every discovered importer/reference, active-change churn, OpenSpec rules, and explicit non-claims. Every evidence-backed blocker is repaired and materially changed boundaries are re-reviewed before the apply phase starts.
2. **Ownership cutover gate:** after implementation, repository reference closure must show one delegated operation model at `models/quint/PortableOperationExecutionCore.qnt`, one evidence composition at `verification/quint/PortableOperationExecution.qnt`, no old wrapper, and no shadow child, transaction, or release state machine.
3. **Verification/evidence gate:** the retained bounded, mutant, Connect, driver, fan-in, and focused production checks pass with current results recorded; no check, property, bound disclosure, or non-claim is weakened.
4. **Fresh exact-snapshot adversarial gate:** after all implementation, verification, evidence, and affected-dependent updates, a fresh independent reviewer audits that exact snapshot for model ownership, imported-owner fidelity, trusted-input boundaries, clean-cutover closure, verification preservation, evidence truthfulness, and active-change isolation. Repair every blocker and rerun review after any material boundary repair. This is distinct from the planning-artifact gate and must complete before sync or archive.
5. **Canonical handoff gate:** only after the preceding gates pass, synchronize the delta, currentize requirement/model/evidence relationships, run the repository's required OpenSpec and knowledge checks, archive the change, and retain the same stable requirement ID.

## Risks / Trade-offs

- Qualified composition can enlarge the bounded state space. Use focused owner prefixes and projections only where they preserve each current checked behavior and keep all bounds explicit; never replace a model check with a hand-authored success trace alone.
- The current wrapper's repeated state makes traces easy to read. Removing copies may make qualified traces longer, but it eliminates semantic drift and makes ownership review reliable.
- Treating identity values as trusted inputs can be misread as validation. The spec, module comments, evidence, and Connect mapping must repeat that the checks establish exact correlation only, not upstream truth.
- Active lifecycle work may later change surrounding composition. This repair deliberately targets current owners and requires affected review; later changes must adapt their own consumers rather than preloading future semantics here.
- A direct import may reveal that two owner relations cannot be composed without an unowned consequential decision. That is a semantic reconciliation blocker, not permission to add another delegated model. Add a new owner only if review identifies a genuinely independent current decision and processes it through a separate approved OpenSpec change.
