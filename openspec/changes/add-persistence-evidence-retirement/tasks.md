## 1. Exact Occurrence and Root Model

- [ ] 1.1 Add the persisted semantic fence-occurrence identity and canonical certificate validation; verify monotonic non-reused identities, equal-occurrence multiplicity, canonical internal ordering, and rejection of new duplicate bindings
- [ ] 1.2 Add owner-qualified root bindings for current CLEAN regions, valid integrity records, conservative current-session bindings, coded-capture evidence, unresolved recovery commits, and legacy-unreconciled migration state; verify each root's creation, discharge, exact rebind, predecessor binding, and owner boundary
- [ ] 1.3 Update dirty-region, integrity, session, coded-capture, and recovery lookup paths to resolve occurrence identities rather than certificate value or vector position; verify retired evidence cannot satisfy a remaining claim and retained evidence satisfies every existing exact lookup

## 2. Atomic Retirement

- [ ] 2.1 Add the generation- and topology-checked predecessor-to-successor retirement mutation with exact retirement occurrences and root rebind/discharge facts; verify complete successor validation and all-or-none application
- [ ] 2.2 Implement exhaustive retirement outcomes for durable successor, known rejection/non-commit, and lost/corrupt/unclassifiable acknowledgement; verify dependent authority is withheld until exact reopen reconciliation for may-have-committed outcomes
- [ ] 2.3 Reject over-bound successors without removing predecessor evidence and record the explicit `dwv-x6y.2.2` pre-mutation reservation prerequisite; verify this change does not claim to reserve capacity before a write-recovery record or authorize serving after reservation failure

## 3. Owner Composition

- [ ] 3.1 Compose current CLEAN and checksum root discharge/rebind at their existing durable owner transitions; verify a never-rewritten clean region retains its root, dirty transition discharges stale embedded bindings, and valid-integrity evidence remains live until stale/absent or exact replacement
- [ ] 3.2 Create a conservative session-lifecycle root whenever a supported successor, `CloseWritableSession`, migration, or reopen/import contains a global fence; preserve it across `BeginWritableSession` replacement and leave discharge/rebind to its future canonical owner and `dwv-x6y.2.3`; verify generic recovery retirement cannot discharge or rebind session state
- [ ] 3.3 Connect coded-capture root discharge only through existing dirty-integrity cleanup, membership-compaction, and retirement capabilities; verify open, commit-pending, commit-unknown, refused, clean-known, and retired captures retain or discharge exact roots correctly
- [ ] 3.4 Isolate unresolved dependencies from unrelated settled evidence; verify known failed, lost, or uncertain disk/range/capture/recovery work retains only reachable occurrences while unrelated occurrences remain retirement-eligible

## 4. Schema and Adapter Migration

- [ ] 4.1 Add the next semantic recovery schema version and explicit migration step after version 6; verify older manifests are migration-required before writable use and are never reinterpreted through serde defaults
- [ ] 4.2 Migrate legacy occurrences with stable identities while preserving equal duplicates, legacy ordering, exact values, and ambiguous copied references under `legacy-unreconciled` roots; verify the migration-owner predecessor-bound inventory proof is the only automatic orphan-resolution authority and no silent coalescing or deletion occurs
- [ ] 4.3 Persist retirement successors through the semantic adapter and reconcile exact prior/proposed manifests after process loss; verify durable successor, known non-commit, reopen-prior, reopen-proposed, and reopen-neither outcomes
- [ ] 4.4 Update semantic export, inspection, fixtures, snapshot constructors, adapter dispatch, and public projections for the new schema; verify storage-engine layout remains absent from the portable boundary

## 5. Bounded Long-Use Evidence

- [ ] 5.1 Exercise a long representative sequence of successful write/CLEAN churn with bounded current roots and derive the structural plateau from root cardinalities; verify retained occurrence count and complete successor bytes remain independent of total writes
- [ ] 5.2 Exercise repeated same-range supersession, disjoint clean regions, partial overlap, changed store incarnation/domain/capability/topology, equal duplicate occurrences, and stale embedded bindings; verify only exact owner-approved rebinds permit retirement
- [ ] 5.3 Exercise one failed member, an unresolved recovery acknowledgement, open and retired captures, and concurrent retirement proposals; verify dependent evidence remains conservative and unrelated evidence can retire without claiming the separate `.2.2` reservation protocol
- [ ] 5.4 Exercise retirement at every durable crash boundary and retry after known failure; verify the predecessor or exactly reconciled successor remains authoritative and no payload bytes are changed

## 6. Change Verification

- [ ] 6.1 Run focused recovery, checksum, dirty-integrity, service, adapter, migration, and restart checks for the changed requirements and record exact evidence
- [ ] 6.2 Run affected-owner knowledge/readiness checks and verify the finalized relationship graph, then run `cargo xtask docs check` after all implementation markers are finalized
- [ ] 6.3 Run `cargo xtask docs build` and the applicable formatting/lint checks after implementation and evidence are complete
