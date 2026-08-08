## Context

The handoff makes `array.sqlite3` protocol authority, but its loss must not make ordinary data bytes unreadable. Before OS-015, `dwv-recovery` has a portable in-memory semantic store and `dwv-recovery-sqlite` has an evaluation-only header adapter; neither turns the formal Section 12.5 matrix into a concrete recovery plan or creates an auditable fresh state.

The design must preserve OS-014’s authority boundary: a parity mismatch is an inconsistency report, not proof that parity is wrong. It must also avoid pulling SQLite, file paths, frontend handles, or Linux types into the portable recovery crate.

## Decisions

### Matrix as a closed semantic enum

`dwv-recovery::metadata_loss` owns a fixed `MetadataLossCase::ALL` table matching every Section 12.5 row. `MetadataLossPlan::for_case` maps each case to a stable action, disposition, evidence gate, operator confirmation, baseline policy, and payload-write policy. Exhaustive enumeration makes missing cases testable and gives the dry-run a deterministic source.

The plan uses semantic verification outcomes rather than depending on `dwv-verify` types (which already depend on `dwv-recovery`). `dwv-service` translates OS-014 reports at the boundary into `CertifiedCleanEnvelope`, `ExhaustiveMatches`, `IdentifiedRepairPending`, `ExhaustiveIdentifiedRepairs`, `ExplicitDataAuthoritativeRebaseline`, `AmbiguousMismatch`, `IncompleteScan`, `ValidatedBackup`, or `ConflictingReplicas`. An identified mismatch remains pending until every separate-target repair has a verified outcome.

### Authorization before fresh state

`MetadataLossPlan::authorize` returns an authorization only when the supplied outcome is sufficient for that case. Refused cases and ambiguous/incomplete evidence return an error. Explicit data-authoritative rebaseline uses a separate operator-confirmed authorization method. Only the completed exhaustive all-data/single-parity path can turn authorization into fresh state in OS-015. The certified-envelope path remains non-authorizing until Gate H supplies a real certificate receipt. P/Q, missing-member, parity-rebuild, and new-lineage plans remain non-authorizing until later changes supply verified reconstruction/rebuild receipts. `MetadataLossAuthorization::fresh_manifest` accepts only a recovery topology derived from a validated `dwv-core::TopologySnapshot`, derives lineage from that topology, records a fixed-size metadata-loss audit, and exports the manifest at generation zero. The helper is semantic and never writes payload bytes.

The audit records matrix version, lineage, case, source recovery health, topology epoch, verification outcome, action, and baseline disposition. Baseline dispositions ending in `Required` are pending work, not claims that checksum or parity evidence already exists. It deliberately records no path, payload digest, SQL row, or operator free-form text.

### Verification receipts are run-bound

OS-014 repair candidates and outcomes are opaque. Each scan creates a process-local run binding containing the source identities; a repair rechecks those identities, records the separate target identity and read-back digest, and can satisfy OS-015 only for the exact report that produced it. A stale or fabricated range/target tuple cannot authorize recovery.

### Evaluation-only macOS path

`dwv-recovery-sqlite` gets a small `recreate_from_metadata_loss` convenience method that atomically reserves an absent target, initializes its existing checked-in physical schema, writes the authorized semantic manifest, and exports the header. Failed initialization removes that reservation. Its persisted audit summary includes every fixed audit field. A temporary regular-file test deletes the recovery database, keeps direct data files, and verifies recreation leaves those files byte-identical. SQLite `user_version=1` identifies the unchanged physical prototype schema; recovery schema v2 identifies the semantic manifest. This is an evaluation fixture, not a production SQLite selection or durability claim.

### CLI-style dry run

`crates/dwv-recovery/examples/metadata-loss-dry-run.rs` prints `MetadataLossPlan::all()` using only bounded semantic values. It is intentionally an example binary rather than a frontend command so OS-015 does not invent a CLI architecture or Linux integration.

### Explicit boundaries

- OS-014 remains the exhaustive scanner and selective repair engine; OS-015 consumes its semantic outcome.
- OS-016 owns degraded reads, missing-member reconstruction, replacement assignment, and resumable rebuild execution.
- OS-017 owns background scrub and verified-repair scheduling.
- No parity envelope format is selected and no format-v1 claim is made.

## Risks and mitigations

- A matrix enum can drift from the handoff: keep the 18-case array, stable IDs, and exact per-row policy assertions adjacent to the mapping.
- A fresh manifest could be mistaken for historical proof: include a mandatory audit and baseline disposition, and reject ambiguous/incomplete authorization.
- A regular-file test could overclaim APFS durability: label it portable-demo/evaluation-only and leave live bridge work to OS-020/OS-022.
- A future adapter could accidentally pass paths or payloads across the semantic boundary: keep the new API composed only of bounded enums, IDs, topology values, and manifests.
