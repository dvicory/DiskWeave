## Context

OS-013 owns healthy single-parity request execution over ordinary file-backed
members. OS-011 owns generational checksum evidence, while OS-015 will own
metadata-loss recovery actions. OS-014 therefore supplies a portable,
read-first verification and repair-planning boundary that can be exercised on
macOS without selecting a filesystem bridge or a permanent parity format.

## Goals / Non-Goals

**Goals:**

- Scan bounded ranges across all data and parity members and compare the
  reference XOR equation without mutating a matching array.
- Turn current checksum evidence into conservative mismatch classifications
  and separate-target repair candidates.
- Reconstruct and verify a candidate target through a small store adapter seam.
- Keep sampled diagnostics, parity consistency, checksum validity, and recovery
  `CLEAN` authorization as distinct facts.

**Non-Goals:**

- Reconstructing a missing member or resumable rebuild; those belong to
  OS-015/016.
- Mutating an ambiguous or hashless mismatch, in-place destructive repair, or
  automatic data-authoritative rebaseline.
- Reading SQLite tables, owning topology discovery, implementing P/Q, or
  adding macOS/Linux frontend APIs.

## Decisions

### 1. Add a portable `dwv-verify` crate with explicit modules

Create `scan`, `evidence`, `repair`, and `report` modules. The public API uses
`dwv_core::ByteRange`, `dwv_codec::Geometry`, checksum semantic records/digests,
and a small `VerificationStore` trait. This keeps file descriptors and
operation-slot details in adapters while making the scan/classifier usable by
the simulator, service, and future recovery tools.

An implementation inside `dwv-service` was rejected because it would couple
verification policy to the healthy request lifecycle and make OS-015/016
reuse harder. A direct SQLite reader was rejected because recovery semantics,
not table layout, are the authority.

### 2. Scan by bounded logical regions

`ScanConfig` partitions the protected address space into non-empty regions,
checks geometry and a maximum region count before I/O, and holds only the
current data/parity buffers. Each region uses a local equal-length codec
geometry so the reference XOR operation remains exact even when a future
member has a shorter logical extent with explicit zero extension.

The exhaustive path and sampled path share range validation, but the report
records `Exhaustive` versus `Sampled`; no report from this crate directly
authorizes recovery `CLEAN`.

### 3. Classify evidence before planning a write

The adapter supplies per-shard `DigestEvidence` derived from current
OS-011-valid records. The verifier hashes the bytes it read and compares those
digests independently from the parity equation. A mismatch is repairable only
when exactly one shard is identified; missing/stale/conflicting records and
all-valid equation conflicts become non-repairable reports.

The classifier intentionally does not infer that parity is wrong merely
because the equation disagrees. This follows the handoff's Section 12.4 and
26.6 policy and preserves the old bytes for forensic comparison.

### 4. Separate-target repair is the only first implementation

`apply_repair` accepts a target store distinct from the source stores by API
contract, reconstructs the candidate range with `XorReference`, writes the
target, reads it back, and verifies both its digest and the resulting equation.
The source data and parity stores are never written by this operation. An
adapter that cannot prove target/source separation must refuse the operation;
in-place authorization is deferred to a later explicit recovery change.

### 5. Reports never contain payload bytes

Reports carry region/range, scan mode, read disposition, evidence status,
classification, and repair outcome only. This keeps traces bounded and avoids
turning diagnostic output into a second data authority.

## Risks / Trade-offs

- **[Risk]** A full scan is expensive for large arrays. → Bound transfer size,
  region count, and live buffers; leave resumable cursors to OS-015/016.
- **[Risk]** A checksum record can be semantically current while the adapter
  supplies bytes from a changed path. → Require the existing store identity and
  topology checks at the adapter boundary; the portable classifier never treats
  path names as identity.
- **[Risk]** A separate target can still be misconfigured as an alias. → Make
  separation a required adapter precondition and reuse OS-012 identity/alias
  checks before exposing repair execution.
- **[Risk]** A successful equation scan may be mistaken for a clean state. →
  Keep `parity_consistent` and recovery `CLEAN` authorization separate and
  assert the distinction in tests.

## Migration Plan

No persistent format changes. Add the crate as a workspace member, use it for
portable verification fixtures, and preserve all source payloads and existing
OS-011 records. Future OS-015 can consume reports to build a fresh recovery
state, while OS-016/017 can add missing-member and scrub workflows.
