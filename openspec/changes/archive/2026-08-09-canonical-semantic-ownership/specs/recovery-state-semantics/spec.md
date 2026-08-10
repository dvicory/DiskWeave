## MODIFIED Requirements

### Requirement: Home mutation requires durable dirty and integrity invalidation intent
<!-- dwv:req req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

The semantic store SHALL atomically persist the affected dirty regions and stale integrity extents at one generation before a caller may rely on that transaction as the owner's durable invalidation intent. Dirty and stale generations SHALL survive in-memory process loss until a later explicit recovery transaction changes them. A rejected or generation-mismatched transaction leaves the prior snapshot authoritative. A lost, corrupt, or indeterminate commit observation establishes no authoritative resulting snapshot for the caller, provides no permission for protected home mutation, and requires reconciliation through the recovery commit-observation contract.

#### Scenario: A valid integrity extent is touched

- **WHEN** one recovery transaction marks a region dirty and invalidates a valid extent
- **THEN** the resulting snapshot records both changes at one committed recovery generation and no valid digest remains authoritative for that extent

#### Scenario: Recovery intent commit is rejected before authoritative commit

- **WHEN** the transaction is rejected or generation-mismatched before authoritative commit
- **THEN** the proposed transaction does not become authoritative, the prior snapshot remains authoritative, and the caller receives no permission for home mutation

#### Scenario: Recovery intent commit observation is lost, corrupt, or indeterminate

- **WHEN** commitment may have occurred but its acknowledgement is lost, corrupt, or indeterminate
- **THEN** neither the prior nor proposed resulting snapshot may be assumed authoritative for protected mutation, no home-mutation permission exists, and reconciliation through the recovery commit-observation contract is required

### Requirement: Clean and valid claims require typed fence evidence
<!-- dwv:req req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->

The recovery store SHALL own the admissibility of typed evidence used for durable clean, valid-integrity, and clean-session claims. Evidence SHALL identify the store incarnation, ordering domain, accepted and synchronized-through watermarks, topology epoch, affected range or region, capability evidence, and relevant generations. A claim SHALL be rejected when required evidence is missing, volatile, future, stale, partial, cross-store, or mismatched.

#### Scenario: A region is cleared after valid typed evidence

- **WHEN** a dirty region has admissible covering evidence and one recovery transaction records a clean checkpoint
- **THEN** the region may become clean and the checkpoint generation is durably recorded

#### Scenario: Volatile completion is supplied as durable authority

- **WHEN** a caller attempts a clean or valid claim using volatile or unknown persistence evidence
- **THEN** the mutation is rejected and the affected state remains dirty, stale, or indeterminate

### Requirement: SQLite remains an evidence-driven adapter decision
<!-- dwv:req req.recovery-state-semantics.sqlite-remains-an-evidence-driven-adapter-decision -->

The project SHALL keep SQLite, journal mode, synchronization, checkpoint policy, connection topology, schema, and migration details behind the semantic interface. Production selection SHALL require evaluation across the declared durability and reset cases and SHALL NOT follow from folklore or a successful process-local commit alone.

#### Scenario: Candidate SQLite modes are compared

- **WHEN** candidate journal and synchronization configurations run through the declared crash and reset cases
- **THEN** the evidence records each result while unsupported or untested durability claims remain unavailable

### Requirement: Evaluation fixtures cover candidate durability and reset boundaries
<!-- dwv:req req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries -->
<!-- dwv:requires req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects -->

Evaluation SHALL provide deterministic fixtures for candidate journal modes, synchronization modes, checkpoint policies, process reset, VM reset, power loss, commit rejection, lost commit acknowledgement, missing state, main-state corruption, and journal-state corruption. Each fixture SHALL state the conservative semantic disposition and evidence still required; fixture presence SHALL NOT select a production SQLite mode.

#### Scenario: A dirty-intent commit is rejected or uncertain

- **WHEN** the simulator reports that the recovery commit did not become durably known
- **THEN** the expected disposition forbids protected home mutation and requires reconciliation

#### Scenario: Recovery state is missing or corrupt after reset

- **WHEN** the recovery adapter cannot validate its state or journal
- **THEN** the expected disposition blocks writable assembly and permits only an explicit recovery plan
