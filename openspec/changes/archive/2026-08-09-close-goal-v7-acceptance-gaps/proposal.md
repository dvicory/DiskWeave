## Why

The archived OS-031 implementation proved real ARM64 ublk/ext4 I/O, but its frontend evidence does not yet preserve the full kernel-request → normalized-request → semantic-result → kernel-completion chain, and several Goal-v7 failure checks and proof mappings remain unevidenced. Goal-v7 must remain incomplete until those observable contracts are implemented and rerun.

## What Changes

- Make bounded Linux frontend traces versioned, privacy-safe, serializable, and deterministically replayable while preserving each correlation boundary separately.
- Add deterministic checks for trace limits, resource exhaustion, abandonment, unsupported and unrepresentable completion outcomes, probe classification, and failed shutdown/checkpoint behavior.
- Extend the ARM64 Linux workflow to retain the replayable trace and exercise the remaining environment- and lifecycle-gated failure cases where executable fault injection is possible.
- Map every correction layer to its canonical requirement and architecture-v0.8 property, including exact Kani input bounds and explicit non-claims.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `linux-ublk-frontend`: Clarify the required replayable correlation evidence, deterministic failure coverage, lifecycle failure behavior, and exact proof-bound recording needed for Goal-v7 completion.

## Impact

Affected code is limited to `dwv-frontend-ublk`, the root demo CLI integration needed to replay retained frontend traces, and focused fault-injection seams in the disposable fixture/lifecycle path. Affected evidence is limited to OS-031 scripts, records, manifest mappings, and Goal-v7 completion claims. Portable parity, recovery, and store semantics remain owned by their existing crates and requirements; no production durability, deployment, multi-device, or stable-format claim is added.
