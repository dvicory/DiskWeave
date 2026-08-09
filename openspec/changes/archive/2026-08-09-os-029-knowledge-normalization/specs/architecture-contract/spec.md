## MODIFIED Requirements

### Requirement: Architecture artifacts preserve the handoff contract

The existing architecture-contract is a migration baseline, not the desired permanent shape. Normalization SHALL preserve every genuinely cross-cutting product invariant while moving artifact-format, Phase-0 sequencing, OpenSpec workflow, and historical handoff machinery to durable historical/workflow records. Before any baseline clause is removed or relocated, the active normalization evidence SHALL name its old section/heading, disposition, current owner, and evidence or gap.

#### Scenario: A baseline clause is removed
- **WHEN** an OS-000-style artifact rule, dependency rule, or handoff clause is deleted from the canonical architecture contract
- **THEN** it has an explicit historical/work-program disposition and the final contract contains no replacement documentation machinery

### Requirement: Portable boundaries and safety invariants are normative

The normalized architecture-contract SHALL retain only timeless product invariants that cross capability boundaries: DiskWeave is block-level parity beneath conventional filesystems; ordinary data payloads remain independently readable and metadata-free; portable semantic behavior is independent of implementation libraries and platform frontends; and ambiguous identity, durability, recovery, integrity, or format evidence fails closed. Capability-local geometry, topology, transaction, checksum, repair, frontend, and persistence details SHALL remain in their dedicated canonical specs.

#### Scenario: A capability detail is relocated
- **WHEN** a clause is specific to topology, parity geometry, recovery state, checksums, store I/O, repair, frontend, or a platform bridge
- **THEN** the normalization audit names its dedicated canonical owner and the architecture constitution retains only the boundary invariant that prevents cross-layer confusion

### Requirement: Evidence claims are scope-accurate

Product and capability artifacts SHALL distinguish executable portable evidence from platform, device, power-loss, and hardware evidence and SHALL state unsupported claim boundaries. Detailed evidence governance and completion records belong to the canonical verification/evidence owner and repository evidence, not to permanent architecture workflow rules.

#### Scenario: A platform claim lacks evidence
- **WHEN** a criterion requires Linux, macOS, power loss, or hardware evidence that is unavailable
- **THEN** the criterion remains explicitly gated or unclaimed and no architecture text promotes portable tests into that claim

## ADDED Requirements

### Requirement: Normalization dispositions are durable and non-authoritative

The active normalization change SHALL produce a section-level disposition record for the prior architecture-contract and v0.8 handoff. The record SHALL preserve old identities and source digests, classify each removed or relocated normative unit, name its durable current owner, and record evidence or an explicit gap. The disposition record is migration evidence only; it SHALL NOT become a permanent architecture requirement or a parallel source of product semantics.

#### Scenario: The audit is complete
- **WHEN** every old architecture-contract requirement and every v0.8 section has a disposition
- **THEN** the final small constitution can omit historical workflow and documentation machinery without losing an unowned current invariant
