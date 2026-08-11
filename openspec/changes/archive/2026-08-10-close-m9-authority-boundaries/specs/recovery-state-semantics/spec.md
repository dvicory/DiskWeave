## MODIFIED Requirements

### Requirement: Recovery adapters report conservative commit observations
<!-- dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

The replaceable adapter seam SHALL distinguish durable, rejected, lost, and corrupt commit observations. The semantic store SHALL accept a transaction as a protocol fact only when the adapter reports durable commitment. A rejected commit SHALL leave the exact prior semantic state authoritative. Before a concrete writable adapter attempts to publish proposed state, it SHALL durably preserve enough exact prior/proposed semantic evidence to resolve an interrupted or uncertain commit after process-local state is released. After a lost or corrupt acknowledgement for an operation that may have committed, process-local belief SHALL authorize no dependent mutation; the caller SHALL release that belief and reopen the durable artifact through current semantic validation. Reconciliation SHALL compare the complete validated durable semantic state, not generation alone: exact prior state preserves prior authority, exact proposed state may establish proposed authority, and any unreadable, unsupported, or semantically different state remains reconciliation-required.

#### Scenario: A checkpoint acknowledgement is lost

- **WHEN** a backend may have committed a checkpoint but cannot prove the resulting semantic state
- **THEN** the semantic disposition is reconciliation-required, no clean or writable claim is inferred, and dependent mutation stops

#### Scenario: Reopen finds the exact prior state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete prior state
- **THEN** the prior state remains authoritative and the unacknowledged proposal is not assumed committed

#### Scenario: Reopen finds the exact proposed state

- **WHEN** reconciliation reopens and validates durable state semantically identical to the complete proposed state
- **THEN** the proposed state may become authoritative under the same validation required for an ordinarily opened current state

#### Scenario: Reopen finds neither exact state

- **WHEN** reopened state differs semantically from both prior and proposed state, cannot be read, or cannot be interpreted currently
- **THEN** reconciliation remains required and neither candidate authorizes dependent mutation
