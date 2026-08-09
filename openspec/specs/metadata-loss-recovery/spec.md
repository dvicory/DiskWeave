# metadata-loss-recovery Specification

## Purpose
This capability makes loss, corruption, staleness, or disagreement of DiskWeave recovery metadata a conservative, executable recovery plan. It follows handoff Sections 8.10, 12.5, 17.6, 22.3, 26.3, 26.5, and 26.6; it does not treat algebraic solvability or a process-local database write as historical correctness.
## Requirements
### Requirement: The metadata-loss matrix is total and conservative
<!-- dwv:req req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->

The portable recovery layer SHALL expose a bounded plan for every Section 12.5 case: all-data single-parity certified/uncertified, all-data dual-parity certified/uncertified, all data with parity lost, one-data erasure with certified/uncertified P or P/Q, two-data erasures with certified P/Q or lost coding positions, all metadata lost with all data present, ambiguous topology, ambiguous parity identity, a validated backup, disagreeing replicas, surviving checksum evidence, and unavailable checksum evidence. Each plan SHALL name its disposition, required evidence, operator confirmation requirements, payload-write policy, and whether a fresh checksum baseline is required.

#### Scenario: Every handoff row has an executable plan

- **WHEN** the matrix is enumerated
- **THEN** every named Section 12.5 case has exactly one stable identifier and a conservative recovery action, and no case permits an implicit in-place payload write

#### Scenario: An uncertified all-data array is encountered

- **WHEN** all data and parity are present but the envelope is dirty, unknown, absent, or not past the session-certificate gate
- **THEN** the plan requires exhaustive verification, leaves matching regions untouched, and routes mismatches through the OS-014 evidence policy

#### Scenario: A certified clean envelope is encountered

- **WHEN** every required parity certificate agrees and the declared session-certificate gate has passed
- **THEN** the plan may recreate recovery state without an exhaustive parity rewrite, while still rebuilding checksum coverage and recording that latent media corruption is not ruled out

#### Scenario: The certificate gate has no executable receipt

- **WHEN** Gate H has not supplied a validated certificate receipt type
- **THEN** the certified-envelope row remains a non-authorizing plan and cannot create fresh state from a caller-selected classification alone

### Requirement: Evidence gates control recovery authorization
<!-- dwv:req req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->

A plan SHALL distinguish certified-clean evidence, exhaustive matching, verified evidence-gated repairs, explicit data-authoritative rebaseline, ambiguous mismatch, incomplete scan, validated backup, and conflicting/absent evidence. Ambiguous or incomplete outcomes SHALL NOT authorize a fresh clean state or writable assembly. A parity equation mismatch SHALL not identify a culprit without current valid independent evidence.

#### Scenario: Exhaustive verification finds only matches

- **WHEN** all data and parity regions are read successfully and every equation agrees
- **THEN** the recovery state may record the completed scan, matching regions incur zero payload writes, and checksum baseline creation is labeled as required work rather than recovered historical evidence

#### Scenario: Hash-backed selective repair is verified

- **WHEN** OS-014 identifies one bad shard and a separate-target repair passes readback, digest, and equation verification
- **THEN** the run-bound repair receipt may satisfy the underlying survivor plan; a completed all-data/single-parity plan may create new recovery state without overwriting the only historical source as an implicit action

#### Scenario: A mismatch is hashless or ambiguous

- **WHEN** hashes are absent, stale, conflicting, or more than one correction is plausible
- **THEN** the plan preserves the mismatch and requires explicit data-authoritative rebaseline or a separate forensic target; it does not choose data or parity automatically

#### Scenario: Data authority requires explicit confirmation

- **WHEN** a caller selects data-authoritative rebaseline after an unlocalizable mismatch
- **THEN** ordinary authorization fails and only the explicit operator-confirmed authorization path can acknowledge the destructive loss of forensic certainty

### Requirement: Identity and topology ambiguity fails closed
<!-- dwv:req req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed -->

Recovery discovery SHALL preserve all candidates and require an unambiguous topology/profile/coding-position interpretation before writable assembly. Duplicate clone evidence, ambiguous parity candidates, lost historical Q coding positions, and unresolved assignment mappings SHALL produce read-only or refused outcomes. When all data survive and historical identity is unnecessary, a new array lineage SHALL be explicit rather than silently reusing an old assignment.

#### Scenario: All data survive but historical metadata is unavailable

- **WHEN** data members are explicitly identified and a new topology is proposed
- **THEN** the plan requires a new lineage and fresh parity/checksum baseline, preserving old parity as evidence or targeting new parity separately; fresh state remains blocked until rebuild and baseline completion are verified

#### Scenario: A cloned candidate or parity identity is ambiguous

- **WHEN** two candidates share identity observations or more than one parity device could fill the role
- **THEN** writable assembly and destructive selection are refused while bounded read-only inspection remains available

#### Scenario: Q coding positions are lost

- **WHEN** two data members are absent and surviving P/Q metadata cannot establish historical coding positions
- **THEN** automatic decode and writable assembly are refused; the plan explains that guessing coefficients is unsafe

### Requirement: Fresh recovery state records a new baseline and audit
<!-- dwv:req req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->

After an authorized completed all-data/single-parity recovery, the implementation SHALL be able to create a bounded semantic manifest with the selected validated topology, generation zero, a new checksum-baseline obligation, and a metadata-loss audit record. P/Q, parity rebuild, missing-member reconstruction, and new-lineage plans SHALL NOT create fresh state until later changes provide verified completion receipts. The operation SHALL not depend on SQLite pages, row IDs, paths, or data-member payload metadata, and deletion of recovery state SHALL not delete direct payload files.

#### Scenario: An authorized fresh state is created

- **WHEN** an evidence gate succeeds for a plan that creates recovery state
- **THEN** the resulting manifest has an active topology carrying array identity, stable slots, roles, coding positions, assignment instances/generations/evidence, and a checksum-baseline-required audit with no claim that absent integrity records are already established

#### Scenario: Authorization is insufficient

- **WHEN** a caller presents an ambiguous mismatch, incomplete scan, conflicting replica set, or refused matrix case
- **THEN** fresh-state creation fails without mutating the semantic snapshot or payload stores

#### Scenario: Recovery state is recreated through the evaluation adapter

- **WHEN** an evaluation-only SQLite adapter initializes a new database from an authorized semantic manifest
- **THEN** export returns bounded semantic header data and direct regular-file payload bytes remain unchanged and readable

### Requirement: Dry-run reporting is bounded and portable
<!-- dwv:req req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable -->

The metadata-loss dry run SHALL enumerate the complete matrix with stable case IDs, dispositions, evidence requirements, confirmation requirements, and payload-write policy. It SHALL omit payload bytes, paths, SQLite handles, OS/frontend types, and unbounded operator text. The dry run SHALL be diagnostic and SHALL not authorize `CLEAN`, writable assembly, or payload mutation.

#### Scenario: The matrix dry run is requested on macOS

- **WHEN** the portable example is run against the local workspace
- **THEN** it prints every matrix case deterministically and exits without opening or changing a data or recovery store

#### Scenario: Linux-only integration is unavailable

- **WHEN** a Linux frontend or kernel bridge is not present
- **THEN** the portable planner, tests, and regular-file evaluation remain usable and make no Linux conformance claim

