## 1. Artifact contract

- [x] 1.1 Cite OS-000, the relevant handoff sections, D/P/T/F identifiers, and the Phase 0 gate contribution.
- [x] 1.2 Make `design.md` contain the twenty numbered sections in the prescribed order with portable/platform scope separated.
- [x] 1.3 Define validator-compatible deltas for request validation, intent preservation, lifecycle, bounded conformance, and evidence classification.
- [x] 1.4 Record OS-004, OS-013, OS-020, and OS-030 as successors without claiming their platform evidence.

## 2. Portable acceptance to be evidenced by implementation

- [x] 2.1 Verify the dependency-free semantic package exposes stable identifiers, checked byte ranges, normalized operations, ordering, durability, lifecycle, capability, and completion-interest contracts.
- [x] 2.2 Verify overflow, invalid buffer relationships, unsupported discard/zoned operations, and unproven durability intents fail before backend admission.
- [x] 2.3 Verify abandonment suppresses delivery interest only, frontend loss preserves duplicate uncertainty, and quiescence is not reported as media durability.
- [x] 2.4 Verify bounded per-request transfer admission, captured topology-epoch propagation for adapter comparison, exact error classes, and no OS/runtime/database types in the public dependency graph. Operation-slot generation rejection is evidenced by OS-002.
- [x] 2.5 Add or verify deterministic conformance evidence for field preservation and all applicable portable failure cases; retain minimized regressions for any counterexample.

## 3. Gated evidence and verification

- [x] 3.1 Keep physical flush/FUA, Linux, macOS, device, power-loss, and hardware criteria explicitly gated to OS-020, OS-030, and later release OpenSpecs.
- [x] 3.2 Run `openspec validate os-001-normalized-block-semantics --json` after the final artifact edits and resolve all reported errors.
- [x] 3.3 Run `openspec status --change os-001-normalized-block-semantics --json` and `openspec instructions apply --change os-001-normalized-block-semantics --json` to confirm reproducible scope and remaining evidence.
- [x] 3.4 Keep the handoff untouched and defer archive until implementation and verification are complete.
