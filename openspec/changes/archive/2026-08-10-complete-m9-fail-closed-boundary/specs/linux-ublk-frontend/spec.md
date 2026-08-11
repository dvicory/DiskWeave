## MODIFIED Requirements

### Requirement: The initial Linux publication profile is complete and narrow
<!-- dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.healthy-portable-io.publication-identity-is-derived-from-admitted-semantics -->

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required current recovery and portable admission authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. Publication metadata SHALL carry the exact publication identity supplied by the admitted service, including its array identity, and live discovery SHALL match by array identity before comparing the complete publication identity rather than reconstructing admission from fixture or path coincidence. Publication SHALL return a distinct successful published result only after the owned endpoint exists and is accepting work; admission success alone SHALL remain pre-publication. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or mutation; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state.

#### Scenario: The acceptance topology is assembled

- **WHEN** one data assignment, one parity assignment, recovery authority, portable admission, geometry, identity, epoch, capabilities, and admitted publication identity all validate and endpoint publication succeeds
- **THEN** the frontend reports the complete data-device group published as one fixed-size writable endpoint bound to that publication identity

#### Scenario: Admission succeeds before publication

- **WHEN** portable admission succeeds but endpoint publication has not completed
- **THEN** the frontend remains pre-publication and does not report an online/read-write endpoint

#### Scenario: A different admitted object shares the fixture directory

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service even if fixture paths or labels coincide
- **THEN** discovery reports reconciliation-required and does not report the current array online

#### Scenario: A different admitted object for the same array is live

- **WHEN** live endpoint metadata carries the same array identity but a publication identity different from the currently assessed admitted service
- **THEN** discovery reports reconciliation-required and does not report the current array online

#### Scenario: An unrelated array is live

- **WHEN** live endpoint metadata carries a different array identity from the currently assessed admitted service
- **THEN** discovery ignores that endpoint while continuing the bounded search for the requested array

#### Scenario: A wider topology is supplied

- **WHEN** a valid topology contains more than one data slot or otherwise exceeds the initial Linux profile
- **THEN** publication is refused as unsupported with no endpoint, partial claims, recovery mutation, or payload mutation

#### Scenario: Publication fails after admission

- **WHEN** current state changes, endpoint creation fails, or publication outcome requires reconciliation after portable admission
- **THEN** the frontend reports the exact failed or reconciliation-required outcome and does not report successful publication
