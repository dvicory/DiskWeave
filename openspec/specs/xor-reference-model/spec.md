# xor-reference-model Specification

## Purpose
The XOR reference model defines portable single-parity byte semantics and independently testable geometry so every transaction, simulator, and optimized codec can be compared against one exact oracle.
## Requirements
### Requirement: XOR parity uses explicit protected geometry

The system SHALL map each data slot and parity slot through explicit protected lengths and byte ranges. Parity SHALL be the bytewise XOR of all data ranges with logical zero extension for shorter members. A parity payload SHALL be at least as large as the largest protected data payload, and virtual byte zero SHALL map to data-store byte zero.

Protected lengths and range invariants SHALL be established by checked constructors and remain immutable through the public API. A full-parity input SHALL contain exactly the declared protected length for each data slot; bytes outside that length SHALL be rejected rather than silently ignored. A caller-provided survivor SHALL contain exactly the intersection of its protected range with the requested range.

#### Scenario: Data members have different lengths

- **WHEN** parity is computed for data members with heterogeneous protected lengths
- **THEN** bytes beyond a shorter member contribute logical zero and no protected data tail is silently omitted

#### Scenario: Parity capacity is too small

- **WHEN** the usable parity payload is shorter than the largest protected data payload
- **THEN** geometry validation rejects the array before parity is exposed or written

#### Scenario: A range is split for computation

- **WHEN** a protected range is divided into batches or buffers
- **THEN** the resulting parity bytes are independent of batching, CPU vector width, or implementation language

#### Scenario: An input contains bytes outside protected geometry

- **WHEN** a data or survivor input is longer than the protected length represented by the geometry
- **THEN** the reference model rejects it before computing or mutating parity rather than silently truncating the extra bytes

### Requirement: Incremental updates and full recomputation are equivalent

For a write to data slot `k`, the reference model SHALL compute `P_new = P_old XOR D_k_old XOR D_k_new` over the affected range and SHALL produce the same result as recomputing parity from all current data. Partial writes SHALL be merged only within the declared protected range.

#### Scenario: A partial data write updates parity

- **WHEN** old target bytes, old parity bytes, and new target bytes are supplied for a valid range
- **THEN** the incremental parity result equals full XOR recomputation for that range

#### Scenario: The write is outside the protected range

- **WHEN** an update extends beyond the slot’s protected length or overflows its byte range
- **THEN** the reference model rejects it rather than truncating or silently changing protection coverage

### Requirement: Every single known erasure reconstructs exact bytes

Given parity and all data slots except one known missing slot over a valid range, the reference model SHALL reconstruct the missing bytes exactly. It SHALL refuse reconstruction when the missing slot, coding position, range, or surviving geometry is ambiguous or insufficient.

#### Scenario: One data range is missing

- **WHEN** parity and every other data range are available and trusted
- **THEN** XOR reconstruction returns bytes identical to the original missing range

#### Scenario: More than one data range is missing under single parity

- **WHEN** two or more data ranges are missing from a single-parity equation
- **THEN** reconstruction reports insufficient redundancy and does not invent bytes

#### Scenario: A source range is incomplete

- **WHEN** a surviving input is short, uncertain, or outside the captured topology epoch
- **THEN** reconstruction refuses the range and preserves the incomplete evidence

#### Scenario: A survivor contains an ambiguous extra tail

- **WHEN** a surviving input contains more bytes than the exact protected intersection requested for reconstruction
- **THEN** reconstruction rejects the input rather than treating the extra bytes as an unprotected or implicitly shifted range

### Requirement: Reference vectors and the future P/Q seam are portable

The project SHALL maintain deterministic golden vectors and property checks for parity, update, reconstruction, capacity, and checked arithmetic. The reference model SHALL expose a codec seam that does not depend on device I/O, frontend tags, runtime tasks, or CPU vector width. P/Q semantics SHALL remain unspecified until a separate fully parameterized profile is accepted.

#### Scenario: An optimized implementation is compared

- **WHEN** an optimized codec is run on generated inputs or golden vectors
- **THEN** its semantic results equal the reference model for all valid cases and its errors match the declared invalid cases

#### Scenario: Arithmetic input is invalid

- **WHEN** a range length, offset, allocation, or parity buffer calculation would overflow
- **THEN** the reference model returns a checked arithmetic error without allocation or mutation

#### Scenario: A P/Q implementation is proposed

- **WHEN** a future implementation proposes P/Q coefficients or field behavior
- **THEN** it uses a separate named profile and cannot change the XOR reference semantics or silently become stable format behavior

