## 1. Independent Inspection Command

- [x] 1.1 Add the `dwv-recovery-inspect [--json] <artifact>` binary target to `dwv-recovery-sqlite` using only the existing public read-only inspection seam and no new dependency.
- [x] 1.2 Map every `RecoveryInspection` value into one bounded `dwv.recovery-inspection.v1` result with experimental format status, known layer/version facts, supported manifest, non-authorization statement, and deterministic next action.
- [x] 1.3 Render the same result in human and JSON modes, returning success for a produced classification and distinct nonzero status for usage or pre-classification operational failure.

## 2. Contract Evidence

- [x] 2.1 Add focused command fixtures for supported, absent, corrupt-or-unreadable, unsupported, migration-required, and reconciliation-required artifacts; prove each fixture and adjacent directory state remain byte-for-byte unchanged.
- [x] 2.2 Add focused checks for malformed or oversized input, bounded output, human/JSON semantic equivalence, experimental and non-authorization claims, and deterministic process status.
- [x] 2.3 Prove the binary builds and runs without production `diskweave`, service, operator, or frontend dependencies, and smoke-test both a supported artifact and an absent artifact through the executable.

## 3. Verification and Canonicalization

- [x] 3.1 Run the focused `dwv-recovery-sqlite` tests and command smoke checks; fix every failure without broadening the capability.
- [x] 3.2 Run strict OpenSpec verification, archive the completed change, add the canonical requirement's implementation and evidence links, and resolve the resulting reviewed-requirement gate.
- [x] 3.3 Run `cargo xtask docs knowledge readiness`, `cargo xtask docs check`, and `cargo xtask docs build`; update U26/C8 campaign state with final canonical, implementation, evidence, and Bead anchors without starting another capability.
