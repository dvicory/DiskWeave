# file-backed-stores Specification

## Purpose
This capability supplies fixed-geometry file-backed stores and disposable control state so the portable engine can run real macOS file-backed arrays without making Linux transports, SQLite layout, or host paths part of core semantics.
## Requirements
### Requirement: File-backed stores expose fixed exact-range semantics
<!-- dwv:req req.file-backed-stores.file-backed-stores-expose-fixed-exact-range-semantics -->

The file store SHALL open a regular or sparse file at a fixed protected length, reject resize/truncate through the store API, and implement exact read, write, write-zeroes, and flush operations using the normalized store contract. Ranges outside geometry, overflow, invalid alignment, and missing buffers SHALL fail before media mutation.

#### Scenario: A sparse hole is read

- **WHEN** a valid read targets an unallocated region of a sparse file
- **THEN** the store returns zero bytes for the exact range and preserves apparent fixed length

#### Scenario: A request exceeds fixed geometry

- **WHEN** a write or read ends beyond the protected length
- **THEN** the store returns a range error without extending or mutating the file

### Requirement: File completion evidence is conservative
<!-- dwv:req req.file-backed-stores.file-completion-evidence-is-conservative -->

The store SHALL distinguish exact success, short, backend failure, uncertain, and duplicate completion. A successful write SHALL report only the evidence established by the requested sync/FUA contract. Host-file synchronization SHALL be named portable-demo evidence until the macOS bridge characterization proves its mapping.

#### Scenario: A write is followed by explicit sync

- **WHEN** the file write and requested synchronization both succeed
- **THEN** completion reports the exact range and the strongest portable evidence allowed by the configured profile

#### Scenario: Synchronization fails

- **WHEN** host synchronization returns an error or the process loses the operation result
- **THEN** the store reports failed or uncertain evidence and does not promote the write to durable

### Requirement: Capabilities and identity observations are probed, not invented
<!-- dwv:req req.file-backed-stores.capabilities-and-identity-observations-are-probed-not-invented -->

The store SHALL expose fixed geometry, alignment, transfer limits, write/flush support, sparse behavior, cancellation limitations, file identity observations, and evidence strength. Unknown physical cache, FUA, discard, and power-loss behavior SHALL remain unknown or unsupported.

#### Scenario: A file is opened on macOS APFS

- **WHEN** metadata and file identity are probed
- **THEN** the capability report includes logical length and file identity while physical durability claims remain gated

#### Scenario: File identity changes

- **WHEN** a path is replaced with a different file identity or geometry
- **THEN** comparison reports changed/ambiguous identity and active writable reuse is refused

### Requirement: Single-writer ownership and endpoint aliasing are explicit
<!-- dwv:req req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->

The store layer SHALL acquire a bounded ephemeral lease before writable use and hold ownership with an operating-system advisory lock on an open descriptor. Marker content MAY remain diagnostic, but marker-file existence SHALL NOT own the lease. A competing live process SHALL fail; process death SHALL release the lease without destructors or cleanup; partial multi-store acquisition failure SHALL release every acquired lease; and reacquisition SHALL revalidate identity, geometry, topology, and recovery authority. The store SHALL reject a backing path that aliases an exported/proxy endpoint while active. Lease loss or conflicting ownership SHALL block writable assembly without modifying payload bytes.

#### Scenario: A second writer opens the same backing store

- **WHEN** the lease already exists
- **THEN** the second open fails deterministically and the existing payload remains untouched

#### Scenario: A writer dies without cleanup

- **WHEN** the process holding a backing-store lease is killed without running destructors
- **THEN** the operating system releases the descriptor lock, and a new process may acquire only after identity and geometry revalidation

#### Scenario: Multi-store claim acquisition fails

- **WHEN** assembly acquires some required leases and a later member or recovery claim fails
- **THEN** every partial lease is released and no writable endpoint or residual ownership marker remains


#### Scenario: Backing and exported paths alias

