## ADDED Requirements

### Requirement: Offline rebuild progress is durable semantic authority

Recovery state SHALL represent an offline rebuild with a stable rebuild identifier, source array/topology/recovery generation, missing stable slot and coding position, replacement assignment instance and store identity, protected geometry, first-unprocessed-byte cursor, and lifecycle state. Cursor advancement and verified completion SHALL be generation-checked recovery mutations, bounded in semantic export, and independent of SQL rows, file paths, runtime handles, or executor objects.

#### Scenario: A rebuild cursor advances

- **WHEN** the replacement bytes through offset N have been read back, equation-verified, and durably fenced under the captured source topology
- **THEN** one atomic recovery transaction advances the first-unprocessed-byte cursor to N and records the matching fence evidence

#### Scenario: A stale rebuild transaction commits

- **WHEN** another recovery transaction or topology change invalidates the generation captured by a rebuild checkpoint
- **THEN** the checkpoint mutation fails atomically and cannot claim unverified replacement bytes durable

#### Scenario: A rebuild manifest is exported

- **WHEN** a current bounded semantic manifest is exported during an interrupted rebuild
- **THEN** it contains enough typed rebuild state to validate a later resume without exposing a path, SQLite layout, or process-local resource
