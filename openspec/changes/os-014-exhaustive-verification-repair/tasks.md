## 1. Artifact and evidence contract

- [x] 1.1 Cite handoff invariants 13, 14, 15, and 20; Sections 8.10, 12.4, 22.3, 26.3, and 26.6; Phase 1 OS-014; and the portable/macOS boundary.
- [x] 1.2 Define exhaustive versus sampled reports, checksum evidence states, mismatch classifications, repair authorization, and source/target invariants.
- [x] 1.3 Define bounded range, short-read, identity/alias, repair-failure, and no-clean-certification behavior.

## 2. Portable verification crate

- [x] 2.1 Register `dwv-verify` in the workspace with explicit `scan`, `evidence`, `repair`, and `report` module boundaries.
- [x] 2.2 Add the bounded `VerificationStore`, scan configuration, evidence input, and stable error seams without OS, runtime, SQLite, or frontend types.
- [x] 2.3 Implement checked region partitioning and local XOR geometry, including short final regions and explicit zero extension.

## 3. Exhaustive scan and classification

- [x] 3.1 Implement read-only exhaustive scans that read every selected data/parity region and never issue payload writes.
- [x] 3.2 Implement sampled diagnostic scans that remain explicitly non-exhaustive and cannot authorize recovery `CLEAN`.
- [x] 3.3 Hash observed bytes and classify matching, parity-identified, data-identified, ambiguous, evidence-conflict, and incomplete regions.

## 4. Evidence-gated repair

- [x] 4.1 Build separate-target repair plans only for uniquely identified parity/data faults with current valid evidence.
- [x] 4.2 Reconstruct candidate bytes with the XOR reference, write only a separate target, read back, and verify digest plus parity equation before accepting.
- [x] 4.3 Preserve source bytes and mismatch reports on ambiguous, aliased, failed, or uncertain repair paths.

## 5. Verification and macOS evidence

- [x] 5.1 Add model tests for matching arrays, zero writes, sampling limits, and bounded scan failures.
- [x] 5.2 Add parity-fault, data-fault, hashless, stale/conflicting, multi-suspect, all-valid-conflict, and failed-repair tests.
- [x] 5.3 Add macOS temporary regular-file adapter tests for direct payload equality, separate repair targets, identity checks, and source preservation.
- [x] 5.4 Run workspace tests, format, targeted clippy, dependency inspection, and OpenSpec validation; record portable-demo and OS-015/016/017 boundaries.
- [x] 5.5 Mark complete after the workspace, identity/alias, repair, dependency, and OpenSpec acceptance passes; leave metadata-loss, degraded rebuild, and scrub/verified repair as separate changes.