- **WHEN** assembly resolves the same file identity for a backing payload and exported endpoint
- **THEN** assembly refuses the alias before serving the endpoint

### Requirement: Disposable control SQLite state is separate from recovery authority
<!-- dwv:req req.file-backed-stores.disposable-control-sqlite-state-is-separate-from-recovery-authority -->

The project SHALL provide a versioned control-state projection for inventory/history/job presentation that can be deleted and rebuilt from semantic topology and store observations. Control-state loss SHALL not make direct data unreadable and SHALL not authorize or invalidate recovery-state transitions.

#### Scenario: Control state is deleted

- **WHEN** `control.sqlite3` is removed while direct payloads and recovery authority remain
- **THEN** the system continues safety-critical semantics and rebuilds the projection on demand

#### Scenario: Control state is corrupt

- **WHEN** the projection fails integrity validation
- **THEN** it is quarantined/rebuilt without changing direct data or authoritative recovery state

### Requirement: File-backed evidence remains platform-scoped
<!-- dwv:req req.file-backed-stores.file-backed-evidence-remains-platform-scoped -->

The implementation SHALL identify which behavior is portable file semantics, macOS bridge evidence, Linux-only evidence, or hardware durability evidence. Passing file-store tests SHALL not certify APFS/DiskImages synchronization, physical flush/FUA, or Linux frontend conformance.

#### Scenario: The Linux frontend is unavailable

- **WHEN** the current host runs the file-backed store on macOS
- **THEN** portable and macOS evidence may be recorded while Linux-only criteria remain visibly unmet

### Requirement: Post-acquisition writer-claim release is exact and owner-observed
<!-- dwv:req req.file-backed-stores.post-acquisition-writer-claim-release-is-exact-and-owner-observed -->
<!-- dwv:requires req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->

After a caller has accepted a complete file-backed writer-claim set and later requests release of that exact set, the file-backed owner SHALL return a typed `FileBackedReleaseObservation` bound to the accepted acquisition. For every exact claim, the observation SHALL preserve the opaque claim token, store incarnation, stable backing-resource identity, and alias binding from that acquisition and SHALL classify its ownership disposition as `released`, `held`, or `uncertain`. The claim token SHALL include a file-backed-owner acquisition identity that is unique within one live file-backed owner process and SHALL NOT repeat in that process merely because disposable marker state is removed or recreated. The acquisition identity and every capability containing it SHALL expire at process death and SHALL NOT be interpreted as persisted or cross-process identity.

At complete-set acceptance, the file-backed owner SHALL consume the original writable stores into one opaque, non-duplicable accepted ownership value that retains every exact claim binding and store while exposing store operations only by non-extracting borrow. Each live exact acquisition SHALL enter at most one accepted set; no accepted member store SHALL be independently removed, dropped, replaced, or released outside that owner value. Losing or dropping the accepted ownership value SHALL transfer all still-held exact claims into bounded file-owner quarantine and SHALL NOT permit reissuance or fresh same-process acquisition. A later explicit release SHALL consume the accepted ownership value itself. A replacement, remapped, or reacquired claim with the same store ID SHALL NOT enter that value, substitute for an accepted claim, or obtain `released-all` for the accepted set.

An exact claim SHALL be `released` only when every ownership resource or lock constituting that claim is definitely relinquished. For the current file-backed mechanism, that resource set includes both the backing-payload descriptor lock and the `.dwv-lease` marker descriptor lock. Releasing only part of a claim SHALL classify it as `held` when remaining ownership is established and otherwise as `uncertain`; it SHALL NOT classify the claim as `released`.

The aggregate result SHALL be `released-all` only when every exact claim is `released`; it SHALL be `residual` when one or more exact claims are known `held`; otherwise it SHALL be `uncertain` when one or more exact claims cannot be classified. Partial release, caller abandonment, token or path loss, resource remap, alias change, an operational error, or a lost release result SHALL NOT be promoted to `released-all`.

