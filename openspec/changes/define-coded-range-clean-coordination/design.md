## Context

See `proposal.md` for the motivation and selection boundary. The current owners already provide the pieces that this design must compose:

- `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity` owns service orchestration for checked dirty mapping, recovery invalidation, parity computation, and exact store writes.
- `req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions` owns the semantic action vocabulary, including range acquisition and release. `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent` delegates the exact admitted single-write lifecycle to `models/quint/RecoveryProtocol.qnt`.
- `req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic`, `req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence`, and `req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations` own recovery generation/topology checks, evidence admissibility, and uncertain commit handling.
- `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked`, `req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence`, `req.dirty-integrity-invalidation.failures-and-restart-are-conservative`, and `req.dirty-integrity-invalidation.transitions-and-evidence-are-deterministic` own dirty geometry, exact `CLEAN` region selection, conservative failure, and transition evidence.
- `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence`, `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence`, `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`, and `req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit` own store ranges, watermarks, operation lifetime, and bounded resources.
- `req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch` and `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` own immutable topology snapshots and binding rejection. `req.xor-reference-model.xor-parity-uses-explicit-protected-geometry` owns parity geometry and bytes.
- `req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts` is the canonical owner of exact generation-qualified `ReleaseAllowed`; this change consumes that result and does not restate its lifecycle prerequisites.

The current `RecoveryProtocol` model represents one active write, one abstract owned range, generic `Regions`, and explicit release. It does not represent multiple concurrent operations, a coded-range conflict key, or a recovery-`CLEAN` capture boundary. Those missing transitions are not silently inferred from the model.

The v0.8 and v0.9 roadmap clauses and the v0.9 normalization campaign are retained target/reconciliation inputs only. Current OpenSpec requirements remain the semantic starting point until this change is verified and later synchronized.

## Goals / Non-Goals

**Goals:**

- Give exactly one owner the portable conflict domain for mutations and relevant reads: the mapped coded parity/codeword range under one captured topology and profile.
- Give exactly one owner the closed-set rule for recovery `CLEAN`, while making it consume rather than redefine coded-range authority.
- Make conflict classification, capture linearization, refusal, stale-capture, abandonment, release, failure, restart, and evidence outcomes deterministic at the semantic boundary.
- Preserve independent request, coded/store, dirty-region, and checksum geometries and the current owner of each.
- Leave downstream consumers a bounded, mechanism-neutral contract without assigning their startup, shutdown, publication, or currentization policy here.

**Non-Goals:**

- Choosing advisory locks, lock quantum, sharding, actor/task topology, queue or ring structure, scheduling/fairness policy, or a generic concurrency framework.
- Defining scan-independent epoch admission, fresh-basis currentization, post-gap custody, background rollover, historical-claim retention, prior-version recovery, production P/Q, or Linux behavior.
- Moving persistence-evidence, transaction lifecycle, store watermark, operation-slot, topology, dirty-region, checksum, or parity-byte ownership into either new requirement.
- Defining writable-session begin/close, shutdown, deployment, endpoint publication, or mount ordering.
- Editing `RecoveryProtocol.qnt`, adding verification fixtures, or implementing product code in this planning change.

## Decisions

### 1. Split ownership by decision, not by call path

The change adds two owner requirements in two existing capabilities and modifies the healthy service only as a downstream consumer:

| Decision | Owner | Composed by |
|---|---|---|
| Which operations conflict, how complete coded authority is admitted, and when a claim is removable after external release authorization | `req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts` | healthy writes and mutation-basis reads for admission/effect; canonical lifecycle-release `ReleaseAllowed` for removal |
| Which mutations a recovery `CLEAN` may clear and how its durable post-capture cut is enforced | `req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set` | the coded-range owner, dirty-region owner, recovery generation/evidence owners, and healthy durable completion |
| How the service consumes both decisions | modified `healthy-portable-io` write and durable-completion requirements | the two new owners and all existing lower-level predicates |

