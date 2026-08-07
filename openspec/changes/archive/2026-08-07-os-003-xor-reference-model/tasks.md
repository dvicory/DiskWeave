## 1. Reference codec

- [x] 1.1 Add the dependency-free `dwv-codec` workspace package and preserve a separate reference path for later optimized implementations.
- [x] 1.2 Define protected geometry, explicit slot lengths, parity capacity validation, zero extension, and checked range arithmetic.
- [x] 1.3 Implement full XOR parity, incremental read-modify-write update, and single-known-erasure reconstruction.

## 2. Correctness evidence

- [x] 2.1 Add deterministic golden vectors for heterogeneous lengths, zero tails, updates, and reconstruction.
- [x] 2.2 Add randomized/property-style tests proving incremental parity equals full recomputation and every valid single erasure reconstructs exactly.
- [x] 2.3 Add negative tests for overflow, insufficient capacity, invalid ranges, incomplete survivors, and multiple erasures.

## 3. Future seam

- [x] 3.1 Define a codec seam that leaves P/Q field, coefficient, ordering, and persistent profile decisions to a later OpenSpec.
- [x] 3.2 Run `openspec validate os-003-xor-reference-model --json`, workspace tests, and the OpenSpec apply instructions; resolve failures.
- [x] 3.3 Mark OS-003 complete only after reference and negative tests pass and record OS-004, OS-006, and OS-008 as successors.

## 4. Handoff acceptance hardening

- [x] 4.1 Make `ByteRange` and `Geometry` invariants enforceable through checked construction and immutable public accessors.
- [x] 4.2 Enforce exact protected data and survivor lengths, preserve logical zero extension, and derive slice indexes through checked bounds.
- [x] 4.3 Add deterministic boundary/property coverage for zero-length members, non-zero offsets, protected tails, and rejected overlong inputs.
- [x] 4.4 Run focused `dwv-codec` tests and validate the updated OS-003 artifacts without archiving the change.
