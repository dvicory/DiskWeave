## MODIFIED Requirements

### Requirement: Requests have validated frontend-neutral semantics
<!-- dwv:req req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->

The system SHALL represent each block request with stable request and frontend identities, target slot, captured topology epoch, operation, byte range, optional generational buffer handle, submission sequence, ordering intent, and durability intent. Byte ranges SHALL use checked end arithmetic. The operation vocabulary SHALL support read, write, flush, write-zeroes, and discard while explicitly rejecting unsupported zoned operations. From admission through terminal reconciliation, operation ownership and terminal evidence SHALL preserve and expose the exact canonical request fields rather than substituting positional, adapter-local, or internally generated identities.

#### Scenario: A valid aligned write is normalized

- **WHEN** a frontend submits a non-overflowing write with its target, epoch, buffer, sequence, and durability intent
- **THEN** normalization preserves every semantic field without an operating-system tag, pointer, runtime future, or database type

#### Scenario: A byte range overflows

- **WHEN** offset plus length cannot be represented safely
- **THEN** normalization returns a stable range error before backend admission

#### Scenario: A request has an invalid buffer relationship

- **WHEN** a read or write lacks the required generational buffer, or a flush supplies a data buffer
- **THEN** normalization rejects the request without submitting backend I/O

#### Scenario: An admitted request reaches a terminal disposition

- **WHEN** a canonical request completes, fails, becomes uncertain, or finishes reconciliation after delivery interest is abandoned
- **THEN** terminal evidence identifies its exact frontend, request, target slot, topology epoch, operation, range, buffer token when present, submission sequence, ordering intent, and durability intent
