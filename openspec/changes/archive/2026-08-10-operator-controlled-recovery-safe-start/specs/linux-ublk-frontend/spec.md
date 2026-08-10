## MODIFIED Requirements

### Requirement: The initial Linux publication profile is complete and narrow
<!-- dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required current recovery and portable admission authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. Publication SHALL return a distinct successful published result only after the owned endpoint exists and is accepting work; admission success alone SHALL remain pre-publication. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or mutation; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state. Frontend publication SHALL not broaden current durability or physical power-loss claims.

#### Scenario: The acceptance topology is assembled

- **WHEN** one data assignment, one parity assignment, recovery authority, portable admission, geometry, identity, epoch, and capabilities all validate and endpoint publication succeeds
- **THEN** the frontend reports the complete data-device group published as one fixed-size writable endpoint

#### Scenario: Admission succeeds before publication

- **WHEN** portable admission succeeds but endpoint publication has not completed
- **THEN** the frontend remains pre-publication and does not report an online/read-write endpoint

#### Scenario: A wider topology is supplied

- **WHEN** a valid topology contains more than one data slot or otherwise exceeds the initial Linux profile
- **THEN** publication is refused as unsupported with no endpoint, partial claims, recovery mutation, or payload mutation

#### Scenario: Publication fails after admission

- **WHEN** current state changes, endpoint creation fails, or publication outcome requires reconciliation after portable admission
- **THEN** the frontend reports the exact failed or reconciliation-required outcome and does not report successful publication
