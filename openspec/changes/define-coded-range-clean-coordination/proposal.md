## Why

The current write, transaction, dirty-region, recovery, topology, and store requirements require range acquisition, parity-correct orchestration, generation checks, and evidence-bound recovery `CLEAN`, but they do not close the conflict domain across different members that map to one coded parity/codeword range. They also do not close the admission race in which recovery `CLEAN` clears a range while an overlapping mutation is admitted, so two conforming implementations could make different safety decisions; the retained v0.8 §11.6–§11.7 and v0.9 §8.6–§9.11 targets make both decisions consequential now.

## What Changes

- Add one transaction-owned semantic contract for coded parity/codeword range authority: a validated semantic mutation unit (which MAY be a request decomposition unit) must carry a complete mapped coded claim before dependent basis I/O or mutation; operations on different members conflict when their mapped claims overlap under the captured topology and profile.
- Define coded authority admission and retention without choosing a mechanism: partial physical acquisition is pre-admission and authorizes no I/O; scope-local classification is exact and does not require a global lock, counter, or connected-component interpretation.
- Define coded-claim removal as consumption of exact external `ReleaseAllowed(operation-generation)` supplied by the canonical healthy-portable-io lifecycle-release requirement. The coded owner treats that authorization as opaque with respect to its contributing terminality, child, transaction, recovery, basis, and cleanup predicates; independent CLEAN-capture uncertainty neither supplies nor negates it, and capture cleanup remains a distinct dirty-state decision.
- Define the authority lifecycle for mutations and relevant basis reads: captured topology/profile and distinct geometries, complete multi-range admission, and correlated basis observations that remain coherent under complete coded authority through a consuming mutation or are discarded/reconciled before release, with bounded refusal, stale-generation/topology rejection, and conservative failure/restart handling.
- Enumerate exact owner-approved observations for durable dirty/recovery intent, lifecycle disposition/handoff/uncertainty/reconciliation/release facts, `CLEAN` `Durable`/`Rejected`/`Unknown` outcomes, and reopen reconciliation. The CLEAN owner supplies one explicit capture-specific `Accepted` or `Rejected` decision; the delegated relation consumes it without deriving dirty/checksum, lifecycle, persistence, or release policy. `Unknown` cannot become `CLEAN` without authoritative recovery reconciliation, but the capture independently retains its bounded identity/frontier evidence and exclusion obligation rather than automatically pinning an otherwise releasable operation.
- Compose the new owners with existing healthy-write, transaction-trace, recovery-transaction, persistence-evidence, store-watermark, operation-slot, topology, dirty-region, checksum, and XOR requirements without restating their predicates.
- Keep request, coded/store, dirty-region, and checksum geometries distinct; no lock, sharding, runtime queue, production P/Q, or generic concurrency framework is selected.
- Define deterministic evidence scenarios for same-codeword conflict, disjoint coded ranges, complete-scope admission, pre-admission refusal, held correlated basis reads, stale captures, bounded refusal, abandonment/release authorization, uncertain failure, restart/reopen reconciliation, closed-set inclusion, durable post-capture cuts, later dirty mutation, and clean-commit rejection.
- Delegate only the bounded relation needed for coded conflict, capture-side membership, consumption of externally supplied capture-specific `Accepted`/`Rejected` decisions, and CLEAN/removal eligibility from externally supplied owner facts to a new `models/quint/CodedRangeClean.qnt` module after the approved focused semantic and projection-feasibility review. The Connect-ready model pass SHALL refine the projection map against actual declarations without introducing an unmapped distinction; product Rust/Connect implementation remains outside this apply slice.
- Maintain a refined projection-feasibility map for every consequential delegated action/state distinction: identify an observable future Connect correspondence or explicitly named external owner and non-delegated evidence. Healthy-service basis coherence remains external to `CodedRangeClean`, and the canonical lifecycle-release owner supplies exact `ReleaseAllowed` for coded removal; the delegated relation manufactures neither fact. No full Connect driver may precede an implementation seam, and the first Rust pass SHALL bind the refined map to the actual Connect bridge before broader implementation.
- Keep the two OpenSpec owners separate while delegating their distinct coded-conflict and CLEAN-capture projections to the same small model; lifecycle owners supply disposition facts, the CLEAN owner supplies explicit capture-specific `Accepted`/`Rejected` decisions, and the relation does not own lifecycle terminality, CLEAN decision policy, persistence/topology truth, geometry construction, basis-read correctness, or `RecoveryProtocol.releaseRange`.

