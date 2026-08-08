# explicit-transaction-machine Specification

## Purpose
This capability provides an auditable reference state machine that orders protected home-media actions around durable recovery intent, fences, checkpoints, abandonment, and crash reconciliation.
## Requirements
### Requirement: Transactions emit normalized semantic actions

The reference machine SHALL expose actions for range acquisition, durable dirty/integrity invalidation intent, reads, parity computation, writes, flush/fence, checkpoint/clear, and range release. Actions SHALL contain semantic ranges, generations, identities, and evidence rather than backend child-operation or runtime types.

#### Scenario: A write transaction starts

- **WHEN** a caller starts a protected write with affected regions and integrity extents
- **THEN** the machine emits a deterministic action sequence beginning with range acquisition and durable invalidation intent

#### Scenario: Backend fanout is used

- **WHEN** one semantic action is implemented by several child operations
- **THEN** the reference trace records one semantic action and leaves child completion accounting to the operation-slot layer

### Requirement: Durable intent precedes every protected home mutation

The machine SHALL not emit a home read/compute/write action that can mutate protected media until the recovery store reports a durable dirty and integrity-invalidation generation. A rejected, lost, or corrupt intent commit SHALL transition to a blocked or reconciliation-required state.

#### Scenario: Recovery intent commits durably

- **WHEN** dirty and stale mutations commit at the expected recovery generation
- **THEN** the machine may proceed to read, compute, and write actions under the captured topology

#### Scenario: Recovery intent commit fails

- **WHEN** the recovery adapter returns rejected, lost, or corrupt observation before home mutation
- **THEN** no protected write action is emitted and the transaction records the conservative failure

### Requirement: Clean and checkpoint claims require fence evidence

The machine SHALL emit flush/fence actions after writes and SHALL not emit checkpoint/clear or release actions until required child operations are terminal and fence evidence covers the affected stores, watermarks, topology epoch, dirty regions, and integrity generations.

#### Scenario: A fence covers the write set

- **WHEN** all writes complete and the required durable fence evidence matches the captured topology
- **THEN** the machine emits checkpoint/clear and only then releases its range guard

#### Scenario: Fence evidence is volatile or incomplete

- **WHEN** a completion lacks durable evidence or misses a store/range/watermark
- **THEN** the transaction remains dirty/reconciliation-required and cannot report clean

### Requirement: Failure, abandonment, and crash states are conservative

The machine SHALL distinguish failed, uncertain, abandoned, daemon-crashed, and reconciliation-required outcomes. Abandonment SHALL suppress frontend delivery interest only; it SHALL not cancel an irreversible home mutation or reclaim range/buffer ownership before backend and semantic reconciliation.

#### Scenario: A frontend abandons after intent

- **WHEN** the frontend abandons a transaction after durable intent but before checkpoint
- **THEN** the machine continues drain/reconciliation and never converts abandonment into cancellation or clean state

#### Scenario: A daemon crashes after a home write

- **WHEN** process state is lost after a write but before fence/checkpoint
- **THEN** restart recovery sees dirty/indeterminate state and does not infer a clean checkpoint from the missing action result

### Requirement: Reference traces are deterministic and implementation-independent

The machine SHALL emit versioned normalized action traces with stable error classes and semantic pre/post states. Equivalent implementations SHALL be compared by allowed trace normalization rather than private enum layout, batching, runtime, or database identity.

#### Scenario: The same plan is replayed

- **WHEN** the same plan and result sequence are applied twice
- **THEN** the semantic trace and terminal state are identical

#### Scenario: An invalid transition is attempted

- **WHEN** a caller supplies a result out of order or twice
- **THEN** the machine returns a stable transition error and leaves its prior state unchanged

