## MODIFIED Requirements

### Requirement: Writes follow the reference transaction and update single XOR parity
<!-- dwv:req req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity -->
<!-- dwv:requires req.dirty-integrity-invalidation.write-recovery-record-precedes-data-parity-write -->
<!-- dwv:requires req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic -->
<!-- dwv:requires req.recovery-state-semantics.data-parity-write-requires-write-recovery-record -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts -->
<!-- dwv:requires req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

This requirement owns service orchestration and conformance only. A protected write SHALL compose one complete validated semantic mutation unit (which MAY be a request-decomposition unit) with the canonical dirty-region mapping, transaction-owned coded-range authority for every mutation and relevant basis read, the durable write-recovery-record owner, atomic recovery transaction, delegated reference-transaction relation, single-XOR computation, and exact-range store operations. The service SHALL present complete owner-approved admission before dependent basis I/O or protected mutation, preserve each owner's result, and keep relevant basis observations coherent through consumption or discard/reconciliation before release. No protected member data/parity write SHALL occur until both coded/resource admission and write-recovery-record admission succeed. It SHALL NOT restate or reinterpret coded, transaction, dirty, recovery, XOR, store, or release predicates.

#### Scenario: Partial write requires read-modify-write

- **WHEN** a write covers part of a parity extent
- **THEN** the service composes required basis reads, reference-equivalent XOR, and exact writes only after coded/resource and write-recovery owners admit the unit, preserving basis coherence through consumption or discard/reconciliation

#### Scenario: Full overwrite is aligned

- **WHEN** a write fully covers the required data and parity extent
- **THEN** the service may avoid old-data reads only under the XOR contract while preserving every owner-approved coded, transaction, dirty, recovery, persistence, and store boundary

#### Scenario: Basis coherence is required through consumption

- **WHEN** a relevant basis observation remains needed after authority would otherwise release
- **THEN** the service holds it coherently through the consuming mutation or discards/reconciles it before release and never uses a release-then-use gap
### Requirement: Durable completion and recovery CLEAN require persistence evidence
<!-- dwv:req req.healthy-portable-io.durable-completion-and-recovery-clean-require-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-requires-persistence-evidence -->
<!-- dwv:requires req.dirty-integrity-invalidation.recovery-clean-captures-a-closed-mutation-set -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

This requirement owns service composition and conformance only. The service SHALL preserve normalized durability intent and compose owner-approved dirty/recovery observations, exact lifecycle dispositions, store watermarks and persistence evidence, dirty/checksum coverage, recovery generation/topology and reopen reconciliation, one capture-wide CLEAN-owner `Accepted`/`Rejected` decision, and the closed mutation set. It may report durable completion or commit recovery state `CLEAN` only when the owning requirements accept current-generation evidence and permit the exact selected clear. Unsupported durability requirements SHALL be rejected or reported at the explicitly established weaker scope. CLEAN-capture uncertainty remains distinct from exact external `ReleaseAllowed` supplied by the canonical lifecycle-release requirement; it neither supplies nor negates that authorization. The service SHALL NOT restate or weaken owner predicates.

#### Scenario: All required owners accept completion evidence

- **WHEN** writes are complete, lifecycle owners provide dispositions, the CLEAN owner accepts the closed set, the durable post-capture cut is satisfied, and store/recovery/dirty owners accept matching evidence
- **THEN** the service may report durable completion and commit `CLEAN` only for the proven selected regions

#### Scenario: An owner rejects completion evidence

- **WHEN** any owner rejects missing, volatile, stale, partial, future, or mismatched evidence, or the CLEAN owner does not accept the capture
- **THEN** the service reports the conservative result and leaves affected state dirty, stale, or indeterminate

#### Scenario: CLEAN disposition is Unknown

- **WHEN** recovery or an adapter reports an unclassifiable `Unknown` commit or reopen disposition
- **THEN** the service preserves dirty/indeterminate state and bounded capture evidence, obtains authoritative reconciliation before `CLEAN` or cleanup, and independently consumes exact external `ReleaseAllowed` when the canonical lifecycle-release owner authorizes operation or coded-claim release
