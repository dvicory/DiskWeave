## MODIFIED Requirements

### Requirement: The outcome is a bounded portable-demo decision
<!-- dwv:req req.macos-bridge-feasibility.the-outcome-is-a-bounded-portable-demo-decision -->
<!-- dwv:requires req.normalized-block-semantics.adapters-expose-bounded-deterministic-conformance-behavior -->

The ADR SHALL select a candidate only for the evidence-backed macOS functional reference, or document a blocker and narrow the live frontend boundary. It SHALL state that process kill/restart, detach, host-file synchronization, and simulator schedules are covered while physical power loss, controller cache, production FUA, and Linux frontend semantics are not certified.

#### Scenario: Candidate passes the required matrix

- **WHEN** fixed geometry, block trace, synchronization, failure, identity, and separation tests pass
- **THEN** the candidate is selected behind the normalized frontend seam for the bounded macOS functional reference

#### Scenario: No candidate passes

- **WHEN** all candidates fail a required coherence or synchronization criterion
- **THEN** the ADR compares DriverKit/SCSI or a narrower live reference and does not force an unsafe bridge into the product