The transaction owner does not decide which persistence evidence authorizes `CLEAN` or whether a lifecycle result is terminal. The dirty owner does not decide whether two member operations conflict or authorize coded claim removal. Healthy service orchestration consumes both owners without becoming a second conflict or closed-set owner, while the canonical lifecycle-release requirement remains the sole owner of final `ReleaseAllowed`.

The dependency is explicit: `coded-range-authority-covers-shared-parity-conflicts` consumes the canonical lifecycle-release `ReleaseAllowed` only for claim removal; `recovery-clean-captures-a-closed-mutation-set` consumes externally validated complete coded-scope classification; and the modified healthy requirements consume both new owners. The coded-range requirement does not depend on the modified healthy consumer requirements or the closed-set requirement. Neither new owner depends on writable-session, startup, shutdown, or publication semantics.

The healthy-portable-io delta preserves its existing orchestration and result boundaries while requiring complete coded admission for protected writes and closed-set permission plus the durable post-capture cut for durable completion/`CLEAN`. It does not alter degraded-read quiescence or define a new operation mechanism.

### 2. Keep five geometries and their mappings separate

Every admitted semantic mutation unit carries the following semantic values, with checked mapping between them. A frontend request MAY decompose into several units, but no unit is admitted until its coded scope is complete:

1. **Request geometry:** the normalized frontend target range and operation identity.
2. **Coded geometry:** the interval(s) in each configured coded parity/codeword range produced by the captured topology, coding profile, protected lengths, and request/decomposition range. This is the only conflict key.
3. **Store geometry:** exact per-member data/parity byte ranges submitted to stores and reported by store outcomes.
4. **Dirty geometry:** recovery-owned dirty-region identities derived from each affected member range.
5. **Checksum geometry:** target-specific checksum extents and their target/content generations.

Partial physical acquisition is pre-admission and authorizes no I/O. A shared dirty region, store interval, or checksum extent is not by itself a coded conflict. Conversely, different member byte ranges that map to one coded range conflict even when their physical stores and dirty/checksum partitions are disjoint. The new contracts expose all mappings in evidence rather than allowing an implementation to substitute one geometry for another; exact classification remains local to the supplied scope.

### 3. Coded-range owner and delegated relation

The coded-range owner defines the product meaning and external bindings:

- a validated semantic mutation unit (which MAY be a request-decomposition unit) must carry one complete mapped coded claim covering every dependent basis observation and mutation before admission; an undecomposed frontend request is not itself a complete scope unless its mapping proves that fact;
- partial physical acquisition is pre-admission and authorizes no dependent I/O; exact classification is local to the supplied scope and does not require a global lock, counter, or connected-component interpretation;
- the captured topology and coding profile map each mutation and relevant mutation-basis read to a coded parity/codeword claim;
- intersecting coded claims conflict even when member identities, store ranges, dirty regions, or checksum extents differ;
- a relevant basis observation is correlated to its consuming semantic mutation unit and must remain coherent under that unit's complete coded authority through any consuming mutation; otherwise healthy-service composition must establish discard or reconciliation before the canonical lifecycle owner can supply `ReleaseAllowed`. Independent overlapping observations coordinate, with no release-then-use gap or unmodeled claim upgrade;
- the healthy service may perform dependent I/O only after coded authority and bounded operation/resource admission; their relative acquisition order and physical mechanism are not product semantics;
- bounded refusal, stale topology/profile/generation, short/failed/uncertain results, abandonment, restart, and release retain existing owner interpretations and cannot produce a stronger clean or durability claim.

The coded owner consumes exact owner-approved facts for mapping validity, operation/resource admission, recovery generation and write-recovery-record/data-parity observations (`CommitDurable`, `CommitRejected`, or operation/media-effect `CommitUnknown`), store ranges/watermarks/persistence evidence, authoritative reopen/reconciliation, and exact external `ReleaseAllowed`. The canonical lifecycle-release requirement is the sole owner of that authorization; the coded owner does not inspect or reconstruct its terminality, child, transaction-release, recovery, basis-conformance, or cleanup predicates. CLEAN-commit uncertainty remains independently effective for capture cleanup and future exclusion but neither supplies nor negates `ReleaseAllowed`.

