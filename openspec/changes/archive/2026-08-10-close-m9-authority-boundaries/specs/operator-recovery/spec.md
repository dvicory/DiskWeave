## ADDED Requirements

### Requirement: Production commands share one semantic result boundary
<!-- dwv:req req.operator-recovery.production-commands-share-one-semantic-result-boundary -->

After a production command has parsed and entered operator semantics, success, semantic refusal, blocked or unavailable state, unsupported capability, reconciliation-required uncertainty, and operational failure SHALL all be represented by the same versioned semantic result contract. Human and structured renderings SHALL preserve the same command, outcome, reason code, reason, and available multidimensional assessment. Usage and input-shape errors MAY remain parser diagnostics outside that contract.

#### Scenario: A semantic start refusal is rendered

- **WHEN** start reaches operator semantics but a current admission prerequisite is unavailable
- **THEN** human and structured modes render the shared result with the same refusal or blockage outcome and deterministic process status rather than substituting an unrelated error envelope

#### Scenario: A frontend capability is unsupported

- **WHEN** current portable topology is valid but the selected frontend cannot publish its profile
- **THEN** the shared result reports unsupported capability without changing the topology classification or omitting available assessment context

## MODIFIED Requirements

### Requirement: Recovery preview is read-only and apply re-establishes authority
<!-- dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

Production recovery preview SHALL report the current metadata-loss disposition, missing proof, consequences, non-action payload-write policy, baseline consequence, and a deterministic bounded proposal identity without mutation. Apply SHALL validate the proposal, reacquire current claims, reassess identity, topology, recovery state, and current evidence, and re-establish every canonical authorization prerequisite. It SHALL invoke a canonical metadata-loss transition only from those current facts. Stale, malformed, unrelated, ambiguous, conflicting, unreadable, incomplete, unsupported, or non-executable facts SHALL refuse before fresh authority or payload mutation. Total loss of recovery metadata SHALL NOT be treated as executable recovery merely because policy-selected members satisfy parity equations; the current product SHALL preview the required new-lineage operation and refuse apply until that separately specified operation exists.

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

- **WHEN** a future authorized replacement of recovery state cannot prove whether the proposed durable state became current
- **THEN** apply returns reconciliation-required, does not retry automatically, and preserves enough prior and proposed evidence for explicit reconciliation
