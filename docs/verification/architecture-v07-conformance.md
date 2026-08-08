# Architecture v0.7 conformance matrix

Date: 2026-08-08

This matrix compares the v0.7 additions and clarifications with the current
portable implementation. It does not promote platform-gated work into the
portable claim.

## Matrix

| v0.7 property | Status | Implementation/evidence | Required boundary or next action |
|---|---|---|---|
| Consequential work has an owner through terminal outcome | CONFORMANT (portable seam) | `dwv-store::OperationSlotTable` retains buffers, frontend tags, child IDs, completion evidence, drain state, reconciliation, and abandonment until `release`; `dwv-service::finish` reclaims only after reconciliation | No daemon/background-task claim until a real executor exists |
| Frontend abandonment is not rollback | CONFORMANT | `OperationAdmission::abandon` only marks the slot; child completions are drained/reconciled before reclaim; abandonment matrix tests cover every completed-child cut point | Linux duplicate/reissue behavior remains platform-gated |
| Slot, buffer, child, retry, and queue bounds | CONFORMANT | `ResourceLimits`, `AdmissionController`, bounded slot-table tests, and service admission tests reject exhaustion | Add broader schedule evidence when concurrent executor code exists |
| Stale generation/token rejection | CONFORMANT | `OperationSlotTable::active_index` rejects stale tokens; generation reuse and stale-token tests pass | None for current portable contract |
| Durable transfer of unfinished work | CONFORMANT (write protocol) | `IntentCommit` durably marks dirty regions/integrity stale before home mutation; failed or uncertain writes leave recovery dirty/uncertain | Process/daemon handoff is not claimed; it belongs to later executor/recovery work |
| Uncertain completion is explicit | CONFORMANT | `CompletionDisposition::Uncertain`, `CompletionUncertain`, `ReconciliationRequired`, `UncertainRetained`, and simulator fault schedules preserve uncertainty | No blind retry or production reissue claim |
| Deterministic IDs, schedules, fault choices, and traces | CONFORMANT (bounded evidence) | `dwv-sim::Schedule`, deterministic fixture generation, `dwv-trace` schema/limits, and OS-024 replay evidence | Tool-specific scheduler witnesses remain optional |
| `dwv-sim` is the durability/recovery oracle | CONFORMANT | Simulator separates durable, volatile, pending, completion, recovery DB, envelope, and power-loss state; OS-004 and OS-024 evidence | Does not model real filesystem, kernel, device, or hardware behavior |
| Generated operation/fault/crash schedules | CONFORMANT (seeded bounded evidence) | `verification/corpus/simulator-schedule-seeds.txt` and the `dwv-sim` seeded schedule/replay test exercise deterministic operation/fault/interruption schedules | Broaden to parity/topology generation and sustained fuzz/property campaigns |
| Normalized semantic reproducer | CONFORMANT | OS-024 trace model, bounded parser, migration/limit tests, CLI export/render/replay, and baseline trace evidence | Preserve producer-specific witnesses if future tools find them |
| Bounded verification of high-consequence pure components | CONFORMANT (finite bounded domains) | `dwv-core`, `dwv-service`, and `dwv-codec` exhaustive finite-domain tests; decision and bounds in `docs/adr/ve-001-bounded-arithmetic.md` | Evidence covers declared bounds only; no arbitrary-width formal proof |
| Independent abstract recovery protocol | CONFORMANT (finite safety/reachability) | PlusCal source, generated translation, TLC run, `tla-rs` cross-check, and seeded mutation detection in `docs/adr/ve-002-independent-recovery-model.md` | Bounds and omitted facts remain explicit; no application-wide proof |
| Broad concurrency schedule exploration | EVIDENCE GAP | Slot lifecycle has deterministic transition tests, but no real concurrent executor/job/shutdown graph exists | VE-003 after executor/job/shutdown code exists; do not add a runtime for the checker |
| Production-adjacent deterministic filesystem/io_uring simulation | NOT YET APPLICABLE | No Linux-oriented executor or io_uring seam exists in the portable product | VE-004/OS-031 dependency-gated; `dwv-sim` remains authoritative |
| Capability-oriented APIs | CONFORMANT | `dwv-core`, `dwv-store`, file-store capability reports, persistence evidence, and explicit unsupported results avoid ambient OS assumptions | Physical capability certification remains outside the claim |
| Asupersync restriction | CONFORMANT | No dependency, copied code, execution, test, or benchmark use | Keep it design-reference-only |
| OS-007 parity-envelope boundary | CONFORMANT (experimental only) | Archived comparison/decoder evidence and current CLI inspection report; no stable-format claim | Gate C and stable-format evidence still required |
| OS-009 transaction selection boundary | CONFORMANT (fallback retained) | Explicit reference machine remains production fallback; `procmachines` candidate is isolated with comparison ADR and no service integration | Final production selection remains deferred |
| OS-017 repair authority | CONFORMANT | Scrub classifier requires independent digest/equation evidence, separate target, identity/generation-bound plan, readback, and conservative refusal | Keep source media preservation and no parity-only repair claims |
| OS-024 replay boundary | CONFORMANT | Trace schema is bounded/privacy-safe and replays simulator/file-backed semantics; it does not claim exact scheduler replay | Linux trace capture remains OS-033 |
| Portable CLI claim honesty | CONFORMANT | `docs/verification/portable-demo-baseline.md` records successful paths and unsupported physical/platform semantics; `dwv demo` uses real stores/service/recovery paths | Do not add bridge/Linux claims to the demo |

## Immediate blockers

No portable correctness or unsupported-claim blocker was found in this audit.
The operation/recovery seams are conservative for their current synchronous,
file-backed scope. The missing broad concurrency, production-adjacent I/O,
Linux frontend, live macOS bridge, and hardware evidence are dependency or
platform gates, not reasons to weaken the portable claim.


## Evidence-layer status

- **Portable evidence:** VE-001 finite-domain arithmetic/range/parity
  harnesses and VE-002 independent recovery-model checks are implemented and
  recorded.
- **Initial VE-005 evidence:** deterministic trace seeds, parser mutations, and
  seeded simulator operation/fault schedules are recorded in
  `verification/corpus/`; `dwv-trace` and `dwv-sim` tests exercise them. This
  supports VP-005, VP-008, VP-010, and VP-011 within declared bounds.
- **Remaining VE-005 work:** generated/fuzzed parity, topology, and boundary
  schedules plus broader operation/fault campaigns. It has no Linux, bridge,
  or hardware prerequisite.
- **Later evidence:** VE-003 concurrency schedules after a real
  executor/job/shutdown seam; VE-004 simulated I/O after that seam exists.
- **Platform-gated:** OS-021/022/023 and OS-030+ remain explicitly unclaimed
  until bridge, Linux, or hardware evidence exists.

## Remaining evidence gates

The current portable record remains incomplete for the broader VE-005
schedule/fuzz corpus and VE-003 concurrency evidence. The retained trace and
simulator corpora do not establish parity/topology generation, exhaustive
schedule coverage, concurrency, or platform behavior. No platform-gated claim
is being used to hide either gap; VE-003 remains gated on the
executor/job/shutdown seam.