The delegated projection is limited to coded conflict, capture classification/acceptance, durable post-capture cuts, CLEAN cleanup eligibility, and claim removal after external `ReleaseAllowed`. It does not model basis lifecycle, basis-read correctness, lifecycle terminality, recovery release policy, persistence admissibility, topology mapping, geometry construction, operation resources, sessions, or product claims.

### 4. CLEAN owner and delegated relation

The dirty-integrity owner defines the product meaning and external bindings:

- a `CLEAN` capture names a stable capture identity, selected dirty/checksum geometry, an externally validated complete coded scope covering every coded claim whose admitted mutation could invalidate the selected state whether admitted before, during, or after capture, recovery generation, lower admission frontier, and capture frontier;
- mutations classified at or before the capture frontier enter the captured set; existing transaction/operation/recovery lifecycle owners supply exact disposition facts, while the CLEAN owner supplies one explicit capture-wide accepted or rejected decision after deciding whether those facts satisfy this capture, complete coded scope, and selected state. A generic handoff, requested action, attempted persistence, classification, or model transition is insufficient;
- before a later mutation affects media, the captured `CLEAN` is durably committed before its durable dirty/recovery boundary, or the mutation durably establishes a newer exact boundary/frontier/generation that stales/refuses the older `CLEAN`; already-DIRTY optimization requires an owner-approved durable boundary/frontier/generation, not classification alone;
- exact selected clearing consumes dirty-region/checksum, recovery-generation, topology, store-watermark, persistence-evidence, supplied lifecycle disposition, the CLEAN-owner capture-wide accepted decision, and commit-observation facts. `Durable`, `Rejected`, and CLEAN-commit `Unknown` remain distinct; CLEAN-commit `Unknown` requires authoritative recovery/reopen reconciliation before `CLEAN` or capture cleanup;
- stale captures, missing evidence, refusal, abandonment, uncertain CLEAN commit, failure, and restart preserve conservative dirty/indeterminate consequences and bounded capture evidence. Separately, the canonical lifecycle owner may withhold `ReleaseAllowed`; CLEAN-capture uncertainty alone neither changes that decision nor retains an otherwise independently releasable operation slot or coded claim.

The CLEAN owner projection delegates capture-side membership, before/after classification, and consumption/validation of the externally supplied capture-wide accepted/rejected decision to the same `CodedRangeClean.qnt` after explicit focused-review approval. The relation does not recompute that decision from lifecycle disposition, dirty/checksum policy, persistence admissibility, coded authority removal, `RecoveryProtocol.releaseRange`, or a second release policy. CLEAN-capture cleanup and retained capture exclusion evidence remain distinct from coded-claim release.

### 5. Make refusal, abandonment, and restart observable

The semantic result boundary preserves these dispositions:

| Boundary | Accepted result paths | Conservative non-success paths |
|---|---|---|
| Coded-range admission | admitted complete coded scope; claim removal eligible only after exact external release/disposition authorization | bounded conflict/exhaustion refusal; stale topology/profile/generation refusal; failure, frontend abandonment without terminal/reconciled release, unknown operation effect, or unreconciled basis while claims remain owned |
| Relevant read | exact completed range correlated to its consuming unit and held coherently through the consuming mutation under complete authority | short, failed, uncertain, or stale result without a coherent-basis claim; no release-then-use gap |
| Mutation release | exact external `ReleaseAllowed` from the canonical lifecycle-release owner authorizes coded-claim removal independently of CLEAN-capture state | no removal while that owner withholds `ReleaseAllowed`; coded/CLEAN logic does not reconstruct lifecycle predicates or redefine `RecoveryProtocol.releaseRange` |
| `CLEAN` capture/commit | exact selected-region clear after closed-set completion, one exact CLEAN-owner capture decision, durable post-capture cut, and all evidence owners accept | deferred/dirty later mutation; bounded capture refusal; stale generation/topology/evidence refusal; `Rejected` or CLEAN-commit `Unknown` requiring reconciliation while bounded capture evidence remains effective |
| Process loss/restart | reopen and reconcile durable operation, basis, capture, generation, and disposition state before reuse | dirty/indeterminate evidence and owned reconciliation; no inferred clean state, coded release, or silent claim reuse |

