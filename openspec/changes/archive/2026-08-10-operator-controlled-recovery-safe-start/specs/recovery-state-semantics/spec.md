## MODIFIED Requirements

### Requirement: Semantic export and health are independent of storage engine layout
<!-- dwv:req req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout -->

The recovery boundary SHALL expose a bounded semantic snapshot/manifest, writable-open path, and observational inspection path. Observational inspection SHALL classify absent state, present supported state, corrupt or unreadable state, unsupported interpretation, migration required before writable use, and uncertain or reconciliation-required state without creating a database, acquiring writable ownership, running initialization or migration, repairing state, or changing the inspected artifact. Missing, corrupt, stale, unavailable, unsupported, migration-required, reconciliation-required, or failed-to-load recovery state SHALL remain an explicit conservative result and SHALL block new home mutations until the owning recovery semantics establish current authority. A caller SHALL NOT substitute generation zero, a clean snapshot, success, or a successful trace for failed recovery access. Export SHALL not expose SQLite pages, row IDs, or implementation pointers.

#### Scenario: Recovery state is missing

- **WHEN** observational inspection finds no durable recovery artifact
- **THEN** it reports absent state, creates no artifact, and callers receive a conservative recovery decision rather than an implicit clean state

#### Scenario: Recovery state cannot be loaded

- **WHEN** observational inspection finds corrupt, unreadable, stale, locked, unavailable, unsupported, migration-required, or reconciliation-required state
- **THEN** it preserves the exact conservative classification without initialization or migration and callers do not continue with generation zero

#### Scenario: Supported recovery state is inspected

- **WHEN** observational inspection validates a supported current semantic manifest
- **THEN** it returns the bounded semantic manifest while leaving payload, parity, recovery generation, semantic schema, migration state, and integrity state unchanged

### Requirement: Recovery adapters report conservative commit observations
<!-- dwv:req req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations -->

The replaceable adapter seam SHALL distinguish durable, rejected, lost, and corrupt commit observations. The semantic store SHALL accept a transaction as a protocol fact only when the adapter reports durable commitment. A rejected commit SHALL leave the exact prior semantic state authoritative. After a lost or corrupt acknowledgement for an operation that may have committed, process-local belief SHALL authorize no dependent mutation; the caller SHALL release that belief and reopen the durable artifact through current semantic validation. Reconciliation SHALL compare the complete validated durable semantic state, not generation alone: exact prior state preserves prior authority, exact proposed state may establish proposed authority, and any unreadable, unsupported, migration-required, reconciliation-required, or semantically different state SHALL remain reconciliation-required. Uncertain or failed observations SHALL not be converted into a clean checkpoint, valid digest, or writable authorization.

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
