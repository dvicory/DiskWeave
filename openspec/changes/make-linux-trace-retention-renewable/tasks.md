## 1. Implementation

- [x] 1.1 Replace append-only trace reservation with a fixed-capacity renewable slot collection, choosing the smallest ring/deque-like or equivalent representation that preserves original sequence order and explicit available, incomplete, terminal-retained, and reclaimable eligibility states.
- [x] 1.2 Wire reservation and retirement into the Linux admission path so every request reserves before semantic admission or protected mutation, the oldest safely reclaimable terminal record is the only reusable record, and no-safe-slot refusal leaves existing records and owners untouched.
- [x] 1.3 Close the terminal-evidence/resource-release ordering race: make trace-slot reclaimability occur only after operation-slot, tag, buffer, child, durability, recovery, and reconciliation owners permit reclamation, while preserving stale and duplicate generation checks and abandonment semantics.
- [x] 1.4 Update bounded export and replay to retain original non-contiguous sequence values, accept a retained window whose first sequence is greater than one, disclose retired prefixes as partial-session evidence, and keep incomplete reservations reconciliation-required.
- [x] 1.5 Separate retired-record accounting from refusal outcomes, make the retired count saturating with an explicit saturation state that never gates ordinary service, and explicitly migrate or refuse ambiguous experimental trace-schema versions without promising compatibility.
- [x] 1.6 Preserve conservative shutdown and crash handling: never synthesize terminal evidence or reuse an incomplete reservation, and keep the change limited to the existing Linux frontend without durable trace segments, full history, or generic retention infrastructure.

## 2. Focused Evidence

- [ ] 2.1 Add focused evidence for the 4,097th ordinary request, safe reuse of only a fully terminal record, and explicit refusal before semantic admission when every trace slot is unsafe to reclaim.
- [ ] 2.2 Add focused evidence for partial-session export disclosure and deterministic replay when the first retained sequence is greater than one, including preservation of original sequence values and first-divergence reporting.
- [ ] 2.3 Add focused evidence that retired-record accounting exposes saturation while later safely reclaimable records continue to admit ordinary requests, and that saturation is not treated as a lifetime service ceiling.
- [ ] 2.4 Add focused shutdown/crash evidence for an incomplete reservation, proving reconciliation-required handling, no fabricated terminal result, no premature ownership release, and no reuse of the incomplete slot; retain existing stale, duplicate, and abandonment coverage.

## 3. Dependent Review and Verification

- [ ] 3.1 Review the changed owner and each named dependent/constraint (`kernel-requests-preserve-normalized-semantics`, `operation-slots-own-backend-lifetimes-and-generations`, `frontend-lifecycle-events-have-explicit-abandonment-semantics`, and `hostile-inputs-and-resources-are-bounded-before-admission`) individually; record why each remains valid and remove any duplicate rolling-retention policy from implementation or evidence.
- [ ] 3.2 Run the complete focused `dwv-frontend-ublk` test module and executable smoke checks for every scenario above and the existing trace/tag lifecycle cases.
- [ ] 3.3 Confirm the implementation and evidence make no durable-history, complete-session, stable-format, or compatibility promise and introduce no new capability or general retention framework; resolve any schema or container choice within the design boundary before archive.

## 4. Canonicalization and Archive

- [ ] 4.1 Run strict OpenSpec verification, archive `make-linux-trace-retention-renewable`, add the unchanged canonical requirement's implementation and evidence links, and resolve every affected reviewed-requirement gate individually.
- [ ] 4.2 Run `cargo xtask docs knowledge readiness`, `cargo xtask docs check`, and `cargo xtask docs build`; update the **mounted-service trace continuity and renewable diagnostic retention** and `arch.diskweave.correctness-resources-are-bounded-and-observable` campaign state with final canonical, implementation, evidence, and Bead anchors before closing the change.
