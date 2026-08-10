## MODIFIED Requirements

### Requirement: Operations use exact normalized ranges and structured evidence
<!-- dwv:req req.volatile-media-simulator.operations-use-exact-normalized-ranges-and-structured-evidence -->
<!-- dwv:refines req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->

Simulator submissions SHALL use exact byte ranges, generation-bearing child operation identities, and the canonical store completion dispositions and persistence evidence. Short completion SHALL expose only the completed subset. Failed and uncertain outcomes SHALL preserve the evidence needed to decide whether retry is legal; the simulator SHALL not silently turn them into success.

#### Scenario: A short write is delivered

- **WHEN** a submitted write completes only a strict subset of its requested range
- **THEN** the simulator exposes that exact subset and no caller may treat the omitted bytes as completed

#### Scenario: Completion effect is uncertain

- **WHEN** completion delivery is lost after a write may have reached volatile or durable media
- **THEN** the simulator preserves the unknown effect and does not classify retry as safe automatically

### Requirement: Core safety invariants are executable
<!-- dwv:req req.volatile-media-simulator.core-safety-invariants-are-executable -->

The simulator SHALL provide checks for no durable mutation from discarded volatile writes, exact range bounds, no successful incomplete completion, and stable serialized replay. Transaction, dirty-region, integrity, and parity-envelope invariants SHALL remain the responsibility of their owning capabilities rather than being inferred from this media primitive.

#### Scenario: A power-loss schedule is replayed

- **WHEN** the same initial media, operation sequence, and fault schedule are applied twice
- **THEN** durable bytes, visible bytes, completions, and serialized trace are identical
