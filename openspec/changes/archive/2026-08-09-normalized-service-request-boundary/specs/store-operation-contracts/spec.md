## MODIFIED Requirements

### Requirement: Operation slots own backend lifetimes and generations
<!-- dwv:req req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

Every admitted logical operation SHALL reserve a generation-bearing operation slot before backend submission. The slot SHALL retain the exact canonical normalized request, own its frontend buffer token, frontend tags, child-operation identities, submitted watermarks, drain state, and terminal evidence, and expose the canonical request alongside terminal state. It SHALL become reclaimable only after all children are terminal and required reconciliation is recorded. Frontend abandonment SHALL remove completion-delivery interest only and SHALL NOT release the slot or any owned resource early.

#### Scenario: A slot is reused after a terminal operation

- **WHEN** all child operations are terminal and reconciliation is complete
- **THEN** the slot may be reclaimed and a later reservation receives a generation that prevents stale completions from addressing its resources

#### Scenario: A stale completion arrives

- **WHEN** completion references a released slot generation
- **THEN** the adapter rejects it before resource lookup and never applies it to reused memory

#### Scenario: A child operation completes twice

- **WHEN** a terminal child receives a duplicate completion
- **THEN** the duplicate is recorded and ignored without changing the semantic result or reclaiming resources early

#### Scenario: Canonical identity reaches terminal evidence

- **WHEN** an admitted operation reaches completion, failure, uncertainty, or reconciled terminal state
- **THEN** its slot evidence exposes the exact frontend, request, target slot, topology epoch, operation, range, buffer token, submission sequence, ordering intent, and durability intent admitted for that operation

#### Scenario: Completion interest is abandoned while a child remains active

- **WHEN** the frontend abandons completion delivery after backend submission but before every child is terminal
- **THEN** completion delivery is suppressed while the slot, canonical request, buffer, child identity, and reconciliation obligations remain owned until safe reclamation