Ownership disposition and operational cause SHALL remain separate. When a concrete operational failure or lost or unclassifiable observation exists, the owner SHALL preserve that cause without using it as a substitute for the exact ownership disposition.

A consuming release SHALL return a success value only for `released-all`, and consumers SHALL NOT be able to construct that success from a residual or uncertain observation. The accepted ownership value SHALL retain every accepted live store until explicit release or quarantine transfer. For `residual` or `uncertain`, the file-backed owner SHALL return or retain a non-duplicable same-owner ownership value containing every unresolved ownership resource and a lightweight exact-resource process reservation for every original accepted member, including any member already definitely released. Definitely released payload and marker descriptors SHALL close normally and SHALL NOT be retained merely to preserve the reservation. The observation SHALL NOT replace either live ownership value or permit accepted or unresolved descriptors, resources, or reservations to be independently discarded, forgotten, or intentionally leaked.

Accepted ownership, `residual`, or `uncertain` SHALL NOT authorize the file-backed owner to issue a fresh same-process claim for any physical resource in the original accepted set, including a member already definitely released while another member remains unresolved, and observations SHALL NOT be exposed to consumers as reusable release evidence. Dropping the accepted ownership value or cloning an observation and dropping an unresolved capability SHALL NOT make an original member freshly reacquirable in the same live process. Ordinary Drop SHALL transfer every still-held resource and every released-member process reservation into an explicit bounded process-local file-owner quarantine keyed by the exact acquisition, without forgetting or leaking descriptors. Quarantine ownership SHALL continue blocking fresh acquisition from owner state, SHALL be unique for each live claim, and SHALL support same-process owner retry without holding the quarantine lock across release operations. Retry SHALL reuse the existing claim and set records rather than append historical state; live file-writer bookkeeping SHALL remain proportional to currently admitted, accepted, or unresolved claims and SHALL NOT grow with release failures or retries. Aggregate `released-all` SHALL remove the quarantine entry and every original member reservation. Failed or uncertain retry SHALL retain exact unresolved resources and released-member reservations in quarantine. Definitely released resources unnecessary for retry SHALL close normally and SHALL NOT remain open for reservation. A genuinely replaced or remapped different physical identity MAY remain separately owned but SHALL NOT satisfy the old accepted set. A later process after process death MAY acquire only through the existing fresh identity, geometry, topology, and recovery revalidation rules. Process death SHALL retain its existing operating-system crash-release meaning and SHALL NOT retroactively manufacture a successful explicit-release observation.

This requirement applies to post-acquisition release after the complete set has been accepted. It SHALL NOT weaken, replace, or unify with the existing definite partial multi-store acquisition cleanup rule merely to share an API. File-backed release changes only ephemeral file-backed writer ownership and SHALL NOT by itself roll back, close, validate, invalidate, or reinterpret writable-session, protection-epoch, recovery, `CLEAN`, checksum, publication, operation, transaction, or other independently owned semantic state.

#### Scenario: Complete post-acquisition release succeeds

- **WHEN** the caller requests release of one exact accepted complete writer-claim set and the live file-backed owner establishes that every exact claim is no longer held
- **THEN** the owner reports `released-all`, with every member classified `released` and bound to its accepted acquisition identity, and makes no stronger lifecycle, durability, recovery, integrity, protection, or publication claim

#### Scenario: Only part of one claim is relinquished

- **WHEN** explicit release definitely relinquishes one ownership resource of an exact claim but another constituent ownership resource remains held or cannot be classified
- **THEN** the owner reports that claim as `held` or `uncertain` as applicable, preserves the unresolved linear ownership value and any concrete operational cause, and does not report aggregate `released-all`

#### Scenario: Disposable marker state is recreated

- **WHEN** one exact claim is released, its disposable marker state is removed or recreated, and the same live process later acquires the same backing resource again
- **THEN** the later claim has a fresh owner-issued acquisition identity and prior release evidence cannot bind or identify it as the earlier accepted acquisition


#### Scenario: A same-store-ID replacement cannot substitute for accepted ownership

