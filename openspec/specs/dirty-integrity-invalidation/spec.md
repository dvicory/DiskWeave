# dirty-integrity-invalidation Specification

## Purpose
This capability provides the portable dirty-region protocol that atomically invalidates affected integrity evidence before protected home mutation and clears state only with generation-matched fence proof.
## Requirements
### Requirement: Dirty-region coverage is complete and checked
<!-- dwv:req req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked -->

One canonical semantic mapping SHALL derive every dirty region intersected by a member byte range from the recovery-owned region geometry. It SHALL return each region exactly once, preserve the member identity without collisions within declared supported bounds, and reject empty ranges, range arithmetic overflow, region-ID representation overflow, or unsupported geometry before invalidation or home mutation. Service, transaction, fence, checkpoint, simulator, and recovery paths SHALL use this mapping rather than a separate hard-coded region size.

#### Scenario: A write crosses a region boundary

- **WHEN** a valid protected write intersects two or more dirty regions
- **THEN** every intersected region is included once in intent, transaction ranges, fence coverage, and any later matching clear decision

#### Scenario: Region coverage cannot be represented

- **WHEN** the member/range/geometry calculation overflows or two supported inputs would map to the same region identity
- **THEN** the request is rejected before recovery or protected payload mutation

### Requirement: Durable intent precedes protected mutation
<!-- dwv:req req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

The recovery protocol SHALL durably mark every affected region `DIRTY` and every affected `VALID` checksum extent `STALE` before the first protected home mutation. A rejected, lost, or uncertain intent commit SHALL prevent protected home mutation.

#### Scenario: First write crosses clean regions

- **WHEN** a transaction targets a clean region and a valid checksum extent
- **THEN** the protocol commits dirty and stale intent before emitting any protected home read/compute/write action

#### Scenario: Intent commit fails or is uncertain

- **WHEN** the recovery adapter rejects, loses, or cannot classify the intent commit
- **THEN** no protected home mutation is permitted and the transaction returns a conservative failure

### Requirement: Already-dirty writes preserve the invalidation boundary
<!-- dwv:req req.dirty-integrity-invalidation.already-dirty-writes-preserve-the-invalidation-boundary -->

The protocol MAY avoid a redundant durable intent commit only when every affected region is already durably dirty and every affected valid checksum extent is already durably stale under the current topology and generation. Crossing any clean or valid boundary SHALL require new durable intent.

#### Scenario: Write remains within an already dirty/stale set

- **WHEN** a second transaction targets only regions and checksum extents already covered by durable dirty/stale state
- **THEN** it may proceed without another identical intent commit while retaining the captured generations

#### Scenario: Write crosses a clean or valid boundary

- **WHEN** a transaction expands into any region not covered by dirty state or any checksum extent still `VALID`
- **THEN** it commits a new intent before home mutation

### Requirement: Checkpoint and clear require fence evidence
<!-- dwv:req req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence -->

The protocol SHALL clear dirty state or install a clean checkpoint only when all required home and parity writes are terminal, each participating store has durable fence evidence covering its exact accepted write watermark, every region selected for clearing is covered by that evidence, and captured topology, region, checksum, capability, store-incarnation, and recovery generations still match current state. Future, stale, partial, omitted-region, or cross-store evidence SHALL fail closed, and the protocol SHALL clear only the regions proven by the matching fence set.

#### Scenario: All writes have covering fences

- **WHEN** all affected stores provide matching fence evidence and all captured generations remain unchanged
- **THEN** the protocol may durably checkpoint/clear the proven regions

#### Scenario: Fence evidence is missing or generations changed

- **WHEN** a fence is volatile/incomplete or a captured generation no longer matches
- **THEN** checkpoint/clear is refused and dirty or reconciliation-required state remains

### Requirement: Failures and restart are conservative
<!-- dwv:req req.dirty-integrity-invalidation.failures-and-restart-are-conservative -->

Any short, failed, uncertain, abandoned, crashed, or post-intent recovery result SHALL preserve dirty/indeterminate evidence for every affected region. Restart SHALL discover that evidence and SHALL NOT infer clean state from elapsed time, process success, or a missing action result.

#### Scenario: Home write fails after intent

- **WHEN** a data or parity home operation is short, failed, or uncertain after intent is durable
- **THEN** the affected state remains dirty/indeterminate and no clean completion is reported

#### Scenario: Daemon crashes before checkpoint

- **WHEN** process state is lost after a home mutation but before fence/checkpoint
- **THEN** restart enters recovery/blocked handling with dirty evidence rather than assuming the write was clean

### Requirement: Dirty and integrity/session dimensions remain independent
<!-- dwv:req req.dirty-integrity-invalidation.dirty-and-integrity-session-dimensions-remain-independent -->

The protocol SHALL represent dirty-region state, checksum validity/coverage, parity cleanliness, and session-dirty state as separate dimensions. A clean parity state SHALL NOT establish current checksum coverage, and checksum validity SHALL NOT clear dirty state without the required transaction proof.

#### Scenario: Parity is clean but checksums are stale

- **WHEN** parity state is clean while some checksum extents are stale or absent
- **THEN** the system reports clean parity with incomplete integrity coverage and does not claim all checksums valid

#### Scenario: Session close cannot be proven

- **WHEN** a session fence or recovery checkpoint is missing at shutdown
- **THEN** the session remains dirty even if individual regions were previously checkpointed

### Requirement: Transitions and evidence are deterministic
<!-- dwv:req req.dirty-integrity-invalidation.transitions-and-evidence-are-deterministic -->

The protocol SHALL emit stable semantic transitions containing transaction identity, topology epoch, affected ranges/extents, captured/current generations, and missing evidence. Replaying the same plan and results SHALL produce the same terminal state and error class.

#### Scenario: Same schedule is replayed

- **WHEN** the same intent, home, fence, checkpoint, and failure results are applied twice
- **THEN** the normalized transition trace and terminal recovery state are identical

#### Scenario: Invalid ordering is attempted

- **WHEN** a caller requests home mutation before intent or clear before a covering fence
- **THEN** the transition is rejected without changing the prior semantic state

