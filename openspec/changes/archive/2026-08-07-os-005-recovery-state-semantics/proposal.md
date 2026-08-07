## Why

The simulator can now distinguish durable media from volatile and uncertain effects, but there is no portable recovery-state authority that records dirty intent, stale integrity evidence, fences, topology epochs, and clean checkpoints. Without that boundary, later transaction work could accidentally treat a database commit or successful I/O call as proof that independent home media is durable.

## What Changes

- Define a portable `RecoveryStateStore` semantic interface and generation-checked transactions.
- Define dirty-region, integrity-generation, store-fence, writable-session, topology, and maintenance-checkpoint mutations.
- Add an in-memory reference store and semantic export suitable for simulator and transaction tests.
- Establish a replaceable SQLite evaluation seam without selecting journal mode, synchronization, schema, or a stable database format.
- Add a versioned semantic schema/migration/export representation, deterministic candidate fixtures, and conservative fault dispositions for simulator and future SQLite adapters.
- Add a separate evaluation-only `dwv-recovery-sqlite` prototype that exercises the checked-in migration and candidate settings through the host `sqlite3` executable without making SQLite types part of the portable API.

## Capabilities

### New Capabilities

- `recovery-state-semantics`: durable recovery-state transactions, evidence-gated clean/integrity transitions, and topology snapshots.

### Modified Capabilities

None. The change consumes OS-004 fault schedules and OS-002 persistence evidence.

## Impact

- Adds a portable recovery package with no SQLite, filesystem, runtime, or frontend type in its semantic API.
- Provides the authority boundary used by later transaction, topology, parity-envelope, and file-store work.
- Does not choose a production SQLite configuration or claim recovery-state durability on a particular host path.
- The SQLite prototype is an executable evaluation fixture only; it does not select a production binding, journal mode, synchronization setting, schema, or hardware durability profile.
