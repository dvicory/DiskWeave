## MODIFIED Requirements

### Requirement: Identity and topology ambiguity fails closed
<!-- dwv:req req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:refines req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity -->

Metadata-loss recovery SHALL preserve all candidates and require independently established, unambiguous topology, profile, coding-position, and member-identity authority before recovering a prior array lineage or permitting writable assembly. Declarative policy and algebraic parity agreement SHALL NOT manufacture historical identity or topology authority. Duplicate clone evidence, ambiguous parity candidates, lost historical Q coding positions, and unresolved assignment mappings SHALL produce read-only or refused outcomes. When all data survive but prior recovery authority is unavailable, the operation SHALL be classified as explicit new-lineage creation rather than recovery of the policy-described topology.

#### Scenario: All data survive but prior recovery state is unavailable

- **WHEN** all payload members named by declarative policy are readable and parity equations match but independent current evidence cannot establish the prior array identity, assignments, and topology epoch
- **THEN** the plan is non-executable recovery, requires explicit new-lineage creation with fresh independently verified parity and checksum baselines, preserves old parity as evidence or targets new parity separately, and creates no fresh recovery state

#### Scenario: A cloned candidate or parity identity is ambiguous

- **WHEN** two candidates share identity observations or more than one parity device could fill the role
- **THEN** writable assembly and destructive selection are refused while bounded read-only inspection remains available

#### Scenario: Q coding positions are lost

- **WHEN** two data members are absent and surviving P/Q evidence cannot establish historical coding positions
- **THEN** automatic decode and writable assembly are refused and the plan reports that guessing coefficients is unsafe
