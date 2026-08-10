## ADDED Requirements

### Requirement: Current baseline completion is persisted and exact
<!-- dwv:req req.checksum-plane.current-baseline-completion-is-persisted-and-exact -->
<!-- dwv:requires req.checksum-plane.checksum-coverage-names-targets-and-extents -->
<!-- dwv:requires req.checksum-plane.validity-is-generation-bound -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->

A checksum baseline SHALL identify its digest profile, checksum-set generation, topology epoch, complete required target and extent set, each target content generation, and each record's validity and typed fence evidence in durable recovery state. A current baseline is complete only when every required data and parity extent appears exactly once with a supported active profile and set, matches the current topology and target identity, and carries current `VALID` evidence for the exact bytes. Missing, duplicate, stale, unknown, unsupported, wrong-topology, wrong-target, wrong-profile, wrong-set, invalid, or out-of-range records SHALL remain absent, partial, or invalid rather than complete. Reopen SHALL derive completion from the validated persisted semantic state; a persisted completion bit, process-local job success, parity cleanliness, or matching current digest alone SHALL NOT establish completeness.

#### Scenario: Baseline work is interrupted

- **WHEN** only a strict subset of required current extents has durably valid records before interruption
- **THEN** reopen reports the same baseline as partial and does not promote it to complete

#### Scenario: Complete current coverage reopens

- **WHEN** every required current target extent has one valid correctly bound persisted record and the semantic state reopens successfully
- **THEN** the checksum owner reconstructs a complete current baseline from those records

#### Scenario: Persisted evidence is no longer current

- **WHEN** a record names another topology, target, profile, set, content generation, unsupported digest, invalid fence, or duplicate extent
- **THEN** that record does not contribute to completion and the baseline remains partial or invalid

#### Scenario: A post-recovery baseline is calculated

- **WHEN** current bytes are read and committed as valid checksum evidence after metadata-loss recovery
- **THEN** the baseline may establish current coverage but SHALL remain labeled as newly calculated evidence rather than historical correctness proof
