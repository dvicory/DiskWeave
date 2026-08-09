# parity-verification-repair Specification

## Purpose
This capability verifies every protected single-parity region without mutation
and permits selective repair only when independent current integrity evidence
uniquely identifies the bad shard.
## Requirements
### Requirement: Exhaustive verification is a read-only full scan
<!-- dwv:req req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->

The verifier SHALL read every configured data and parity byte in each selected
protected region, compare the observed parity with the configured XOR equation,
and produce a bounded per-region report. A matching region SHALL produce zero
parity writes and SHALL remain distinguishable from a parity rewrite.

#### Scenario: Matching array is scanned

- **WHEN** every data and parity region is readable and each parity equation
  matches
- **THEN** the report marks each region verified and the verifier performs no
  payload write

#### Scenario: A scan read is short or fails

- **WHEN** any required data or parity range cannot be read completely
- **THEN** the affected region is reported incomplete/unknown and no repair or
  clean authorization is produced

### Requirement: Sampling cannot establish clean state
<!-- dwv:req req.parity-verification-repair.sampling-cannot-establish-clean-state -->

The verifier MAY run a bounded sample for diagnostics, but sampled agreement
SHALL never be represented as exhaustive verification or authorize a `CLEAN`
transition.

#### Scenario: Sampled regions agree

- **WHEN** all selected sample ranges satisfy the parity equation
- **THEN** the report identifies the result as sampled and leaves exhaustive
  verification and clean certification outstanding

### Requirement: Mismatch classification requires independent evidence
<!-- dwv:req req.parity-verification-repair.mismatch-classification-requires-independent-evidence -->

For an equation mismatch, the verifier SHALL classify a parity shard as the
repair candidate only when all required data evidence is current and valid and
the parity evidence identifies the parity bytes as invalid. It SHALL classify a
single data shard only when that shard is the unique invalid digest and all
other required data and parity evidence is current and valid. Missing, stale,
conflicting, or multiple invalid evidence SHALL produce an ambiguous or
evidence-conflict result with no automatic repair candidate.

#### Scenario: Current hashes identify bad parity

- **WHEN** all data digests match current valid records, the parity digest does
  not match, and the parity equation mismatches
- **THEN** the report identifies only the parity region as a repair candidate

#### Scenario: Current hashes identify one bad data shard

- **WHEN** exactly one data digest does not match, every other data digest and
  the parity digest match current valid records, and the equation mismatches
- **THEN** the report identifies that data region as the sole repair candidate

#### Scenario: Evidence cannot identify a shard

- **WHEN** hashes are absent/stale/conflicting, more than one shard is invalid,
  or all hashes claim validity while the equation disagrees
- **THEN** the verifier reports an ambiguous integrity fault and performs no
  automatic repair

### Requirement: Selective repair is separately targeted and verified
<!-- dwv:req req.parity-verification-repair.selective-repair-is-separately-targeted-and-verified -->

The repair planner SHALL create a candidate only from an identified mismatch
and SHALL require a separate repair target by default. A repair result SHALL be
accepted only after the target bytes are read back, the independent digest is
verified, and the parity equation is recomputed. The original mismatch report
SHALL remain available for audit and forensic handling.

#### Scenario: Identified parity is rebuilt to a separate target

- **WHEN** current data evidence uniquely identifies parity as bad and a
  separate target is supplied
- **THEN** the verifier reconstructs parity, writes only the separate target,
  reads it back, and accepts it only after equation verification

#### Scenario: Identified data is reconstructed to a separate target

- **WHEN** current evidence uniquely identifies one data shard as bad and
  sufficient surviving data plus parity are readable
- **THEN** the verifier reconstructs that data range to a separate target and
  accepts it only after digest and parity verification

#### Scenario: Repair verification fails

- **WHEN** a repair target write, readback, digest, or equation check fails
- **THEN** the result is failed/uncertain, the source members remain untouched,
  and no clean or valid state is asserted

### Requirement: Reports are bounded and preserve authority boundaries
<!-- dwv:req req.parity-verification-repair.reports-are-bounded-and-preserve-authority-boundaries -->

Verification reports SHALL contain ranges, dispositions, evidence summaries,
error classes, and repair decisions without requiring payload bytes or raw
SQLite rows. The verifier SHALL reject invalid geometry, range, or identity
inputs and SHALL not introduce filesystem, FSKit, DiskImages, Linux, runtime,
or database types into portable verification semantics.

#### Scenario: Bounded scan configuration is valid

- **WHEN** the configured geometry, region size, and maximum region count are
  valid
- **THEN** the verifier accepts the scan and bounds live buffers and report
  entries to those limits

#### Scenario: Scan configuration is invalid

- **WHEN** a region size is zero, a range exceeds geometry, or the scan would
  exceed its configured region bound
- **THEN** the verifier fails before reading or writing any member