Frontend abandonment suppresses delivery interest only. It does not cancel a mutation, release coded authority, remove a mutation from a closed set, or authorize `CLEAN`. Process death may release mechanism-owned ephemeral resources according to existing owners, but durable dirty/indeterminate consequences and unresolved commit outcomes remain authoritative. Coded claims remain owned until exact external `ReleaseAllowed`; an unresolved capture preserves bounded exclusion evidence rather than pinning otherwise releasable operation state.

### 6. Preserve deterministic, implementation-independent evidence

A normalized evidence record for coded coordination includes operation/decomposition identity and generation, captured array/topology epoch, assignment generations, profile/coding identity, request range, complete coded claim intervals, exact store ranges, dirty-region IDs, checksum extents, correlated relevant-read ranges, basis consumption/discard result, claim/resource admission observations, admission disposition, terminal/reconciliation result, and owner-approved release authorization/result. It distinguishes a coded conflict from a dirty/checksum/store overlap and never substitutes private handles or backend queue identity.

A `CLEAN` capture evidence record additionally names the capture identity, lower and capture frontiers, an externally supplied bounded lower-frontier summary rather than an unbounded historical operation list, externally supplied membership observation, generation-qualified operation identities, captured recovery generation, future-inclusive complete coded scope, selected dirty/checksum geometry, active or unreconciled mutation obligations since the lower frontier and at or before the capture frontier, exact lifecycle disposition facts from existing owners, one CLEAN-owner capture-wide accepted/rejected decision, durable post-capture boundary/frontier/generation, later mutations observed after the capture frontier, recovery `Durable`/`Rejected`/`Unknown` commit or reconciliation observation, and later-cut durable/rejected/unknown observations with their reconciliation outcome.

Evidence is bounded by existing report/resource limits. A named frontier and owner-approved exact generation/frontier summary bound already-accounted history; the contract does not require an unbounded mutation-identity list and does not permit an overlapping active or unreconciled obligation since the frontier to be dropped. This bounded capture evidence, not unbounded operation history or permanently retained released claims, guards future overlap until CLEAN reconciliation or a named exact stale boundary exists.

### 7. Delegate only to a new bounded CodedRangeClean model

The candidate delegated relation is `models/quint/CodedRangeClean.qnt`.
`models/quint/RecoveryProtocol.qnt` remains byte-for-byte separate and owns
its existing single-write release relation. The canonical relation receives
only validated complete claims/scopes and owner observations through explicit
actions; finite operation, capture, coded-unit, and input sets belong only to
verification instances.

The refined projection-feasibility map below is the declaration-level
correspondence record. Model names identify the exact bounded relation being
projected; they do not create a second production authority.

