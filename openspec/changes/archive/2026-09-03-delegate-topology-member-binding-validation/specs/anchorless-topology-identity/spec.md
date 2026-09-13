## MODIFIED Requirements

### Requirement: Topology validation rejects ambiguous or inconsistent assignments
<!-- dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->

The requirement retains ownership of topology and member-binding validity, its supplied-input assumptions, its product-facing rejection meaning, and its refusal boundaries. The exact finite pure validity relation over an already supplied, owner-accepted topology snapshot and member-binding set SHALL be delegated to the canonical `TopologyMemberBindingValidation` module in `models/quint/TopologyMemberBindingValidation.qnt`; within the declared surface, that module SHALL be the sole exact authority for the valid/invalid result.

The delegated domain SHALL contain only supplied semantic values: an owner-accepted selected profile and protected geometry; one snapshot array identity and topology epoch; snapshot assignments with logical slot, role, coding position, assignment-instance identity, and assignment generation; member bindings with the corresponding captured fields; opened-store identity, topology epoch, protected-length, and block-size facts; and one owner-qualified identity comparison for every pair of bound stores.

Identity observations MAY accompany production correspondence, but the relation SHALL consume only their owner-qualified pairwise comparison results and SHALL NOT decide observation provenance, confidence, assessment precedence, candidate choice, or operator attestation. It SHALL NOT own discovery; candidate enumeration; profile or protected-geometry construction, deserialization validation, or trust; identity evidence production or assessment; service-profile selection or authorization; implementation resource ceilings; writable-assembly or request-admission policy; capability support beyond equality to supplied geometry; topology preparation, transition, verification, commit, or publication; payload validity; parity computation; recovery; persistence; physical durability; or physical I/O. Snapshot construction and service assembly SHALL compose the delegated result with those non-delegated owners and SHALL preserve current acceptance and rejection outcomes within the mapped input domain.

The current obligations to reject mismatched array identities and invalid protected lengths remain non-delegated because the current pure boundary supplies no second array claim and unchecked deserialization can bypass the checked profile/geometry constructors. This change SHALL NOT invent the missing comparison source, treat transition-owned active/candidate array comparison as snapshot validation, silently trust unchecked deserialization, or change current admission outcomes. The requirement that a topology assignment authorize the selected stable store identity also remains non-delegated; declared/opened-store equality and uniqueness do not supply that authorization.

A rejected snapshot SHALL not be published. A rejected member-binding set SHALL fail before request admission or member mutation. Neither the relation nor a consumer SHALL use physical enumeration, vector position, discovery order, or collection order as a substitute for a logical slot, coding position, or stable identity.

The parameterized canonical module SHALL define semantics independently of any verification bound. Analysis, deterministic scenario, mutation, and Connect modules under `verification/quint/` SHALL be evidence only and SHALL NOT broaden the delegated domain or establish exhaustive behavior outside their stated finite scopes.

#### Scenario: Two candidates claim one slot

- **WHEN** a topology candidate contains two active assignments for the same logical slot
- **THEN** the delegated relation rejects the candidate and no consumer publishes it

#### Scenario: Coding positions are reordered by discovery order

- **WHEN** devices, snapshot assignments, or bindings are supplied in a different order with unchanged semantic values and owner-qualified identity comparisons
- **THEN** the delegated valid/invalid result is unchanged, the same logical topology is produced, and coding positions remain unchanged

#### Scenario: An opened binding disagrees with its assignment

- **WHEN** a member names a valid slot but carries a different role, coding position, assignment instance, assignment generation, or topology epoch
- **THEN** the delegated relation rejects the binding set and assembly fails before admission or member mutation

#### Scenario: An identity comparison is not distinct

- **WHEN** the identity-assessment owner supplies an aliased or ambiguous comparison for any two member bindings
- **THEN** the delegated relation rejects the binding set without choosing a candidate or changing the supplied assessment

#### Scenario: A finite analysis succeeds

- **WHEN** a bounded analysis instance checks the delegated relation without finding an invariant violation
- **THEN** the evidence reports only that exact finite scope and does not claim arbitrary-width topology coverage, identity-assessment correctness, Rust correctness, transition safety, or physical-I/O correctness