## Capabilities

### New Capabilities

None. These are two independently owned semantic additions to existing capabilities, not a new umbrella capability.

### Modified Capabilities

- `explicit-transaction-machine`: owns coded parity/codeword authority admission/conflict and consumes external owner-approved release authorization without deciding terminality.
- `dirty-integrity-invalidation`: owns the closed mutation set for recovery `CLEAN`, including its durable post-capture cut and bounded membership evidence.
- `healthy-portable-io`: its modified protected-write and durable-completion requirements consume the coded/CLEAN owners, while its canonical lifecycle-release requirement remains the sole final owner of exact `ReleaseAllowed`.

### Composing Capabilities (unchanged owners)

- `recovery-state-semantics`: continues to own atomic generation/topology checks, persistence-evidence admissibility, and conservative commit observations.
- `store-operation-contracts`: continues to own exact store ranges, watermarks, bounded operation slots, resource lifetime, and backend uncertainty.
- `anchorless-topology-identity`: continues to own stable assignments, captured immutable topology snapshots, and stale/ambiguous topology rejection.
- `dirty-integrity-invalidation`'s existing region/checksum mapping remains the dirty geometry owner; the new closed-set requirement does not replace it.
- `xor-reference-model` and checksum requirements continue to own protected parity geometry, parity bytes, and checksum extents.

## Selection and Independence

The selected planning boundary is the pair of retained semantic rows **Global coded-range coordination across different member operations** and **Closed-mutation-set recovery-CLEAN concurrency**. The first decision has one owner in `explicit-transaction-machine`; the second has one owner in `dirty-integrity-invalidation`. They compose, but neither is an umbrella replacement for the other.

The dependency structure is explicit: the coded-range owner consumes the canonical lifecycle-release owner's exact `ReleaseAllowed` only for coded-claim removal; the closed-set owner consumes externally validated coded-scope classification; and the modified healthy-portable-io requirements consume the coded/CLEAN owners. Existing lower-level transaction, operation-slot, recovery, persistence-evidence, store-watermark, topology, dirty-region, checksum, and parity owners remain authoritative and are referenced, not copied. This change has no dependency on writable-session begin/close, startup, shutdown, or publication semantics.

Return paths are part of the target contract. Coded-range admission returns complete-scope authority, a bounded conflict/exhaustion refusal before dependent I/O, a stale topology/generation refusal, or retained authority pending exact external `ReleaseAllowed`. Recovery `CLEAN` returns a committed exact clear only after its durable cut and owner evidence, a deferred/dirty result for a later mutation, a stale or evidence-refused result with no clear, or reconciliation-required state after an uncertain outcome. No path treats an overlapping or `Unknown` mutation as clean or releasable. An unresolved `CLEAN` capture remains unable to authorize `CLEAN` and independently guards future overlapping effects, but it does not itself negate `ReleaseAllowed` supplied by the canonical lifecycle-release owner.

Rejected adjacent scope:

- scan-independent epoch admission, fresh-basis currentization, background rollover, prior-claim retention, and Linux-specific behavior;
- writable-session begin/close, portable shutdown, deployment ordering, and startup publication;
- lock selection, lock quantum as a mechanism, sharding, queues, scheduling fairness, production P/Q, and a generic concurrency framework;
- parity construction, checksum algorithms, store durability policy, dirty-region mapping, transaction state vocabulary, and recovery persistence predicates already owned elsewhere;
- product Rust/Connect implementation, further lifecycle-release redesign or duplication, canonical-spec synchronization, Beads, and maintained documentation unrelated to the model/evidence record.

## Impact

This apply slice adds the approved bounded canonical relation, finite analysis/replay/mutant artifacts, and the necessary delegation/projection/task/evidence updates under this change directory. It does not alter current canonical coded/CLEAN requirements until the normal OpenSpec sync path. The focused model/projection review and independent lifecycle prerequisite lane are complete; exact lifecycle release is now canonical and Connect-backed. Actual coded-clean Rust integration is next gated on the coded/CLEAN Connect bridge and later conformance/adversarial verification. This change imposes no order on unrelated startup, shutdown, or publication work. Implementation must preserve existing owner boundaries, report exact request/store/dirty/checksum geometry separately, and expose deterministic bounded evidence without claiming a backend mechanism or physical durability tier.
