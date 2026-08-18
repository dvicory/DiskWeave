## 1. Delegated relation repair

- [x] 1.1 Add explicit range-held, terminal-pending-release, aborted, and release transitions to `models/quint/RecoveryProtocol.qnt`; prevent `begin` from replacing an unreleased terminal obligation.
- [x] 1.2 Guard durable home reconciliation by complete represented mutation coverage and preserve continuation after volatile-home abandonment with incomplete coverage.
- [x] 1.3 Encode invariants for terminal release, durable-home coverage, aborted pre-mutation state, and no ownership-free uncertainty; keep invalid/repeated actions absent from the enabled relation.
- [x] 1.4 Extend `verification/quint/RecoveryProtocolAnalysis.qnt` with release, released-range reuse, aborted-release, volatile-abandonment, continuation, and terminal-release witnesses.

## 2. OpenSpec and knowledge authority

- [x] 2.1 Validate the active proposal, delta specs, design, and task dependencies against the repaired ownership boundary.
- [x] 2.2 Update the maintained knowledge contract and scanner regression coverage for marked delegated-source changes and dependent review propagation.
- [x] 2.3 Review every direct VE-002 dependent and preserve current-versus-target authority wording; do not edit historical requirements or archive records.

## 3. Evidence and Connect assessment

- [x] 3.1 Run Quint typecheck, bounded Apalache verification, bounded assumptions, seeded simulations, release/reuse/uncertainty witnesses, and deterministic same-seed replay.
- [x] 3.2 Run disposable terminal-begin, durable-home-coverage, and intent-order mutations and retain only their observed failure summaries, not mutant files.
- [x] 3.3 Run focused Rust transaction-machine tests plus two seeded Quint Connect projections for release ordering, duplicate/out-of-order results, abandonment, crash, and uncertainty; record the exact model-to-Rust granularity mismatch instead of adding a general bridge.
- [x] 3.4 Update the VE-002 ADR, portable evidence log, verification manifest, and maintained architecture roadmap with repaired claims, bounds, evidence, and non-claims.

## 4. Boundary verification

- [x] 4.1 Run focused OpenSpec validation, knowledge readiness/context/affected-path checks, and documentation checks/build.
- [x] 4.2 Review affected dependents and final authority, evidence, mutation, historical-isolation, and U11 recommendation claims.
- [x] 4.3 Keep `repair-ve002-quint-authority` active and unsynced, and leave canonical sync/archive for the normal authority transition.
