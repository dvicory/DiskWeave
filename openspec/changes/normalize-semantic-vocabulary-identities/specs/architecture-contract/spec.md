## MODIFIED Requirements

### Requirement: Durable authority and uncertainty are not inferred
<!-- dwv:req req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred -->

Acknowledgement, completion, persistence, durability, recovery `CLEAN`, integrity validity, and recovery authorization SHALL remain distinct facts. A protected state change SHALL require its applicable durable prerequisite and current authority evidence. Failed, short, cancelled, abandoned, crashed, lost, stale, or uncertain effects SHALL remain visible and SHALL NOT be converted into recovery `CLEAN`, valid integrity, writable authorization, or proof that an irreversible effect did not occur.

#### Scenario: A protected write is acknowledged before a fence

- **WHEN** a caller observes write completion but covering persistence evidence is unavailable
- **THEN** the request may report completion only at the established scope and recovery state remains dirty, uncertain, or otherwise conservative

#### Scenario: A caller abandons an operation

- **WHEN** completion interest is dropped after submission
- **THEN** delivery may be suppressed, but media effects, resource ownership, and recovery reconciliation remain governed by the operation and are not rolled back by abandonment

### Requirement: Recovery and repair never promote algebraic possibility to authority
<!-- dwv:req req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority -->

Read reconstruction, rebuild, repair, rebaseline, and format interpretation SHALL use explicit current identity, geometry, generation, integrity, and durability evidence. Mathematical computability or a successful process return alone SHALL NOT authorize serving data as healthy, publishing a replacement, clearing dirty state, accepting a checksum, or interpreting an unknown format as writable.

#### Scenario: A missing member is mathematically reconstructible
- **WHEN** surviving bytes permit a candidate reconstruction but required authority or integrity evidence is missing, stale, or conflicting
- **THEN** the system refuses or marks the result degraded or uncertain and performs no unauthorized protected state change

#### Scenario: An unknown format is encountered
- **WHEN** a tool cannot establish the payload offsets, coding profile, topology, or recovery semantics of a format family
- **THEN** it refuses writable interpretation while allowing only bounded safe inspection or direct ordinary-payload access
