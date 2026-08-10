## 1. Canonical metadata semantics

- [x] 1.1 Apply the five modified metadata-loss requirements with stable IDs, self-contained wording, and the approved direct relationships.
- [x] 1.2 Remove current OS, gate, handoff-section, architecture-section, and other historical authority wording from metadata-loss canonical and implementation surfaces.
- [x] 1.3 Record individual reviewed fingerprint outcomes for every changed or dependency-invalidated current requirement.

## 2. Non-forgeable certificate boundary

- [x] 2.1 Remove the caller-constructible certified-envelope verification value and add the stable certificate-receipt-unavailable error.
- [x] 2.2 Make every certificate-receipt-gated case fail before evidence matching or operator confirmation without adding a receipt API.
- [x] 2.3 Preserve all non-certificate authorization, refusal, fresh-state, and payload-write policies and update the versioned dry-run output.

## 3. Focused regression coverage

- [x] 3.1 Test every public verification value through ordinary and operator-confirmed authorization for every certificate-gated case.
- [x] 3.2 Test the authorizable-now, certificate-unavailable, and other-refusal matrix classes plus deterministic output.
- [x] 3.3 Retain focused coverage for exhaustive match/repair, backup/replica, operator rebaseline, identity ambiguity, incomplete/conflicting evidence, and non-creating fresh-state paths.

## 4. Validation and review handoff

- [x] 4.1 Run strict OpenSpec, focused crate, xtask, documentation readiness/check/build/clean-room, workspace test, formatting, and strict Clippy gates.
- [x] 4.2 Inspect final ownership views, relationship counts, stale-review results, requirement links, and the exact working-copy diff.
- [x] 4.3 Stage the required self-contained implementation review report and leave this change unarchived pending external approval.
