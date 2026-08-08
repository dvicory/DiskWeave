## 1. Artifact and module contract

- [x] 1.1 Cite archived OS-003/OS-005 prerequisites, handoff Sections 6.1–6.5 and 22.2, and the topology/identity decision IDs.
- [x] 1.2 Define the handoff-required 20-section design with portable, simulator, macOS, Linux, and destructive-operation boundaries separated.
- [x] 1.3 Define the anchorless topology and identity capability delta with explicit fail-closed scenarios and successors.

## 2. Topology implementation

- [x] 2.1 Split topology semantics into a dedicated `dwv-core` module with stable array/slot/role/coding/assignment IDs and public re-exports.
- [x] 2.2 Implement immutable topology snapshots, assignment validation, monotonic epochs, protected geometry checks, and staged prepared/verified/committed transitions.
- [x] 2.3 Add bounded semantic evidence/report values without paths, runtime handles, SQL rows, or data-member metadata requirements.

## 3. Identity implementation

- [x] 3.1 Split identity evidence and assessment into a dedicated `dwv-store` module while preserving existing public types through re-exports.
- [x] 3.2 Implement deterministic match/changed/clone/insufficient/conflict/new-device decisions, including stable-source and geometry policy.
- [x] 3.3 Add clone, USB-bridge, file identity, replacement, capacity/geometry, reordered-discovery, and ambiguous-candidate fixtures.

## 4. Verification and handoff gate

- [x] 4.1 Add property-style order-independence and failed-preparation preservation tests.
- [x] 4.2 Run focused/workspace tests, formatting, dependency inspection, and OpenSpec validation.
- [x] 4.3 Keep Linux discovery, destructive replacement, online reshape, and operator UI explicitly gated to later OpenSpecs.
- [x] 4.4 Record OS-006 as complete only when all topology/identity tests pass; do not archive until the verification phase.
