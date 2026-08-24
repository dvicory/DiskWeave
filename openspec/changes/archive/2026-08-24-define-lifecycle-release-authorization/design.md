## Context

See `proposal.md` for the missing service-composition boundary. Current requirements already expose the contributing decisions separately: transaction traces include the semantic release boundary, operation slots retain generation-qualified child and terminal evidence until safe reclamation, recovery owns generation-checked durable observations, and healthy-portable-io composes operation outcomes. No current requirement defines the composed semantic authorization needed by a later consumer.

The implementation follow-up must remain after the accepted `.6` lifecycle work and before any coded-sync rebase or coded-clean implementation. This change defines product meaning only; it does not choose a Rust representation or a Connect interface.

## Goals / Non-Goals

**Goals:**

- Give the existing healthy service one clear composition boundary for `ReleaseAllowed(operation-generation)` only when the admitted operation carries an owner-approved releasable claim or obligation that another component must be permitted to drop.
- Make the authorization generation-qualified, monotonic, irrevocable after establishment, repeat-observable, and independent of event delivery.
- Require exact terminal/reconciliation, transaction/recovery release, safe `Reclaimable`, and an explicit healthy basis-conformance observation before authorization.
- Separate semantic authorization from later physical resource cleanup while preserving exact retry ownership after cleanup failure.
- Keep each contributing requirement as the authority for its own predicate and leave coded-clean with an observable owner result to consume later. Routine reads, flushes, and other operations without a releasable claim are outside this authorization relation even when their slots terminate.

**Non-Goals:**

- Defining transaction `releaseRange`, recovery commit semantics, child completion vocabulary, operation-slot reclamation mechanics, or basis-read policy.
- Choosing a result type, callback, queue, persistence format, or Connect bridge.
- Adding coded-range or recovery-CLEAN semantics.
- Modifying `RecoveryProtocol.qnt`, `CodedRangeClean.qnt`, canonical requirements, or product Rust in this planning change.
- Defining startup, shutdown, currentization, publication, or scan-independent behavior.

## Decisions

### 1. Add one narrow requirement under the existing healthy service-composition capability

`ReleaseAllowed` is a narrow service-composition fact for an admitted operation whose owner has declared a releasable claim or obligation. It is not a generic operation-completion result: routine reads, flushes, and other operations without that obligation SHALL NOT establish it merely because their child operations terminate or their slots become reclaimable. Applicability is an admission/owner fact and is not a derived eighth model observation.

The requirement SHALL name the seven owner inputs explicitly:

- exact operation/media-effect terminal or authoritative reconciliation evidence from the existing store-operation and recovery boundaries;
- terminal child outcomes from `store-operation-contracts`;
- the required child reconciliation record;
- the exact generation-qualified safe `Reclaimable` state from `store-operation-contracts`;
- the applicable owner-approved semantic transaction release requirement from `explicit-transaction-machine`, including delegated `RecoveryProtocol.releaseRange` where applicable;
- any additional authoritative reconciliation observation required from `recovery-state-semantics`;
- healthy-service conformance that no relevant basis observation remains consumable because it was consumed, discarded, or authoritatively reconciled.

Healthy-portable-io establishes the explicit basis-conformance observation and composes these owner facts into `ReleaseAllowed`. It does not infer any owner's fact from another, and it does not consult dirty-integrity CLEAN-capture state.

### 2. Define authorization before physical cleanup, but after safe Reclaimable

`ReleaseAllowed(operation-generation)` becomes true only after the operation-slot owner has reached safe `Reclaimable` state and every other owner predicate holds. The physical release of slot/resource counters is a later cleanup action, not the semantic authorization itself.

If cleanup fails, the operation-slot owner retains the exact generation, canonical request, evidence, and resources for retry. The established authorization remains valid for that generation, while generation reuse remains blocked by retained ownership. Later cleanup success, topology/recovery-generation changes, unrelated work, or other observations do not revoke the authorization; they may end physical resource retention after the consumer no longer needs it, but they do not make the established fact false.

The implementation must therefore expose the authorization at the service composition boundary before or independently of the fallible physical removal step. It must not derive authorization from a successful `release()` call alone.

