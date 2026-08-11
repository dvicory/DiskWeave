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

The production status, members, scrub, and damage commands SHALL use current read-only identity observations, recovery inspection, and verification evidence without creating, initializing, migrating, or mutating recovery state or payload. The shared semantic result SHALL report lifecycle, access, member reconciliation, recovery classification, checksum state, parity state, verification completeness, damage disposition, remaining redundancy, publication state, and next action as separate fields. A degraded or recovery-needed array that is inspected successfully SHALL remain a successful observation command; semantic refusal, blockage, unsupported capability, reconciliation-required uncertainty, and operational failure SHALL remain distinct outcomes rather than one health bit or generic error.

#### Scenario: Missing recovery artifact is observed

- **WHEN** a production observation targets an array whose configured recovery artifact is absent
- **THEN** it reports absent recovery and recovery-required start state without creating the artifact or changing payload

#### Scenario: A member cannot be opened

- **WHEN** a configured member is currently unreadable or unavailable
- **THEN** that member and affected verification scope are reported explicitly and the command does not synthesize healthy, zero-filled, or repaired evidence

#### Scenario: Exhaustive verification finds readable disagreement

- **WHEN** every selected byte is readable but a protected equation does not match and current evidence does not identify a unique bad target
- **THEN** verification reports the exact bounded range as an unresolved conflict without guessing a repair target

#### Scenario: Exhaustive verification identifies a bad data target

- **WHEN** current checksum and parity evidence uniquely identify one data target as bad
- **THEN** damage reports the bounded range, data target, evidence disposition, and repair availability independently of general lifecycle or access state

#### Scenario: Human and structured renderings are requested

- **WHEN** the same semantic result is rendered for a human or as versioned structured output
- **THEN** both renderings preserve the same outcome, causal dimensions, counts, and bounded findings while process status is mapped deterministically from the outcome

### Requirement: Recovery preview is read-only and apply re-establishes authority
<!-- dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

Production recovery preview SHALL report the current metadata-loss disposition, missing proof, consequences, non-action payload-write policy, baseline consequence, and a deterministic bounded proposal identity without mutation. Apply SHALL validate the proposal, reacquire current claims, reassess identity, topology, recovery state, and current evidence, and re-establish every canonical authorization prerequisite. It SHALL invoke a canonical metadata-loss transition only from those current facts. Stale, malformed, unrelated, ambiguous, conflicting, unreadable, incomplete, unsupported, or non-executable facts SHALL refuse before fresh authority or payload mutation. Total loss of recovery metadata SHALL NOT be treated as executable recovery merely because policy and parity agree.

#### Scenario: Recovery is previewed

- **WHEN** the operator requests recovery without apply
- **THEN** the result explains the current case, executability, required evidence, consequences, and proposal identity while payload and recovery artifacts remain unchanged

#### Scenario: Proposal is stale at apply

- **WHEN** any proposal input or current identity, topology, recovery, or verification fact differs before apply
- **THEN** apply refuses before recovery or payload mutation and a new preview is required

#### Scenario: Policy and parity agree after total metadata loss

- **WHEN** recovery state is absent or untrusted and exhaustive verification shows only that policy-selected payload members satisfy the policy-selected parity equations
- **THEN** preview reports non-executable new-lineage creation and apply refuses without creating recovery state or writing payload

#### Scenario: Supported recovery is applied

- **WHEN** a canonical matrix case is currently executable from independently established current authority and apply revalidates every prerequisite
- **THEN** canonical authorization performs only that case's bounded transition and returns its resulting current assessment without inferring authority from policy or parity agreement

#### Scenario: Recovery commit observation is uncertain

- **WHEN** recovery state has unresolved commit intent or a future authorized replacement cannot prove whether proposed durable state became current
- **THEN** preview or apply returns reconciliation-required, does not retry automatically, and preserves enough prior and proposed evidence for explicit reconciliation

### Requirement: Production commands share one semantic result boundary
<!-- dwv:req req.operator-recovery.production-commands-share-one-semantic-result-boundary -->

After a production command has parsed and entered operator semantics, success, semantic refusal, blocked or unavailable state, unsupported capability, reconciliation-required uncertainty, and operational failure SHALL all be represented by the same versioned semantic result contract. Human and structured renderings SHALL preserve the same command, outcome, reason code, reason, and available multidimensional assessment. Usage and input-shape errors MAY remain parser diagnostics outside that contract.

#### Scenario: A semantic start refusal is rendered

- **WHEN** start reaches operator semantics but a current admission prerequisite is unavailable
- **THEN** human and structured modes render the shared result with the same refusal or blockage outcome and deterministic process status rather than substituting an unrelated error envelope

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
