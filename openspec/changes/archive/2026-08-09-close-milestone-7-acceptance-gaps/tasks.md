## 1. Frontend trace and lifetime correctness

- [x] 1.1 Replace flattened records with a bounded versioned trace document that separately preserves kernel submission, normalized request, semantic terminal result, and kernel completion.
- [x] 1.2 Reserve trace capacity before semantic admission and retain tag ownership until successful kernel completion or explicit reconciliation.
- [x] 1.3 Add deterministic import/replay validation and expose it through `dwv demo disk trace-replay` without opening a fixture or device.

## 2. Deterministic failure coverage

- [x] 2.1 Add adapter regressions for range/flag rejection, tag and trace exhaustion, stale/duplicate completion, abandonment, malformed/divergent traces, and unrepresentable completion mapping.
- [x] 2.2 Add pure probe-control classification checks for absent and permission-denied ublk control access.
- [x] 2.3 Add shutdown-state checks proving unmount success cannot mask drain, checkpoint, or endpoint-removal failure.

## 3. Live workflow and evidence

- [x] 3.1 Retain and replay both live ARM64 frontend traces and record schema, bounds, digest, count, and replay outcome.
- [x] 3.2 Exercise remaining Linux-gated failure cases or record an exact deterministic substitute and claim boundary where host state cannot be safely induced.
- [x] 3.3 Map every correction layer to canonical requirements and architecture-v0.8 properties and record exact TLA+/Kani domains and assumptions.

## 4. Verification and completion

- [x] 4.1 Run focused adapter/CLI checks, workspace tests, Clippy, strict OpenSpec validation, documentation checks/build, TLC, and both Kani harnesses.
- [x] 4.2 Run the fresh ARM64 Linux ublk/ext4 acceptance workflow and verify the retained machine-readable evidence.
- [x] 4.3 Verify change artifacts, archive the change, reconstruct documentation, and describe the final Jujutsu revision accurately.
