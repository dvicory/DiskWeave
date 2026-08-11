# metadata-loss-recovery Specification

## Purpose
This capability makes loss, corruption, staleness, or disagreement of DiskWeave recovery metadata a conservative, executable recovery plan using current canonical identity, verification, repair, envelope, recovery-state, and evidence contracts.
## Requirements
### Requirement: The metadata-loss matrix is total and conservative
<!-- dwv:req req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.parity-envelope-profiles.envelope-copies-are-independently-inspectable -->
<!-- dwv:requires req.parity-envelope-profiles.envelope-disagreement-resolves-conservatively -->
<!-- dwv:requires req.parity-envelope-profiles.session-transitions-have-ordered-recovery-semantics -->

The portable recovery layer SHALL expose a bounded plan for this complete case set: all-data single-parity with available or unavailable certificate receipt, all-data dual-parity with available or unavailable certificate receipt, all data with parity lost, one-data erasure with available or unavailable P or P/Q certificate receipt, two-data erasures with an available P/Q certificate receipt or lost coding positions, all metadata lost with all data present, ambiguous topology, ambiguous parity identity, a validated backup, disagreeing replicas, surviving checksum evidence, and unavailable checksum evidence. Each plan SHALL name its disposition, required evidence, operator-confirmation requirement, payload-write policy, and whether a fresh checksum baseline is required. A certificate-receipt-gated plan SHALL remain inspectable but non-authorizing while no current validator-issued receipt capability exists.

#### Scenario: Every matrix case has an executable disposition

- **WHEN** the matrix is enumerated
- **THEN** every named case has exactly one stable identifier and a conservative recovery action, and no case permits an implicit in-place payload write

#### Scenario: An all-data array has no certificate receipt

- **WHEN** all data and parity are present but envelope evidence is dirty, unknown, absent, conflicting, or lacks a current validator-issued receipt
- **THEN** the plan requires exhaustive read-only verification, leaves matching regions untouched, and routes mismatches through the independently owned classification and verified separate-target repair policies

#### Scenario: A future validated clean-session receipt is available

- **WHEN** a future canonical receipt validator proves all required envelope copies, topology/profile/session bindings, and clean-session evidence current and valid
- **THEN** the plan may recreate recovery state without an exhaustive parity rewrite while still requiring new checksum coverage and recording that latent media corruption is not ruled out

#### Scenario: No certificate receipt capability exists

- **WHEN** the current product has no validator-issued non-forgeable receipt type
- **THEN** every certificate-receipt-gated row remains a non-authorizing bounded plan and cannot create fresh state from caller-selected evidence or operator confirmation

### Requirement: Evidence gates control recovery authorization
<!-- dwv:req req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->
<!-- dwv:requires req.parity-verification-repair.mismatch-classification-requires-independent-evidence -->
<!-- dwv:requires req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution -->
<!-- dwv:requires req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback -->
<!-- dwv:requires req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed -->

A plan SHALL distinguish unavailable certificate receipt, exhaustive matching, uniquely verified separate-target repair, explicit data-authoritative rebaseline, ambiguous mismatch, incomplete scan, validated backup, and conflicting or absent evidence. Ambiguous, incomplete, or receipt-unavailable outcomes SHALL NOT authorize a fresh clean state or writable assembly. A mismatch SHALL use the independently owned current-evidence classification rather than identify a culprit locally. Operator confirmation SHALL authorize only matrix cases assigned to explicit data-authoritative rebaseline and SHALL NOT substitute for a certificate receipt.

The current product has no validator-issued certificate receipt. Every current public verification value supplied to a certificate-receipt-gated case SHALL return `CertificateReceiptUnavailable` before authorization, fresh-state creation, or payload mutation. A future receipt capability requires a separate canonical change defining its producer and validator authority, exact array/topology/profile/session/envelope bindings, generation and freshness rules, replay or expiry behavior, and evidence and failure semantics.

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
- **THEN** authorization returns `CertificateReceiptUnavailable`, no authorization is issued, and no fresh semantic state or payload mutation occurs

### Requirement: Identity and topology ambiguity fails closed
<!-- dwv:req req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:refines req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity -->

