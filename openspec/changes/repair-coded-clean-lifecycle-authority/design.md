## Context

See `proposal.md` for motivation. The canonical dirty-integrity requirement delegates the exact bounded capture lifecycle to `models/quint/CodedRangeClean.qnt`. The model currently ends at operation release and capture resolution; production also exposes deserializable transition DTOs and a test bridge that can construct release authorization directly. On reopen, persisted capture membership can name operation generations that no longer have a live lifecycle owner.

## Goals / Non-Goals

**Goals:**

- Give the delegated model exact, separate successfully linearized full-lifecycle coded release, owner-authorized membership-compaction, and phase-authorized capture-cleanup transitions.
- Keep one coded-geometry owner and make persisted facts, live proof revalidation, capability bindings, and test isolation explicit.
- Provide bounded steady-state retirement and capture-specific authoritative resolution after failure, unknown acknowledgement, and restart.
- Preserve dirty state, exact predecessor/revision bindings, generation checking, atomicity, and conservative unknown outcomes.

**Non-Goals:**

- Reconstruct or persist a general operation ledger across restart.
- Make capture cleanup clear dirty regions, validate checksums, or publish service.
- Change coded-range geometry, CLEAN-owner policy, recovery-state unknown-commit ownership, or physical capture-capacity limits.
- Add compatibility shims for old serialized transition DTOs; they are capabilities, not persistent format.

## Decisions

### One coded-geometry owner supplies claims and capture scope

The canonical coded-range owner owns one pure mapping boundary over the admitted topology/coding profile. It supplies both complete per-write coded claims and the complete future-inclusive coded scope for selected dirty regions and checksum extents. Dirty-integrity consumes those outputs and never reconstructs member-to-parity/codeword mapping.

The production mapping API has two owner operations over the same internal mapping: map a member byte range to its complete coded claim, and map selected dirty/checksum geometry to every coded unit whose mutation could invalidate that selection. Verification compares both operations with an independent brute-force oracle over real geometry: 512-byte coded units, 4-KiB dirty regions, 4-MiB checksum extents, exact boundaries, shared-region/shared-extent cases, and genuinely disjoint cases.

### Persisted state is data; owners reissue live authority

Durable capture data retains capture identity, array/topology/coding identity, exact selected dirty/checksum geometry, exact coded units, capture and lower-frontier summaries, membership and pending/resolved history, durable evidence references/receipts, and exact snapshot/revision identity. Serialized convenience flags such as `scope_complete`, `scope_validated`, or `lower_frontier_covered` may describe the written snapshot but do not recreate proof.

After reopen, the coded-range owner recomputes and validates the scope against the durable topology/coding identity and selected geometry. The dirty-integrity capture-retention owner revalidates membership, lower-frontier and retained-history coverage against durable release/CLEAN/later-cut evidence. Recovery-state owners revalidate generation, topology, predecessor revision, and commit observations. Missing, stale, contradictory, or unavailable underlying facts withhold fresh authority and leave the capture conservative and non-cleanable.

### Production and test authority remain opaque

`CodedCaptureUpdate`, `CodedCaptureRemoval`, lifecycle release authorization, retention authorization, and phase-specific cleanup authorization expose only the minimum exact bindings required by their consumers. They are not deserializable or reconstructible from a snapshot, phase check, expected-state equality, or ordinary values.

A normal Cargo feature, including `test-support`, must not expose a production authority constructor, raw state transition, or all-positive owner-observation builder when feature unification occurs. Tests exercise real owner producers or a deliberately isolated adapter that cannot link into or issue authority for a production build.

### Final coded release consumes full canonical `ReleaseAllowed`

Successful coded admission records that the exact operation generation is in release scope. Final coded-claim removal accepts only the monotonic `ReleaseAllowed(operation-generation)` issued by the canonical healthy-portable-io lifecycle owner after all seven owner-approved observations hold.

Safe slot `Reclaimable`, `OperationReleasePermit`, or another lower-level reclaimable-slot witness is only one prerequisite. If retained internally, it is private to lifecycle composition and is named and typed as a prerequisite rather than final release authority. Coded authority removal cannot accept or derive from it. The lifecycle-owner seam exclusively obtains trusted observations and issues `ReleaseAllowed`; coded-CLEAN consumers cannot call a raw issuer or fabricate an all-positive observation bundle.

The successful production linearization removes coded authority and durably retains the exact release receipt required by every affected capture. Only then does the delegated model observe `Released`. Release neither cleans or reuses the operation slot nor compacts membership nor retires a capture.

### The dirty-integrity retention owner authorizes compaction

The dirty-integrity capture-retention owner is the only owner able to prove that old capture membership/history may be forgotten. It evaluates the exact durable predecessor and all retained or discarded lifecycle/CLEAN/later-cut obligations, then issues an opaque preparation bound to:

- capture, array/topology, and coding identity;
- exact durable predecessor snapshot or revision;
- exact released operation generation or generations and every required durable release receipt;
- complete pre-compaction membership and pending/resolved history obligations;
- every fact proposed to be forgotten and every fact that remains retained;
- exact replacement bounded summary/frontier and proposed successor.

