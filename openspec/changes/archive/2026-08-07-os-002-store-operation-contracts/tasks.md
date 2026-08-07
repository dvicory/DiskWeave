## 1. Artifact contract

- [x] 1.1 Cite OS-000/OS-001 prerequisites, handoff Sections 4.5, 5, 11, 12, 22, and 25, and the relevant D/P/T/F identifiers.
- [x] 1.2 Make `design.md` contain the twenty numbered sections in the prescribed order with portable/concrete-store scope separated.
- [x] 1.3 Define validator-compatible deltas for exact outcomes, evidence, capability profiles, operation-slot lifetime, identity, bounded resources, and conservative failures.
- [x] 1.4 Record OS-004, OS-012, and OS-031 as successors without claiming file-backed, Linux, SQLite, or hardware evidence.

## 2. Portable acceptance to be evidenced by implementation

- [x] 2.1 Verify exact-range success, short, failed, uncertain, and duplicate dispositions include stable evidence and do not hide unknown media effects.
- [x] 2.2 Verify capability observations cover geometry, alignment, transfer limits, flush/FUA/order, torn/volatile behavior, write-zeroes/discard, sparse/cancellation behavior, and identity sources.
- [x] 2.3 Verify simulation-certified, portable-demo, production-read-only, and production-write-safe profiles refuse missing or unknown required evidence.
- [x] 2.4 Verify generation-bearing slots reject stale completions before resource lookup, ignore duplicate terminal deliveries, and release only after child terminality and reconciliation.
- [x] 2.5 Verify bounded admission/backpressure, disappearance and identity-change invalidation, ambiguous writable reuse refusal, and no blind retry of uncertain non-idempotent writes.
- [x] 2.6 Add or verify deterministic portable fake-adapter and simulator evidence for EIO, timeout, reorder, duplicate, stale, disappearance, capability failure, identity ambiguity, and resource exhaustion; retain minimized regressions.

## 3. Gated evidence and verification

- [x] 3.1 Keep concrete file/device, SQLite, io_uring, ublk, Linux, macOS, power-loss, and hardware criteria explicitly gated to OS-004, OS-012, OS-031, and later release OpenSpecs.
- [x] 3.2 Run `openspec validate os-002-store-operation-contracts --json` after the final artifact edits and resolve all reported errors.
- [x] 3.3 Run `openspec status --change os-002-store-operation-contracts --json` and `openspec instructions apply --change os-002-store-operation-contracts --json` to confirm reproducible scope and remaining evidence.
- [x] 3.4 Keep the handoff untouched and defer archive until implementation and verification are complete.