Metadata-loss recovery SHALL preserve all candidates and require independently established, unambiguous topology, profile, coding-position, and member-identity authority before recovering a prior array lineage or permitting writable assembly. Declarative policy and algebraic parity agreement SHALL NOT manufacture historical identity or topology authority. Duplicate clone evidence, ambiguous parity candidates, lost historical Q coding positions, and unresolved assignment mappings SHALL produce read-only or refused outcomes. When all data survive but prior recovery authority is unavailable, the operation SHALL be classified as explicit new-lineage creation rather than recovery of the policy-described topology. Until a canonical authority model defines that new-lineage transition, the current implementation SHALL refuse authorization and SHALL NOT create fresh recovery state.

#### Scenario: All data survive but prior recovery state is unavailable

- **WHEN** all payload members named by declarative policy are readable and parity equations match but independent current evidence cannot establish the prior array identity, assignments, and topology epoch
- **THEN** the plan is non-executable recovery, requires explicit new-lineage creation with fresh independently verified parity and checksum baselines, preserves old parity as evidence or targets new parity separately, and creates no fresh recovery state

#### Scenario: Apply is requested before custody-loss authority exists

- **WHEN** the current implementation is asked to authorize the all-metadata-lost/all-data-present case without a canonical explicit-new-lineage or prior-lineage authority model
- **THEN** authorization is refused regardless of parity agreement or operator confirmation, no fresh recovery state is created, and no payload or parity target is written

#### Scenario: A cloned candidate or parity identity is ambiguous

- **WHEN** two candidates share identity observations or more than one parity device could fill the role
- **THEN** writable assembly and destructive selection are refused while bounded read-only inspection remains available

#### Scenario: Q coding positions are lost

- **WHEN** two data members are absent and surviving P/Q evidence cannot establish historical coding positions
- **THEN** automatic decode and writable assembly are refused and the plan reports that guessing coefficients is unsafe

### Requirement: Fresh recovery state records a new baseline and audit
<!-- dwv:req req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:refines req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

After an authorized completed all-data/single-parity recovery, the implementation SHALL be able to create a bounded semantic manifest with the selected validated topology, generation zero, a new checksum-baseline obligation, and a metadata-loss audit record. Certificate-receipt-gated, P/Q, parity-rebuild, missing-member reconstruction, and new-lineage plans SHALL NOT create fresh state unless a later canonical change provides their required verified completion authority. The operation SHALL not depend on SQLite pages, row IDs, paths, or data-member payload metadata, and deletion of recovery state SHALL not delete direct payload files.

#### Scenario: An authorized fresh state is created

- **WHEN** a currently executable evidence gate succeeds for a plan that creates recovery state
- **THEN** the resulting manifest has an active topology carrying array identity, stable slots, roles, coding positions, assignment instances, generations, and evidence plus a checksum-baseline-required audit with no claim that absent integrity records are established

#### Scenario: Authorization is insufficient

- **WHEN** a caller presents an unavailable certificate receipt, ambiguous mismatch, incomplete scan, conflicting replica set, or refused matrix case
- **THEN** fresh-state creation fails without mutating the semantic snapshot or payload stores

#### Scenario: Recovery state is recreated through an evaluation adapter

- **WHEN** an evaluation-only adapter initializes new storage from an authorized semantic manifest
- **THEN** export returns bounded semantic header data and direct regular-file payload bytes remain unchanged and readable

### Requirement: Dry-run reporting is bounded and portable
<!-- dwv:req req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded -->

The metadata-loss dry run SHALL enumerate the complete matrix with stable case IDs, dispositions, evidence requirements, confirmation requirements, payload-write policy, and certificate-receipt availability. It SHALL omit payload bytes, paths, storage-engine handles, OS/frontend types, and unbounded operator text. The dry run SHALL be diagnostic and SHALL not authorize `CLEAN`, writable assembly, or payload mutation.

#### Scenario: The matrix dry run is requested on a portable host

- **WHEN** the portable example is run against the local workspace
- **THEN** it prints every matrix case deterministically, reports certificate-receipt-gated cases as unavailable, and exits without opening or changing a data or recovery store

#### Scenario: Platform integration is unavailable

- **WHEN** a Linux frontend, macOS bridge, or other platform integration is absent
- **THEN** the portable planner, tests, and regular-file evaluation remain usable and make no platform conformance claim