The service carries this preparation into one generation-checked recovery transaction and confirms it only from the exact durable commit receipt. It never invents the summary from current recovery generation, coded-admission high-water mark, one released sequence, or other local arithmetic. Known rejection preserves the exact prior state. Unknown acknowledgement discards process-local belief and follows recovery-state reopen reconciliation. Stale, cross-capture, cross-topology, cross-revision, cross-generation, missing-receipt, or incomplete-history bindings fail without mutation.

Physical slot cleanup remains separate. Failure after release but before compaction retains the release receipt and membership needed to retry safely.

### The dirty-integrity capture-lifecycle owner issues phase-specific cleanup

There is no generic production `retire_resolved_capture(capture_id)` authority that inspects phase and grants itself deletion. The dirty-integrity capture-lifecycle owner consumes phase-specific evidence and issues a non-interchangeable capability bound to capture identity, exact durable predecessor snapshot/revision, current topology, current recovery generation, exact selected dirty/checksum geometry, and exact proposed successor:

- For a newly refused inherited capture, `RecoveryCleanRefusalPermit` remains CLEAN-policy evidence. The lifecycle owner combines it with the exact capture binding to derive one atomic refusal-and-removal proposal.
- For a persisted `Refused` capture, persisted phase is data. Current recovery and dirty-integrity owners revalidate the refusal, conservative dirty consequence, ended future-exclusion duty, and all exact bindings before cleanup authority is issued.
- For `CleanKnown`, the lifecycle owner consumes a durable-clean retirement certificate described below. Refused cleanup authority and clean cleanup authority are distinct types.

Two captures with identical selected geometry cannot consume the same cleanup capability because capture identity and exact predecessor revision are mandatory bindings. A refused capture may retire with stale prior-process membership because dirty/indeterminate state remains and no release authority is synthesized.

### Empty `CleanKnown` captures retire without unrelated mutations

Strictly newer dirty/recovery supersession is sufficient but not necessary for `CleanKnown` retirement. It does not bound a last write to a quiescent region and would let disjoint sequential writes accumulate one capture each.

The capture-lifecycle owner may instead issue exact `CleanKnown` cleanup authority when:

- the capture-wide accepted decision and recovery `CLEAN` commit are durably known;
- the exact durable recovery successor independently retains the CLEAN/protection result and evidence required for future claims;
- every included or later operation has a durable resolved disposition and required release receipt;
- owner-authorized compaction has removed all membership;
- no pending or unknown later cut, CLEAN outcome, or retained-history obligation remains; and
- the exact bounded retained-history summary covers every forgotten fact and is bound to the live predecessor revision.

The CLEAN transaction alone does not authorize immediate deletion, but no genuinely later write or dirty boundary is required after these closure facts become durable. Under continued healthy owner execution, each ordinary successful write can therefore progress through full release, compaction, and cleanup even if its region is never touched again. Sequential disjoint writes do not monotonically increase retained captures; count is bounded by concurrently unresolved lifecycle work, while stalled/faulted cleanup triggers ordinary finite-capacity backpressure rather than silent forgetting or false `CLEAN`.

### Reopened refusal and removal use exact atomic proposals

While recovering and before live admission, the service accepts only capture-specific cleanup authority derived from current refusal evidence and bound to the exact inherited predecessor. One generation-checked recovery transaction proposes refusal and removal atomically.

A known rejection or failure leaves the exact prior durable and in-memory state authoritative. After a true may-have-committed unknown, lost, or corrupt acknowledgement, the service installs neither candidate, releases all process-local authority, and reopens or inspects the durable artifact under recovery-state semantics. Reopen may accept only the exact prior inherited capture or exact atomically refused-and-removed successor. Both preserve conservative dirty/indeterminate state and neither claims `CLEAN`, operation release, or valid integrity.

### The delegated model names external owner capabilities

The model has separate successful `removeCodedClaim`, `compactReleasedMembership`, and `cleanupResolvedCapture` transitions. `Released` abstracts only the full production release linearization. Model compaction authorization abstracts the exact owner-issued retention preparation and durable confirmation. Distinct model refused/clean cleanup authorizations abstract the corresponding capture-specific opaque production capabilities; capture phase alone never enables cleanup.

Quint remains at the semantic level and does not reproduce recovery-store mechanics, durable revisions, geometry arithmetic, or adapter uncertainty. Connect maps each named authorization to its real owner producer and projects observable operation phase, capture phase, and membership. Recovery-state semantics and Rust reopen tests retain exclusive responsibility for known failure versus may-have-committed unknown outcomes.

## Risks / Trade-offs

- Clean retirement without a newer dirty boundary is safe only because the exact durable CLEAN result remains independently represented and every capture-only obligation is durably closed and summarized before removal. The certificate must fail closed if any fact cannot be revalidated.
- Recording inherited capture revisions in service memory is only a process-local candidate binding. It confers no cleanup authority and is discarded after unknown persistence outcome or unrelated generation/topology change.
- Opaque retention and retirement capabilities add types and explicit owner seams, but avoid a general operation ledger or effect framework and prevent service arithmetic from becoming shadow authority.
- Healthy progress bounds settled captures; it does not make failure disappear. If an owner or store cannot complete closure, finite capacity may backpressure admission while preserving evidence.
