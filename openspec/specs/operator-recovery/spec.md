# operator-recovery Specification

## Purpose
This capability composes current identity, verification, recovery, integrity, admission, and publication decisions into one bounded production operator workflow without becoming a second owner of those decisions.

## Requirements

### Requirement: Declarative array policy locates but does not authorize
<!-- dwv:req req.operator-recovery.declarative-array-policy-locates-but-does-not-authorize -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->
<!-- dwv:requires req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity -->

Production operator commands SHALL accept one bounded versioned declarative array policy containing operator-supplied labels, stable member locators, expected identity observations, desired topology bindings, and frontend selection needed to address an array without repeated flags. The policy and its paths SHALL remain operator-supplied desired context rather than current topology, recovery, integrity, or publication authority. Every assessment or action SHALL re-observe candidate locators and compose the canonical identity, topology, recovery, verification, integrity, and admission decisions. An uncertain consequential effect SHALL NOT be classified as definite failure or become an automatic retry condition.

#### Scenario: Policy and current observations agree

- **WHEN** the policy locates one unambiguous current member for every desired assignment and all owning validators accept the resulting snapshot
- **THEN** the operator workflow may compose those current facts into assessment or action without treating policy text as evidence

#### Scenario: Policy path is stale or replaced

- **WHEN** a candidate path is missing, resolves to another identity, or disagrees with its expected observation
- **THEN** the workflow reports the current discrepancy and refuses any stronger action before payload or recovery mutation

#### Scenario: Two locators resolve to one identity

- **WHEN** two policy entries currently resolve to the same underlying store identity
- **THEN** both observations remain visible as an ambiguous clone or alias and neither is silently selected for writable assembly

#### Scenario: Stronger action lacks authority

- **WHEN** an observation succeeds but current proof does not authorize the requested mutation or publication
- **THEN** the action result is semantic refusal or blockage with the exact causal reason and the observation is not commanded failure

#### Scenario: Commit effect is unknown

- **WHEN** DiskWeave cannot prove whether a consequential recovery commit occurred
- **THEN** the result is reconciliation-required and remains distinct from definite rejection and definite operational failure

### Requirement: Production assessment is observational and multidimensional
<!-- dwv:req req.operator-recovery.production-assessment-is-observational-and-multidimensional -->
<!-- dwv:requires req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->
<!-- dwv:requires req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations -->
<!-- dwv:requires req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan -->
<!-- dwv:requires req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred -->
<!-- dwv:requires req.architecture-contract.operator-result-projections-preserve-consequential-meaning -->

The production status, members, scrub, and damage commands SHALL use current read-only identity observations, recovery inspection, and verification evidence without creating, initializing, migrating, or mutating recovery state or payload. The shared semantic result SHALL report lifecycle, access, member reconciliation, recovery classification, checksum state, parity state, verification completeness, damage disposition, remaining redundancy, publication state, and next action as separate fields.

The result SHALL also classify lineage as `accepted`, `ambiguous`, or `unproved`; custody continuity as `continuity-proved`, `gap-observed`, or `continuity-unproved`; and each configured parity role and bounded protected range as `current`, `prior`, `unprotected`, `indeterminate`, or `not-yet-interpretable`. These dimensions SHALL remain independent. Policy, path identity, topology recognition, a clean shutdown record, equation agreement, checksum validity, or evidence in any other dimension SHALL NOT silently strengthen lineage, custody, protection basis, integrity, recovery meaning, publication, or remaining-redundancy claims.

`accepted` lineage SHALL require independent current identity and recovery evidence for the exact array, topology, assignments, and coding positions. `continuity-proved` SHALL require evidence accepted by the owning continuity profile for the exact stores, interval, write paths, persistence and anti-rollback properties, and failure boundary. A known shutdown or reboot, detached or directly accessed member, boot through another environment, loss of enforceable writer ownership, untrusted ownership transition, or returned member without independent continuity proof SHALL be `gap-observed`; missing, stale, conflicting, incomplete, unsupported, or unvalidated continuity evidence SHALL be `continuity-unproved`.

A `current` basis SHALL require evidence naming the exact active protection epoch, parity role, bounded range, participating data generations or captured snapshot, and committed basis transition with its required durability evidence. A `prior` basis SHALL require evidence naming an exact earlier authoritative protected generation and SHALL NOT be inferred for an already indeterminate range. `unprotected` SHALL require positive evidence that no admitted parity authority exists for the role and range. A crash, timeout, partial or uncertain transition, or conflicting recovery record that prevents naming one exact basis SHALL be `indeterminate`. Absent, corrupt, unreadable, unsupported, migration-required, incomplete, legacy, or otherwise insufficient basis evidence SHALL be `not-yet-interpretable`, not any stronger basis.

The result SHALL aggregate coverage by parity role and basis with bounded range counts and byte totals, preserve bounded exact exceptions, and avoid an array-wide protection or redundancy label stronger than any included range. A command SHALL NOT scan payload solely to manufacture lineage, custody, or protection authority. An explicitly requested scrub may report algebraic and integrity observations, but those observations SHALL NOT by themselves change any authority classification.

A degraded, post-gap, or recovery-needed array that is inspected successfully SHALL remain a successful observation command. Semantic refusal, blockage, unsupported capability, uncertainty that requires reconciliation, and operational failure SHALL remain distinct outcomes rather than one health bit or generic error. Every result SHALL state that observation authorizes no publication, data/parity write, repair, reconstruction, protection-epoch transition, currentization, historical continuity, or destructive recovery action, and SHALL name the next independently gated action or the evidence blocker.

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