- **WHEN** a replacement, remapped, or reacquired claim has the same store ID but a different exact binding from a claim already retained in an accepted ownership value
- **THEN** the replacement cannot enter or release that accepted value, remains separate owner-tracked ownership, and cannot obtain `released-all` for the original accepted claim

#### Scenario: The same live acquisition is accepted twice

- **WHEN** one or more live exact claims have already entered an accepted ownership value and a caller attempts to accept or transfer any of those claims again
- **THEN** the file-backed owner rejects the second acceptance or transfer without issuing parallel release authority, while the original accepted ownership value remains authoritative

#### Scenario: An accepted ownership value is dropped

- **WHEN** a consumer drops or loses one complete accepted ownership value without requesting explicit release
- **THEN** the file-backed owner transfers every still-held exact claim into bounded quarantine, blocks same-process fresh acquisition, manufactures no `released-all`, and permits later exact owner retry or reconciliation

#### Scenario: An unresolved capability is dropped

- **WHEN** explicit release reports `residual` or `uncertain` and the consumer drops the unresolved ownership capability without successful owner reconciliation or retry
- **THEN** the file-backed owner transfers every unresolved live resource into its bounded exact-acquisition quarantine without forgetting or leaking descriptors, keeps the same live process excluded from fresh acquisition, and manufactures no `released-all` result

#### Scenario: A quarantined claim is retried

- **WHEN** the file-backed owner retries release of one exact quarantined set without holding the quarantine lock across the release operation
- **THEN** aggregate `released-all` removes the quarantine entry and all member reservations and closes or drops released resources normally, while failed or uncertain aggregate release returns exact unresolved resources and released-member reservations to quarantine and preserves same-process exclusion

#### Scenario: Post-acquisition release leaves known residual ownership

- **WHEN** release of one exact accepted complete writer-claim set establishes that at least one exact claim remains held by the live file-backed owner
- **THEN** the owner reports `residual`, preserves every exact `held`, `released`, or `uncertain` member binding, and no consumer treats the set as fully released or freshly reacquirable

#### Scenario: One member releases while its accepted set remains unresolved

- **WHEN** release definitely relinquishes one original member's payload and marker locks but another member of the same accepted set remains `held` or `uncertain`
- **THEN** the released member's descriptors close normally, its lightweight exact-resource process reservation remains with the unresolved set without growing on retry, and fresh same-process acquisition of that physical resource remains blocked until aggregate `released-all` or process death

#### Scenario: Post-acquisition release disposition is uncertain

- **WHEN** interruption, lost observation, backend failure, binding loss, or another unclassifiable result prevents the live file-backed owner from establishing whether one or more exact claims remain held and no exact claim is already known held
- **THEN** the owner reports `uncertain`, preserves every exact unresolved acquisition binding, and no consumer infers release, fresh reacquisition, or reusable claim-bound evidence

#### Scenario: Resource or alias binding changes before release completes

- **WHEN** a backing path is replaced, remapped, or gains a changed alias binding after the accepted acquisition and before the exact release disposition is established
- **THEN** the release observation remains bound to the original acquired claim, incarnation, backing-resource identity, and alias binding, and any disposition that cannot be established for that exact claim remains `uncertain` rather than being transferred to the newly observed resource

#### Scenario: The owner process dies during or before explicit release

- **WHEN** the process holding the exact acquired claims dies before it can return a complete explicit-release observation
- **THEN** the operating system releases its descriptor leases under the existing crash-release rule, no `released-all` observation is retroactively manufactured for the dead process, and a later process may proceed only through fresh acquisition and the existing revalidation rules

#### Scenario: Released claims are reacquired by a later attempt

- **WHEN** a prior exact set has a `released-all` observation or its owner process has died and a later attempt chooses to acquire the stores again
- **THEN** the later attempt performs fresh acquisition and required revalidation and does not reuse the prior claim tokens or claim-bound stabilization, admission, or release evidence as current authority
