## MODIFIED Requirements

### Requirement: Evidence gates control recovery authorization
<!-- dwv:req req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->
<!-- dwv:requires req.parity-verification-repair.mismatch-classification-requires-independent-evidence -->
<!-- dwv:requires req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution -->
<!-- dwv:requires req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback -->
<!-- dwv:requires req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed -->

A plan SHALL distinguish unavailable certificate receipt, exhaustive matching, uniquely verified separate-target repair, explicit data-authoritative rebaseline, ambiguous mismatch, incomplete scan, validated backup, and conflicting or absent evidence. Ambiguous, incomplete, or receipt-unavailable outcomes SHALL NOT authorize a fresh clean state or writable assembly. A mismatch SHALL use the independently owned current-evidence classification rather than identify a culprit locally. Operator confirmation SHALL authorize only matrix cases assigned to explicit data-authoritative rebaseline and SHALL NOT substitute for a certificate receipt.

The current product has no validator-issued certificate receipt. Every current public verification value supplied to a certificate-receipt-gated case SHALL return `CertificateReceiptUnavailable` before authorization, fresh-state creation, or a data/parity write. A future receipt capability requires a separate canonical change defining its producer and validator authority, exact array/topology/profile/session/envelope bindings, generation and freshness rules, replay or expiry behavior, and evidence and failure semantics.

#### Scenario: Exhaustive verification finds only matches

- **WHEN** all data and parity regions are read successfully and every equation agrees
- **THEN** recovery state may record the completed scan, matching regions incur zero payload writes, and checksum baseline creation is labeled required work rather than recovered historical evidence

#### Scenario: A uniquely identified separate-target repair is verified

- **WHEN** current integrity evidence uniquely identifies one bad shard and separate-target write, readback, digest, and equation verification all succeed for the exact run
- **THEN** the run-bound repair outcome may satisfy the survivor plan, and only an otherwise authorized fresh-state case may create new recovery state without overwriting the sole historical source

#### Scenario: A mismatch is unsupported or ambiguous

- **WHEN** integrity evidence is absent, stale, conflicting, incomplete, or permits more than one correction
- **THEN** the plan preserves the mismatch and requires explicit data-authoritative rebaseline or a separate forensic target rather than selecting data or parity automatically

#### Scenario: Data authority requires explicit confirmation

- **WHEN** data-authoritative rebaseline is selected after an unlocalizable mismatch
- **THEN** ordinary authorization fails and only the explicit operator-confirmed path may acknowledge the destructive loss of forensic certainty

#### Scenario: Public evidence cannot forge a certificate receipt

- **WHEN** any currently public verification value is supplied to any certificate-receipt-gated case, with or without operator confirmation
- **THEN** authorization returns `CertificateReceiptUnavailable`, no authorization is issued, and no fresh semantic state or data/parity write occurs

### Requirement: Dry-run reporting is bounded and portable
<!-- dwv:req req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded -->

The metadata-loss dry run SHALL enumerate the complete matrix with stable case IDs, dispositions, evidence requirements, confirmation requirements, data/parity-write policy, and certificate-receipt availability. It SHALL omit payload bytes, paths, storage-engine handles, OS/frontend types, and unbounded operator text. The dry run SHALL be diagnostic and SHALL not authorize `CLEAN`, writable assembly, or a data/parity write.

#### Scenario: The matrix dry run is requested on a portable host

- **WHEN** the portable example is run against the local workspace
- **THEN** it prints every matrix case deterministically, reports certificate-receipt-gated cases as unavailable, and exits without opening or changing a data or recovery store

#### Scenario: Platform integration is unavailable

- **WHEN** a Linux frontend, macOS bridge, or other platform integration is absent
- **THEN** the portable planner, tests, and regular-file evaluation remain usable and make no platform conformance claim
