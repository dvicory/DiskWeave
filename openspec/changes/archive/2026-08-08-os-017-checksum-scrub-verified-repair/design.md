## Context

See `proposal.md` for motivation. OS-011 already provides generational checksum jobs, durable invalidation, profile migration, and BLAKE3 evidence. OS-014 already provides bounded exhaustive parity scans, evidence classification, separate-target repair, identity binding, readback, and equation verification. OS-016 provides the file-backed replacement and recovery-generation contracts. The remaining gap is orchestration: one scrub/repair path must compose these seams without weakening their authority boundaries.

## Goals / Non-Goals

**Goals:**

- Reuse `dwv-verify` as the bounded classifier and repair executor rather than duplicating parity logic in the CLI.
- Make the scrub report and repair outcome explicit, serializable, and safe to replay in deterministic tests.
- Bind plans to source/target identities and the recovery/topology/checksum generations available at planning time.
- Use the existing `RecoveryStateStore` protocol transaction for invalidation and integrity-digest publication where the evidence and durable fence permit it.
- Exercise the complete disposable regular-file flow from CLI fixture initialization through fault classification, refusal, or verified separate-target repair.

**Non-Goals:**

- No online scrub scheduler, degraded writes, P/Q repair, Linux frontend, FSKit/DiskImages path, or physical power-loss certification.
- No stable checksum serialization format; the CLI output is an evidence report, not a persistent format promise.
- No in-place source-member repair; source media remains read-only for this workflow.

## Decisions

### Compose existing verifier contracts

Add the smallest scrub orchestration seam to `dwv-verify` around `VerificationReport`, `RepairPlan`, and `RepairOutcome`. The classifier remains exhaustive and read-only; repair uses the existing candidate validation and separate-target adapter. Do not add a second parity implementation or a general job framework.

### Bind plans at the orchestration boundary

The report binding already captures source identities and a verification run ID. The scrub plan will additionally carry topology/recovery and active checksum-set generations plus the replacement identity. The adapter rejects a stale plan before any write when those values differ from the current fixture/recovery snapshot.

### Publish integrity only after verification

A repair outcome is not a valid checksum record by itself. The file-backed integration first completes the separate-target write and readback/equation checks, then uses the existing durable fence and `InstallIntegrityDigest` protocol mutation for the repaired target when available. Failed or uncertain repairs do not publish valid evidence.

### Keep the CLI disposable and fault-driven

Extend the existing `dwv demo` command surface with a scrub command that reports the scan, classifications, repair/refusal outcome, and claim boundary. Tests induce faults by mutating disposable fixture bytes and exercise absent/stale evidence, multiple suspects, target failure, and stale-plan rejection. No user data path is introduced.

## Risks / Trade-offs

- The current file-backed demo has one parity member and two data members, so it proves single-parity scrub/repair only; P/Q and online scheduling remain explicitly unsupported.
- Durable checksum records are represented by the existing recovery integrity state, while the richer checksum profile/set records remain an in-memory authority seam; the CLI must not claim a stable checksum database format.
- A separate target proves source preservation but does not replace a production member in this milestone; replacement assignment remains OS-016/OS-037 policy.
