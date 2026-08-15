## MODIFIED Requirements

### Requirement: Production assessment is observational and multidimensional
<!-- dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->
<!-- dwv:requires req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->
<!-- dwv:requires req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred -->

The production status, members, scrub, and damage commands SHALL use current read-only identity observations, recovery inspection, and verification evidence without creating, initializing, migrating, or mutating recovery state or payload. The shared semantic result SHALL report lifecycle, access, member reconciliation, recovery classification, checksum state, parity state, verification completeness, damage disposition, remaining redundancy, publication state, and next action as separate fields.

The result SHALL also classify lineage as `accepted`, `ambiguous`, or `unproved`; custody continuity as `continuity-proved`, `gap-observed`, or `continuity-unproved`; and each configured parity role and bounded protected range as `current`, `prior`, `unprotected`, `indeterminate`, or `not-yet-interpretable`. These dimensions SHALL remain independent. Policy, path identity, topology recognition, a clean shutdown record, equation agreement, checksum validity, or evidence in any other dimension SHALL NOT silently strengthen lineage, custody, protection basis, integrity, recovery meaning, publication, or remaining-redundancy claims.

`accepted` lineage SHALL require independent current identity and recovery evidence for the exact array, topology, assignments, and coding positions. `continuity-proved` SHALL require evidence accepted by the owning continuity profile for the exact stores, interval, write paths, persistence and anti-rollback properties, and failure boundary. A known shutdown or reboot, detached or directly accessed member, boot through another environment, loss of enforceable writer ownership, untrusted ownership transition, or returned member without independent continuity proof SHALL be `gap-observed`; missing, stale, conflicting, incomplete, unsupported, or unvalidated continuity evidence SHALL be `continuity-unproved`.

A `current` basis SHALL require evidence naming the exact active protection epoch, parity role, bounded range, participating data generations or captured snapshot, and committed basis transition with its required durability evidence. A `prior` basis SHALL require evidence naming an exact earlier authoritative protected generation and SHALL NOT be inferred for an already indeterminate range. `unprotected` SHALL require positive evidence that no admitted parity authority exists for the role and range. A crash, timeout, partial or uncertain transition, or conflicting recovery record that prevents naming one exact basis SHALL be `indeterminate`. Absent, corrupt, unreadable, unsupported, migration-required, incomplete, legacy, or otherwise insufficient basis evidence SHALL be `not-yet-interpretable`, not any stronger basis.

The result SHALL aggregate coverage by parity role and basis with bounded range counts and byte totals, preserve bounded exact exceptions, and avoid an array-wide protection or redundancy label stronger than any included range. A command SHALL NOT scan payload solely to manufacture lineage, custody, or protection authority. An explicitly requested scrub may report algebraic and integrity observations, but those observations SHALL NOT by themselves change any authority classification.

A degraded, post-gap, or recovery-needed array that is inspected successfully SHALL remain a successful observation command. Semantic refusal, blockage, unsupported capability, reconciliation-required uncertainty, and operational failure SHALL remain distinct outcomes rather than one health bit or generic error. Every result SHALL state that observation authorizes no publication, payload mutation, repair, reconstruction, protection-epoch transition, currentization, historical continuity, or destructive recovery action, and SHALL name the next independently gated action or the evidence blocker.

#### Scenario: Complete recognized array is observed without continuity proof

- **WHEN** every configured member is readable and unambiguous and current recovery evidence accepts the exact lineage, but custody continuity and current-basis evidence are not proved
- **THEN** the command reports accepted lineage, continuity-unproved or gap-observed custody, and no current basis or current remaining-redundancy claim; insufficient basis records are not-yet-interpretable and the result names the missing evidence or independently gated next action

#### Scenario: A known custody gap is observed

- **WHEN** current observations establish a shutdown, reboot, detached or directly accessed member, untrusted ownership transition, or returned member without independent continuity proof
- **THEN** custody is gap-observed and prior current-protection claims are not reported as current merely because identity, topology, parity, or integrity observations still agree

#### Scenario: Basis evidence is range-local and mixed

- **WHEN** accepted evidence classifies different bounded ranges or parity roles as current, prior, unprotected, indeterminate, or not-yet-interpretable
- **THEN** human and structured results preserve each role-local coverage total and exact bounded exception without collapsing the array to one stronger protection state

#### Scenario: Missing recovery artifact is observed

- **WHEN** a production observation targets an array whose configured recovery artifact is absent
- **THEN** it reports absent recovery, unproved lineage or recovery authority as applicable, not-yet-interpretable protection basis, and recovery-required start state without creating the artifact or changing payload

#### Scenario: A member cannot be opened

- **WHEN** a configured member is currently unreadable or unavailable
- **THEN** that member and affected authority and verification scopes are reported explicitly and the command does not synthesize healthy, zero-filled, protected, or repaired evidence

#### Scenario: Exhaustive verification finds readable disagreement

- **WHEN** every selected byte is readable but a protected equation does not match and current evidence does not identify a unique bad target
- **THEN** verification reports the exact bounded range as an unresolved conflict without guessing a repair target or changing that range to a stronger basis

#### Scenario: Exhaustive verification identifies a bad data target

- **WHEN** current checksum and parity evidence uniquely identify one data target as bad
- **THEN** damage reports the bounded range, data target, evidence disposition, and repair availability independently of lineage, custody, protection basis, lifecycle, or access state

#### Scenario: Observation leaves stores unchanged

- **WHEN** any production assessment completes successfully, reports a blocker, or returns reconciliation-required uncertainty
- **THEN** recovery artifacts, payload bytes, publication state, writer claims, protection epochs, and authority records remain unchanged and the result states that no stronger action was authorized

#### Scenario: Human and structured renderings are requested

- **WHEN** the same semantic result is rendered for a human or as versioned structured output
- **THEN** both renderings preserve the same outcome, lineage, custody, role-local basis coverage, causal dimensions, counts, bounded findings, non-authorization statement, and next action while process status is mapped deterministically from the outcome