| Actual declaration(s) | Delegated meaning | Production seam or external owner/evidence |
|---|---|---|
| `OperationPhase`, `Operations` map key, `CodedClaim`, `claimOverlap`, `admitComplete`, `permitMutationEffect`, `noHeldOverlap`, `NoHeldCodedOverlap`, `CompleteAdmissionPrecedesEffect` | One model operation identity is one exact generation-qualified production operation-slot/assignment identity. Complete validated coded claims conflict exactly when their coded-unit sets intersect; disjoint claims may coexist; dependent effects require admitted complete authority and every applicable later cut. | A future Connect bridge observes the production slot generation/assignment identity represented by the model key beside the complete validated claim, admission/refusal result, and effect boundary. Topology/profile mapping, member identity, basis coherence, and physical I/O remain external transaction/healthy-service evidence. |
| `CaptureScope`, `startCapture`, `capturesAfterAdmission`, `CaptureOperationStatus`, `CaptureMembershipExhaustive`, `CaptureMembershipExclusive` | For an active capture, membership is exhaustive and exclusive over the bounded operation domain: intersecting operations held at capture start are `CaptureIncluded`; later admissions are `CaptureLater`; definitive refusal stops future obligations. | The bridge observes capture identity, complete scope, `lowerFrontierCovered`, generation-qualified membership, and refusal/classification. Dirty-integrity owns frontier meaning and the bounded summary. |
| `CaptureSatisfactionObservation`, `observeCaptureSatisfaction`, `captureCleanEligible`, `requestClean`, `NoFalseClean`, `RejectedSatisfactionRefusesCapture` | The CLEAN owner supplies one capture-wide accepted/rejected decision after evaluating lifecycle, dirty/checksum, persistence, and selected-state facts. The relation consumes that fact and never derives it from a generic handoff or a per-operation map. Rejection preserves conservative dirty state and refuses the capture. | The bridge observes the capture identity, closed membership, and externally supplied CLEAN-owner decision; dirty-integrity and lifecycle owners supply the decision inputs. |
| `CleanCommitObservation`, `CaptureCommitUnknown`, `observeCleanCommit`, `reconcileCleanCommit`, `UnknownCleanRequiresReconciliation` | CLEAN-commit `Unknown` is represented only by `CaptureCommitUnknown`; the action input is not stored separately. It cannot become `CaptureCleanKnown` without the separate authoritative reconciliation action and does not itself negate independent coded release. | Recovery adapters supply exact durable/rejected/unknown commit and reopen/reconciliation facts; the relation preserves capture uncertainty separately from operation release. |
| `LaterCutObservation`, `CaptureOperationStatus`, `observeLaterCut`, `LaterCutReconciliationObservation`, `reconcileLaterCut`, `allLaterCutsSatisfied`, `LaterEffectRequiresDurableCut` | A later mutation may affect media only after an owner-supplied durable post-capture cut. Later-cut `Unknown` is a distinct blocking membership status. If CLEAN is already durable, reconciliation may establish only the durable-after-CLEAN cut or rejection and preserves `CaptureCleanKnown`; before durable CLEAN, reconciliation may establish only the stale/refusal cut or rejection. | The bridge observes the exact operation/capture/generation boundary, its ordering relative to durable CLEAN, and the reconciliation result. Dirty-integrity and recovery owners supply durability and stale-generation meaning; the model never creates the boundary. |
| `CaptureRefused`, `captureTracksFutureAdmission`, `RejectedSatisfactionRefusesCapture` | Definitively refused captures retain dirty/stale consequences but do not classify future admissions or impose an impossible durable-cut obligation. No retirement/compaction state is modeled because no current/proposed owner requirement establishes one. | The bridge observes refusal and exact final capture facts. Dirty-integrity owns conservative dirty/stale meaning; lifecycle and recovery owners supply finality and reconciliation facts. |
| `removeCodedClaim`, `ReleaseAuthorization` | Exact external `ReleaseAllowed` is consumed directly by coded removal and is the sole coded-removal authority. CLEAN capture state and later-cut reconciliation do not add a second release policy. | The bridge binds the model operation key to the exact production operation generation, observes `ReleaseAllowed` from the canonical lifecycle-release owner for that same identity, then observes coordinator removal. The seven lifecycle predicates and `RecoveryProtocol.releaseRange` remain external and are not reconstructed here. |

The model does not synthesize topology/profile mapping, request/store/dirty/
checksum geometry, lifecycle terminality, persistence admissibility, recovery
correctness, resources, sessions, physical locking, product claims, or global
frontiers/counters. Each declaration above has either the named bridge seam or
the named external owner/evidence; no other consequential distinction is
delegated.

The verification lane contains bounded exhaustive-invariant commands, non-vacuous action/outcome witnesses, deterministic replay hooks, and seeded negative variants. Bounds and mutant expectations remain evidence-only; they do not add product cardinality, a global counter, lock, queue, or frontier mechanism.

