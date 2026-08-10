## MODIFIED Requirements

### Requirement: Kernel requests preserve normalized semantics
<!-- dwv:req req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics -->
<!-- dwv:refines req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:refines req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

The Linux adapter SHALL validate every kernel-provided operation, flag, range, count, and identifier before semantic admission, then map supported exact-range reads, writes, and flushes directly into the canonical normalized request and portable service contracts. It SHALL preserve every normalized request field, pass write payloads as borrowed frontend-owned buffers without allocating a second request-boundary payload, and map terminal outcomes deterministically. Unsupported discard, write-zeroes, zoned operations, FUA, preflush, or unknown flags SHALL fail explicitly unless the complete path advertises and establishes the requested semantics.

#### Scenario: An ext4 read, write, or flush arrives

- **WHEN** the request is aligned, within fixed geometry, uses only advertised flags, and required authority is current
- **THEN** it completes through the normalized portable service path with exact range and persistence evidence

#### Scenario: A range overflows virtual geometry

- **WHEN** sector conversion, byte-count conversion, or checked end arithmetic overflows or exceeds the published capacity
- **THEN** the adapter rejects the request before semantic admission or protected mutation

#### Scenario: Unsupported intent arrives

- **WHEN** a request carries an operation or intent unsupported by the complete path
- **THEN** the adapter returns an explicit unsupported result and never translates it into a weaker operation or success

#### Scenario: A semantic result has no kernel completion mapping

- **WHEN** the semantic terminal result cannot be represented by the selected ublk completion contract
- **THEN** the adapter records the semantic result and explicit mapping refusal and does not report successful completion

#### Scenario: A retained frontend trace is replayed

- **WHEN** a bounded versioned frontend trace is imported
- **THEN** replay revalidates every normalized request and terminal mapping in sequence without executing backing I/O and reports the first divergence

#### Scenario: A write payload crosses the request boundary

- **WHEN** ublk supplies a validated write buffer
- **THEN** translation and service dispatch retain the same borrowed payload storage until synchronous semantic completion rather than cloning it into a service request wrapper
