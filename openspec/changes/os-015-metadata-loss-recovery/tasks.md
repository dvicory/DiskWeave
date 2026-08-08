## 1. OpenSpec and handoff contract

- [x] 1.1 Ground the proposal/spec/design in handoff Sections 8.10, 12.5, 17.6, 22.3, 26.3, 26.5, 26.6, Gate E, and the OS-014 dependency.
- [x] 1.2 Encode all 18 Section 12.5 cases, stable IDs, conservative actions, evidence gates, and no-in-place-write policy in the spec.
- [x] 1.3 Keep OS-016 degraded/rebuild and Linux frontend work explicitly separate.

## 2. Portable metadata-loss planner

- [x] 2.1 Add `dwv-recovery::metadata_loss` with bounded case, action, disposition, evidence, baseline, and authorization types.
- [x] 2.2 Implement a total case-to-plan mapping and deterministic dry-run rendering with no payload/OS/SQLite types.
- [x] 2.3 Add authorization rules for certified clean, exhaustive matches, verified repairs, explicit rebaseline, incomplete scans, ambiguous mismatches, backup validation, and replica disagreement.

## 3. Fresh semantic recovery state

- [x] 3.1 Add a bounded metadata-loss audit to the semantic recovery snapshot/schema descriptor without exposing storage-engine layout.
- [x] 3.2 Build fresh manifests only for completed all-data/single-parity plans with validated core topology, pending-baseline semantics, and unchanged payload stores.
- [x] 3.3 Test new-lineage planning, certified-gate plan-only behavior, mismatch refusal, clone/topology refusal, and lost-Q-position refusal; keep later-spec execution non-authorizing.

## 4. macOS/portable evaluation

- [x] 4.1 Add the evaluation-only SQLite fresh-state adapter operation using the existing prototype and no new database crate.
- [x] 4.2 Add a temporary regular-file fixture that deletes/recreates recovery state and proves direct payload preservation.
- [x] 4.3 Add the `metadata-loss-dry-run` example and test deterministic complete matrix enumeration.
- [x] 4.4 Reserve SQLite recovery targets atomically, clean failed reservations, persist the complete audit summary, and distinguish physical schema v1 from semantic schema v2.

## 5. Acceptance and handoff

- [x] 5.1 Run workspace tests, format, targeted clippy, dependency inspection, dry run, and OpenSpec validation.
- [x] 5.2 Record acceptance evidence and the portable/macOS boundary; do not claim Linux or hardware durability.
- [x] 5.3 Mark complete only after every matrix row has an exact executable policy assertion, run-bound repairs cannot be replayed, and later-spec actions fail closed; leave OS-016/017 as separate changes.
