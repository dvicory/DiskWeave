## MODIFIED Requirements

### Requirement: Recovery preview is read-only and apply re-establishes authority
<!-- dwv:req req.operator-recovery.recovery-preview-is-read-only-and-apply-re-establishes-authority -->
<!-- dwv:requires req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative -->
<!-- dwv:requires req.metadata-loss-recovery.evidence-gates-control-recovery-authorization -->
<!-- dwv:requires req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit -->
<!-- dwv:requires req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

Production recovery preview SHALL report the current metadata-loss disposition, missing proof, consequences, non-action payload-write policy, baseline consequence, and a deterministic bounded proposal identity without mutation. Apply SHALL validate the proposal, reacquire current claims, reassess identity, topology, recovery state, and current evidence, and re-establish every canonical authorization prerequisite. It SHALL invoke a canonical metadata-loss transition only from those current facts. Stale, malformed, unrelated, ambiguous, conflicting, unreadable, incomplete, unsupported, or non-executable facts SHALL refuse before fresh authority or payload mutation. Total loss of recovery metadata SHALL NOT be treated as executable recovery merely because policy-selected members satisfy policy-selected parity. Recovery-store reconciliation-required state SHALL remain reconciliation-required through the operator result boundary and SHALL NOT be relabeled as generic blockage or operational failure.

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