- **WHEN** any production assessment completes successfully, reports a blocker, or returns uncertainty that requires reconciliation
- **THEN** recovery artifacts, payload bytes, publication state, writer claims, protection epochs, and authority records remain unchanged and the result states that no stronger action was authorized

#### Scenario: Human and structured renderings are requested

- **WHEN** the same semantic result is rendered in human and versioned structured modes
- **THEN** human and structured results each preserve the outcome, lineage, custody, role-local basis coverage, causal dimensions, counts, bounded findings, non-authorization statement, and next action while process status is mapped deterministically from the outcome

### Requirement: Recovery preview is read-only and apply re-establishes authority
<!-- dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

Production recovery preview SHALL report the current metadata-loss disposition, missing proof, consequences, non-action data/parity-write policy, baseline consequence, and a deterministic bounded proposal identity without mutation. Apply SHALL validate the proposal, reacquire current claims, reassess identity, topology, recovery state, and current evidence, and re-establish every canonical authorization prerequisite. It SHALL invoke a canonical metadata-loss transition only from those current facts. Stale, malformed, unrelated, ambiguous, conflicting, unreadable, incomplete, unsupported, or non-executable facts SHALL refuse before fresh authority or a data/parity write. Total loss of recovery metadata SHALL NOT be treated as executable recovery merely because policy and parity agree.

#### Scenario: Recovery is previewed

- **WHEN** the operator requests recovery without apply
- **THEN** the result explains the current case, executability, required evidence, consequences, and proposal identity while payload and recovery artifacts remain unchanged

#### Scenario: Proposal is stale at apply

- **WHEN** any proposal input or current identity, topology, recovery, or verification fact differs before apply
- **THEN** apply refuses before recovery or a data/parity write and a new preview is required

#### Scenario: Policy and parity agree after total metadata loss

- **WHEN** recovery state is absent or untrusted and exhaustive verification shows only that policy-selected payload members satisfy the policy-selected parity equations
- **THEN** preview reports non-executable new-lineage creation and apply refuses without creating recovery state or writing payload

#### Scenario: Supported recovery is applied

- **WHEN** a canonical matrix case is currently executable from independently established current authority and apply revalidates every prerequisite
- **THEN** canonical authorization performs only that case's bounded transition and returns its resulting current assessment without inferring authority from policy or parity agreement

#### Scenario: Recovery commit observation is uncertain

- **WHEN** recovery state has an unresolved write-recovery-record outcome or a future authorized replacement cannot prove whether proposed durable state became current
- **THEN** preview or apply reports that reconciliation is required, does not retry automatically, and preserves enough prior and proposed evidence for explicit reconciliation

### Requirement: Production commands share one semantic result boundary
<!-- dwv:req req.operator-recovery.production-commands-share-one-semantic-result-boundary -->
<!-- dwv:requires req.architecture-contract.operator-result-projections-preserve-consequential-meaning -->

After a production command has parsed and entered operator semantics, success, semantic refusal, blocked or unavailable state, unsupported capability, reconciliation-required uncertainty, and operational failure SHALL all be represented by the same versioned semantic result contract. Human and structured renderings SHALL preserve the same command, outcome, reason code, reason, and available multidimensional assessment. Usage and input-shape errors MAY remain parser diagnostics outside that contract.

#### Scenario: A semantic start refusal is rendered

- **WHEN** start reaches operator semantics but a current admission prerequisite is unavailable
- **THEN** the shared result reports the refusal or blockage outcome and maps process status deterministically without substituting an unrelated error envelope

#### Scenario: A frontend capability is unsupported

- **WHEN** current portable topology is valid but the selected frontend cannot publish its profile
- **THEN** the shared result reports unsupported capability without changing the topology classification or omitting available assessment context

### Requirement: Start composes admission and actual publication
<!-- dwv:req req.operator-recovery.start-composes-admission-and-actual-publication -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

Production start SHALL re-observe and validate current identity, topology, recovery, checksum, and admission prerequisites before invoking the selected frontend. Default start SHALL require current read-write admission. An explicit read-only request SHALL succeed only through a supported read-only publication path and SHALL NOT relabel missing recovery authority, incomplete verification, invalid checksum coverage, or other read-write blockage as read-only service. The command SHALL report the array online only after the frontend reports successful owned endpoint publication. A valid topology outside a frontend's narrow profile SHALL remain unsupported rather than topology-invalid.

#### Scenario: Admission succeeds and frontend publishes

- **WHEN** every current read-write prerequisite succeeds and the selected frontend publishes its owned endpoint successfully
- **THEN** start reports the exact published endpoint and remains attached until the frontend shuts down or fails

#### Scenario: Portable admission succeeds but publication fails

- **WHEN** portable admission succeeds and the frontend cannot publish or reports reconciliation-required state
- **THEN** start returns the frontend outcome and does not report an online or read-write endpoint

#### Scenario: Read-only publication is requested without support

- **WHEN** the operator requests read-only start and no supported frontend path can enforce the required read-only service semantics
- **THEN** start reports unsupported without publishing an endpoint or converting read-write blockage into success

#### Scenario: Valid topology exceeds frontend profile

- **WHEN** current topology is valid but wider than the selected frontend's supported publication profile
- **THEN** start reports a frontend capability limitation and preserves the topology as valid
