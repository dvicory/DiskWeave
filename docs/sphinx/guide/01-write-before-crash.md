# A write before the crash

Start with four bytes. The selected data member currently contains:

```text
offset 0: 00 00 00 00
```

The caller asks DiskWeave to replace them with:

```text
offset 0: 01 02 03 04
```

A protected write changes more than one file. DiskWeave must update the selected
data member, update parity from the old and new bytes, and preserve enough recovery
evidence to handle a crash between those actions.

## The safe order

For a previously clean range, the protocol is:

1. Commit the write-recovery record: the range is `DIRTY`, and any checksum that
   described the old bytes is `STALE`.
2. Only after that record is durable, read the old data and parity needed for the
   update.
3. Write the new data and corresponding parity.
4. Observe persistence evidence for the affected stores.
5. Commit recovery state `CLEAN` and the resulting integrity state when each
   owner's evidence predicate permits it.

If step 1 fails or its outcome is unknown, DiskWeave is not allowed to start step
3. This order prevents a changed payload from remaining falsely labelled clean
after restart.

## Completion is not durability

The simulator fixture `scenario.basic-write` deliberately stops between
completion and persistence evidence:

```text
initial durable bytes       00 00 00 00
write becomes visible       01 02 03 04
completion is delivered     yes
persistence evidence exists no
machine crashes             now
```

At that cut point:

- the caller knows that completion was delivered at the operation's declared
  completion scope;
- the running process may observe the new bytes;
- the durable-media model still contains the old zero bytes;
- recovery must preserve uncertainty rather than claim that the write certainly
  survived or certainly rolled back.

The distinction matters even when a caller stops waiting. Dropping a future,
cancelling a request, or losing a response does not stop an already-issued device
operation and does not prove rollback.

## What a flush adds

A successful flush supplies persistence evidence at the declared scope for the
affected stores. It is stronger than write completion, but its real-world
strength is still bounded by the store, operating system, filesystem, and
hardware. The current file-backed demo reports host-file persistence evidence;
it does not certify behavior during physical power loss.

**Next:** {doc}`02-recovery-uncertainty` follows the same range through restart.

## Traceable requirements

```{needlist}
:filter: "type == 'req' and capability in ['volatile-media-simulator', 'dirty-integrity-invalidation', 'explicit-transaction-machine']"
```

**Provenance:** `req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred`; `req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write`; `req.dirty-integrity-invalidation.failures-and-restart-are-conservative`; `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent`; `req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects`; scenario `scenario.basic-write`.

