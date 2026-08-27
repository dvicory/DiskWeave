## ADDED Requirements

### Requirement: Post-acquisition writer-claim release is exact and owner-observed
<!-- dwv:req req.file-backed-stores.post-acquisition-writer-claim-release-is-exact-and-owner-observed -->
<!-- dwv:requires req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->

After a caller has accepted a complete file-backed writer-claim set and later requests release of that exact set, the file-backed owner SHALL return a typed `FileBackedReleaseObservation` bound to the accepted acquisition. For every exact claim, the observation SHALL preserve the opaque claim token, store incarnation, stable backing-resource identity, and alias binding from that acquisition and SHALL classify its ownership disposition as `released`, `held`, or `uncertain`.

The aggregate result SHALL be `released-all` only when every exact claim is `released`; it SHALL be `residual` when one or more exact claims are known `held`; otherwise it SHALL be `uncertain` when one or more exact claims cannot be classified. Partial release, caller abandonment, token or path loss, resource remap, alias change, an operational error, or a lost release result SHALL NOT be promoted to `released-all`. Operational owner/cause information MAY accompany the observation but SHALL NOT replace the exact ownership disposition.

`residual` or `uncertain` SHALL NOT authorize the file-backed owner to treat an unresolved claim as released or issue a fresh claim for its resource as if the old ownership ended, and SHALL NOT be exposed to consumers as reusable release evidence. The file-backed owner MAY continue release/reconciliation of the same exact unresolved claims while the process remains live, or the process may terminate. A later process after process death MAY acquire only through the existing fresh identity, geometry, topology, and recovery revalidation rules. Process death SHALL retain its existing operating-system crash-release meaning and SHALL NOT retroactively manufacture a successful explicit-release observation.

This requirement applies to post-acquisition release after the complete set has been accepted. It SHALL NOT weaken or replace the existing definite partial multi-store acquisition cleanup rule. File-backed release changes only ephemeral file-backed writer ownership and SHALL NOT by itself roll back, close, validate, invalidate, or reinterpret writable-session, protection-epoch, recovery, `CLEAN`, checksum, publication, operation, transaction, or other independently owned semantic state.

#### Scenario: Complete post-acquisition release succeeds

- **WHEN** the caller requests release of one exact accepted complete writer-claim set and the live file-backed owner establishes that every exact claim is no longer held
- **THEN** the owner reports `released-all`, with every member classified `released` and bound to its accepted acquisition identity, and makes no stronger lifecycle, durability, recovery, integrity, protection, or publication claim

#### Scenario: Post-acquisition release leaves known residual ownership

- **WHEN** release of one exact accepted complete writer-claim set establishes that at least one exact claim remains held by the live file-backed owner
- **THEN** the owner reports `residual`, preserves every exact `held`, `released`, or `uncertain` member binding, and no consumer treats the set as fully released or freshly reacquirable

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