The delegated relation retains every valid coded conflict, complete-scope admission, CLEAN membership, durable-cut, refusal, uncertainty, reconciliation, and externally authorized removal behavior named above. The owner delta requirements retain product meaning, ownership, assumptions, external facts, claims, and non-claims; `models/quint/CodedRangeClean.qnt` is the sole authority only for the bounded exact relation declared in the projection map, and `models/quint/RecoveryProtocol.qnt` remains separate and unchanged.

### 8. Implementation boundary and unresolved blockers

The focused repair and projection-feasibility gate is approved. This apply
slice creates the Connect-ready canonical relation and its bounded analysis,
replay, and mutant evidence only; it does not implement coded/CLEAN Rust or
Connect. `RecoveryProtocol.qnt` and its analysis/Connect artifacts remain
separate and unchanged. The operation-lifecycle prerequisite lane is complete:
`ReleaseAllowed` is canonical, generation-qualified, implemented, and
LifecycleRelease-Connect-backed. Coded-clean implementation is now gated on
the coded/CLEAN bridge seams below.

The first Rust pass SHALL build the actual Connect bridge for every delegated
projection with an observable correspondence; it SHALL NOT build a full
driver before those seams exist. Bridge/correspondence evidence and this
refined map are required before implementation completion. The future bridge
must bind complete claims, capture membership/frontiers, basis-read lifetime,
owner disposition/evidence, durable post-capture cuts, CLEAN commit
observations, reconciliation, and exact `ReleaseAllowed` to implementation
observables. This change imposes no ordering on startup, shutdown, session,
publication, currentization, or scan-independent work.

The following remain deliberately unresolved implementation choices, not
semantic alternatives:

- the concrete authority primitive and its storage/lifetime representation;
- finite capacity values and whether bounded refusal is surfaced as
  backpressure or a named conflict result;
- production coding profiles beyond the externally captured profile;
- the coded/CLEAN implementation seams and Connect bindings named by the
  projection map.

The analysis bound is evidence-only: three operation identities, two capture
identities, two coded units, finite valid/invalid claim and scope inputs, and
the declared CLI depth/seed in the evidence record. The completed end-to-end
profile bounds are conflict depth 3, capture depth 5, uncertainty-prefix depth
4, and composition depth 6. Both `CleanCommitUnknown` and
`LaterCutUnknown` are witnessed from canonical `init` at completed depth 4.
Their deeper reconciliation continuations are covered by the canonical-prefix
phase-cut profiles, which complete depth 6 from canonical `init` through their
modeled prefixes but do not establish external lifecycle preconditions or
end-to-end production coverage. Timed-out deeper attempts are recorded only
as non-claims in the evidence record. These bounds are not production
cardinality, a global counter, lock, frontier, or arbitrary-width proof.

Any implementation choice must preserve the exact owner decisions and return
paths above. A choice that changes the conflict domain, admits a post-capture
mutation into an old `CLEAN`, releases uncertain or unreconciled work early,
manufactures a lifecycle/recovery fact, omits the Connect bridge/correspondence
evidence, or starts Rust work before the required seams is non-conforming.
## Risks / Trade-offs

- Coded-range authority may reduce concurrency relative to per-member coordination. That is intentional: parity correctness is shared at the codeword boundary, and the mechanism can optimize disjoint ranges later without changing the semantic key.
- A later mutation may remain dirty even when an earlier `CLEAN` commits. This makes state conservative and requires a later clean, but it prevents a race from manufacturing a clean claim.
- A closed set can retain operation ownership while a mutation is slow or uncertain. Existing bounded operation and reconciliation owners limit lifetime; no timeout converts uncertainty to clean state.
- Distinct geometries increase evidence fields and mapping work. Collapsing them would make parity, dirty, checksum, and store claims ambiguous, so the extra fields are correctness-bearing rather than a general range framework.
- The pre-canonical-model projection gate and first Connect bridge prevent a second normative transition copy: OpenSpec retains product meaning and owner boundaries, the Connect-ready `CodedRangeClean.qnt` relation owns only the exact delegated projections after synchronization, and correspondence evidence prevents an unobservable model from becoming authority.
