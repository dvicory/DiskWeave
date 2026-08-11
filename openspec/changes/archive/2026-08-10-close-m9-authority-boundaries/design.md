## Context

See `proposal.md`. The affected boundaries already have canonical semantic owners; remediation must compose them without moving policy into the CLI or frontend. The supported frontend remains the narrow one-data/one-parity Linux profile. Persistent formats are not stable yet.

## Goals / Non-Goals

**Goals:**

- Make every authority-bearing transition consume evidence supplied by its semantic owner.
- Make uncertain SQLite commit outcomes reconcilable after process-local state is discarded.
- Preserve one production result shape after argument parsing.

**Non-Goals:**

- Implement new-lineage creation, parity rebuild, or a certificate/receipt authority.
- Broaden frontend profiles or add read-only publication.
- Preserve old recovery-record schemas.

## Decisions

### Total metadata loss is not recoverable from policy plus parity

The planner keeps the existing matrix case but maps it to explicit non-executable new-lineage creation. Production preview remains useful and deterministic; apply refuses. This preserves the distinction between payload consistency and historical identity/topology authority.

### Publication identity comes from the admitted service

The portable service hashes a canonical serialization of its validated recovery topology plus every bound member's complete identity-observation set in semantic assignment order. The Linux adapter receives this opaque identity at construction, publishes it in endpoint metadata, and uses it for live discovery. It does not reopen fixture metadata to recreate admission.

### Checksum records are self-binding

A valid integrity state stores `ChecksumEvidenceBinding` beside digest and fence evidence. Baseline assessment compares stored fields directly against expected current extents and rejects duplicates and extras. No migration fallback reconstructs fields omitted by old records.

### SQLite uses a durable commit-intent sidecar

Before writing a candidate manifest, the adapter atomically persists and directory-syncs prior/proposed generations and full-manifest digests. Until that sidecar is reconciled, the live store blocks further operations. Reopen validates the database manifest and compares its full digest and generation: prior or proposed removes the sidecar and opens that exact state; neither refuses reconciliation. The sidecar is removed and its directory synced only after acknowledged manifest publication.

### Semantic command errors become operator results

The CLI converts `OperatorError` into `OperatorResult` after successful parsing and policy loading. Outcome determines exit status; human and JSON rendering use the same object. Parser and policy-input errors remain input diagnostics.

## Risks / Trade-offs

- Existing integrity records without bindings become invalid. This is an intentional clean break before format stability.
- A crash while removing a reconciled sidecar may cause another harmless reconciliation pass.
- Sidecar durability depends on filesystem `fsync`/rename semantics; DiskWeave claims only the observed OS contract, not unsupported hardware guarantees.
