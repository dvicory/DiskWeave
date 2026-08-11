## MODIFIED Requirements

### Requirement: Checksum coverage names targets and extents
<!-- dwv:req req.checksum-plane.checksum-coverage-names-targets-and-extents -->

The checksum plane SHALL identify each data, P, and optional Q target by stable semantic identity and map it to explicit bounded checksum extents. Each persisted record SHALL carry the exact target and range, topology epoch, profile ID, checksum-set generation, target content generation, digest, fence evidence, verification generation, and state needed to validate that record without reconstructing omitted bindings from a current descriptor. A record missing any required binding or carrying an unsupported profile SHALL remain non-valid. Current baseline completeness SHALL require exactly one valid record for every expected current extent and reject missing, duplicate, extra, stale-topology, wrong-target, wrong-range, wrong-profile, wrong-set-generation, wrong-content-generation, unsupported, and mixed-set records.

#### Scenario: Data and parity have separate records

- **WHEN** a checksum baseline is built for data and P targets
- **THEN** records identify the target and extent independently and do not treat parity cleanliness as checksum coverage

#### Scenario: Extent boundaries are crossed

- **WHEN** a write spans multiple checksum extents
- **THEN** each affected extent is independently invalidated and revalidated under its own generation

#### Scenario: A descriptor changes after records were persisted

- **WHEN** the current baseline descriptor names a different profile, set generation, topology epoch, target mapping, range, or content generation than an existing record carried at creation
- **THEN** the existing record remains non-valid and cannot be retrospectively relabeled as current evidence

#### Scenario: Persisted records are mixed or duplicated

- **WHEN** records from different checksum sets coexist or more than one record claims the same expected extent binding
- **THEN** baseline completeness is invalid rather than complete
