## Why

Single-parity correctness needs a small independent reference model before transaction, simulator, topology, or optimized codec code is written. The model must make byte-range geometry, heterogeneous lengths, zero extension, erasure reconstruction, and checked arithmetic unambiguous.

## What Changes

- Define bytewise XOR parity over explicit data slots and protected ranges.
- Define incremental read-modify-write parity updates and full recomputation equivalence.
- Define exact reconstruction for every single known erasure and reject invalid geometry or capacity.
- Define portable golden vectors and a seam for future P/Q profiles without selecting one now.
- Keep codec semantics independent of device I/O, frontend tags, runtime choices, and CPU vector width.

## Capabilities

### New Capabilities

- `xor-reference-model`: Geometry-aware single-parity math, erasure reconstruction, checked arithmetic, golden vectors, and the future codec seam.

### Modified Capabilities

None. No existing OpenSpec capabilities are present.

## Impact

- Adds a portable reference codec and property/golden-vector tests.
- Establishes the parity math used by the simulator and transaction reference machine.
- Does not define P/Q coefficients, parity envelopes, device I/O, topology mutation, or a stable on-disk format.
