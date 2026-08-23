## MODIFIED Requirements

### Requirement: Assembly and shutdown preserve ownership and recovery authority
<!-- dwv:req req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority -->
<!-- dwv:requires req.healthy-portable-io.portable-shutdown-preserves-operation-ownership-and-claim-release-ordering -->

Before publication, the Linux workflow SHALL validate fixture ownership, stable opened-file identities, fixed geometry, topology epoch, recovery-state access, required roles, capability compatibility, and backing/export non-aliasing, and SHALL acquire every writable store and recovery claim with crash-releasing operating-system descriptor locks. Marker existence SHALL not own a claim. Any failure SHALL release partial claims and leave no writable endpoint. Reacquisition after process death SHALL revalidate current store and recovery authority.

For a published service, the Linux workflow SHALL conform to the portable shutdown requirement for admission closure; frontend quiescence; exact-generation safe `Reclaimable` with every child terminal and required reconciliation recorded; terminal or authoritatively reconciled applicable operation/media effects; owner-approved recovery-`CLEAN` and durable session-close evidence that consumes rather than manufactures those facts; endpoint withdrawal; and independent claim-release ordering. Its platform refinement SHALL remove only the owned ublk endpoint, preserve the portable conservative failure or reconciliation-required result when any owner predicate or cleanup is incomplete, and keep OS descriptor claims tied to endpoint non-aliasing. Unmount success SHALL NOT mask a later owner-predicate or owned-endpoint removal failure.

#### Scenario: A backing file is replaced or resized

- **WHEN** current opened identity or geometry differs from the fixture's recorded assignment
- **THEN** assembly fails before publication or recovery/payload mutation

#### Scenario: Backing and export identities alias

- **WHEN** a backing payload and proposed exported endpoint resolve to the same underlying object or permit a direct active backing bypass
- **THEN** assembly fails before publication

#### Scenario: Clean shutdown completes

- **WHEN** consumers have unmounted, the portable shutdown contract's exact operation-slot, operation/media-effect, recovery-`CLEAN`, session-close, endpoint-withdrawal, and independent claim-release prerequisites succeed, and the owned ublk endpoint is removed
- **THEN** the service closes all backing/recovery handles and reports the portable bounded clean-shutdown result

#### Scenario: Shutdown cannot complete after unmount

- **WHEN** consumers have unmounted but an operation generation is not safely `Reclaimable`, a child is non-terminal, required reconciliation is unrecorded, an applicable operation/media effect is unresolved, persistence or recovery-`CLEAN` evidence is unavailable, durable session-close evidence is unavailable, an independent claim-release owner withholds release, or owned-endpoint removal fails
- **THEN** shutdown reports failure or reconciliation-required state and does not report a clean recovery state or clean close

#### Scenario: Stale cleanup is interrupted

- **WHEN** startup or cleanup observes an owned stale ublk endpoint
- **THEN** competing publication is refused and only an explicit bounded cleanup operation for the owned disposable fixture may remove it

#### Scenario: The owner process dies

- **WHEN** the frontend process is killed without running shutdown or destructors
- **THEN** OS descriptor claims are released, but dirty or indeterminate recovery state and any stale endpoint remain explicit inputs to conservative reacquisition and no clean close is claimed

#### Scenario: Recovery authority cannot be loaded

- **WHEN** recovery snapshot access fails or returns missing, corrupt, stale, or mismatched authority
- **THEN** assembly fails without substituting generation zero, publishing an endpoint, or mutating protected payloads