### 3. Make the fact monotonic and observation-based

The contract defines a fact, not a required exactly-once event. A consumer may observe `ReleaseAllowed` more than once for the same operation generation without changing state. The fact is keyed by the complete generation-bearing operation identity; it cannot match a later reservation at the same slot index.

A failed or unresolved operation does not produce the fact. A known failed/short operation may produce it only after its exact terminal evidence and required reconciliation satisfy the same owner predicates. An uncertain operation remains withheld until the service receives authoritative reconciliation for that exact generation. Once established, the fact has no normal invalidation or retirement transition; a wrong owner input is an upstream correctness failure.

### 4. Keep existing release observations subordinate

`RangeReleased` is necessary where the semantic transaction relation requires it, but it is not sufficient: it may precede outer operation terminalization and healthy basis conformance. Physical slot removal is also not sufficient: it proves resource cleanup, not transaction release, recovery reconciliation, or basis authorization. The service must compose all predicates rather than nominate either observation as a proxy.

### 5. Keep basis conformance at the healthy seam

The service-composition requirement owns the explicit typed observation that no relevant basis observation remains consumable. Transaction, store, and recovery owners continue to supply their observations and lifecycle boundaries. Recovery/topology/checksum-generation coherence is a separate fact and SHALL NOT masquerade as basis lifetime. No separate basis-lifecycle state machine is introduced, and the future consumer must receive the resulting fact rather than recreate basis lifetime policy.

### 6. Bound retention and evidence

Authorization is keyed to one operation generation and does not require an unbounded historical authorization log. Until physical cleanup succeeds, the existing bounded slot evidence and resource ownership remain authoritative. A cleanup request is a bounded pending observation; an observed failure preserves the exact authorization for retry, while an observed success may release owned resources but cannot revoke the already-established authorization. Generation-safe lookup prevents later reuse from consuming the old identity.

### 7. Delegate only the bounded composition relation

The exact finite state, action, and invariant relation for this healthy-service composition SHALL be delegated to the canonical `models/quint/LifecycleRelease.qnt` module. Its seven external owner observations are: (1) terminal or authoritative-reconciled operation/media effect, (2) terminal children, (3) recorded required reconciliation, (4) safe generation-qualified `Reclaimable`, (5) the applicable semantic transaction release boundary, (6) authoritative recovery-owned reconciliation, and (7) healthy-service basis conformance showing no relevant basis remains consumable. Applicability is an external admission scope condition, not an eighth observation. The model separates these authoritative observations from pure derived claims, the `ReleaseAllowed` authorization decision, cleanup action requests, and observed cleanup results.

The transaction input is the owner-approved release requirement being satisfied, not raw `RangeReleased` and not a Rust `transaction_required` boolean. Observation acquisition, payload I/O, execution mechanics (delay, queueing, batching, concurrency, reordering, retry, backpressure), owner predicate derivation, transaction/recovery/store/operation-slot policy, and dirty-integrity `CLEAN` capture remain external. Execution mechanics SHALL NOT create, extend, substitute, revoke, or silently preserve semantic authority. Issuing a cleanup request SHALL NOT imply external cleanup success; late or out-of-order results must correlate to the exact requested operation generation and cannot authorize a newer incompatible generation. Separate `verification/quint/LifecycleReleaseAnalysis.qnt` and mutant profiles SHALL bind finite evidence only; they SHALL NOT add product semantics or use `OperationSlotLifecycle` grammar.

## Risks / Trade-offs

- Authorization can remain semantically true while physical cleanup is retryable. This is deliberate: cleanup failure is not silently converted into a media-effect failure, but retained ownership still prevents reuse.
- A repeated observation is less event-like than a one-shot callback, but it avoids lost authorization caused by delivery failure and makes generation correlation explicit.
- The healthy service gains a consequential composition boundary. Keeping contributing predicates outside it prevents duplicate authority but requires later implementation evidence to bind every input.
- The contract intentionally leaves result representation and Connect observation mechanics to implementation. Those choices must preserve the generation key and all owner boundaries without changing this semantic meaning.
