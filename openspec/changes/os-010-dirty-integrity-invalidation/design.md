# OS-010: Dirty-region protocol and atomic integrity invalidation

## 1. Architecture decisions and target gate

Implement handoff D-008, D-009, D-010, D-021 and the Phase 1 OS-010 row. The target is Gate D, with evidence consumed by Gate E. Keep P-005 behind `RecoveryStateStore`; do not resolve P-007 here.

## 2. Concrete outcome

The explicit transaction path can ask recovery state whether intent is required, durably mark dirty/stale generations, and later clear only with complete fence evidence.

## 3. Prerequisites

Reuse `dwv-core` IDs/ranges, `dwv-recovery` state semantics, OS-004 schedules, and OS-008 actions. OS-009 may compare a procedural machine later but is not a dependency of the semantic module.

## 4. Exact scope and non-scope

Add the portable transition model and adapters/tests. Do not add a checksum algorithm, physical store, SQLite-specific API, journal/PPL, Linux frontend, or repair policy.

## 5. Semantic APIs and contracts

Split recovery code into modules for generations, invalidation, intent/checkpoint decisions, and transition evidence. Use typed IDs and checked arithmetic. The API returns structured refusal/uncertainty rather than `bool` and never hides a failed durable transition.

## 6. State ownership and lifecycle

Recovery state owns durable truth; the reference machine owns captured transaction evidence. A transaction gets one intent identity and immutable captured generations. Repeated intent is idempotent for the same identity and target set; a new generation invalidates old checkpoint evidence.

## 7. Persistent-state impact

Extend the in-memory semantic snapshot and the existing SQLite evaluation model with region/checksum generation fields if the adapter needs them. Keep migrations additive and exportable. Do not make the control database authoritative.

## 8. Irreversible and durability boundaries

The machine emits `PersistIntent` before any `ReadHome`, `ComputeParity`, or `WriteHome`. It emits `Checkpoint` only after `Fence` results cover each participating store and the recovery transition verifies generations. A failed/uncertain action terminates in a conservative state.

## 9. State and sequence diagrams

```text
request -> capture -> intent decision
                  -> PersistIntent(dirty + stale)
                  -> home I/O / parity I/O
                  -> Fence(store set)
                  -> generation check
                  -> Checkpoint(clear)
                  -> terminal result
```

The transition trace records the action index, durable evidence, and state digest so faults can cut at every arrow.

## 10. Concurrency and resource rules

Use deterministic sorted region/checksum IDs to avoid lock-order cycles. The module does not own OS locks; callers provide a serialized recovery transaction or equivalent semantic commit. Overlapping requests observe already-durable dirty/stale state and cannot use stale captured generations to clear it.

## 11. Failure matrix

Represent intent failure, home short/failure/uncertainty, fence failure, recovery-write failure, crash, and generation mismatch as distinct typed outcomes. All post-intent outcomes preserve dirty/unknown; only a complete proof may produce a clear decision.

## 12. Deterministic simulator cases

Add a compact schedule builder with cut points for each action and combinations of two overlapping writes. Include repeated intent and session-close schedules. Store fixtures as normalized semantic events, not filesystem paths.

## 13. Property, model, and fuzz tests

Use a small reference model in tests to assert ordering and state monotonicity. Generate bounded region sets, checksum sets, generation values, action outcomes, and cut points. Add mutation assertions for skipped invalidation, premature clear, and reused fence evidence.

## 14. Integration tests

Compose the transition model with the existing `dwv-recovery` snapshot and `dwv-sim` fault engine. Add a SQLite adapter test only as evidence that semantic state survives a durable commit; keep all ordering tests backend-neutral.

## 15. Observability, security, and operator behavior

Expose stable transaction/region/checksum IDs, generations, and missing evidence. Bound trace payloads and redact data buffers. Refusal defaults to write blocking and an actionable recovery mode.

## 16. Performance and resource bounds

Use vectors/sets sized by the request working set. Avoid cloning full-array state. Keep trace capture optional but deterministic when enabled. Benchmark first-write versus already-dirty paths without weakening checks.

## 17. Executable acceptance criteria

Acceptance is the OS-010 spec plus all generated traces proving no forbidden ordering and no false clean/valid outcome. Run focused tests, workspace tests, format, dependency inspection, and OpenSpec validation on macOS.

## 18. Forbidden outcomes

No API may return a clear decision from a stale/unknown fence, or accept home mutation after failed intent. No implementation type from SQLite, an OS, or a runtime may enter the public semantic module.

## 19. Migration and compatibility consequences

Existing recovery fixtures import absent checksum evidence as stale/unknown. New fields are additive in semantic exports and can be reconstructed conservatively. No permanent format field is frozen.

## 20. Next OpenSpecs unlocked

OS-011 can add hashing and asynchronous revalidation. OS-013 can compose this protocol with stores and parity. OS-014/015 remain separate evidence-gated changes.
