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
| Generated operation/fault/crash schedules | CONFORMANT (seeded bounded evidence) | `verification/corpus/simulator-schedule-seeds.txt` plus retained parity, topology, envelope, and trace corpora; `dwv-sim`, `dwv-codec`, `dwv-core`, `dwv-format`, and `dwv-trace` tests replay or reject deterministically | Bounded corpus only; sustained coverage-guided campaigns remain additive |
| Normalized semantic reproducer | CONFORMANT | OS-024 trace model, bounded parser, migration/limit tests, CLI export/render/replay, and baseline trace evidence | Preserve producer-specific witnesses if future tools find them |
| Bounded verification of high-consequence pure components | CONFORMANT (finite bounded domains) | `dwv-core`, `dwv-service`, and `dwv-codec` exhaustive finite-domain tests; decision and bounds in `docs/adr/ve-001-bounded-arithmetic.md` | Evidence covers declared bounds only; no arbitrary-width formal proof |
| Independent abstract recovery protocol | CONFORMANT (finite safety/reachability) | Canonical Quint source and bounded analysis in `verification/quint/RecoveryProtocol.qnt`, with authority and bounds recorded in `docs/adr/ve-002-independent-recovery-model.md` | Bounds and omitted facts remain explicit; no application-wide proof |
| Broad concurrency schedule exploration | NOT YET APPLICABLE | Slot lifecycle has deterministic transition tests, but no real concurrent executor/job/shutdown graph exists and no current product claim depends on one | VE-003 after executor/job/shutdown code exists; do not add a runtime for the checker |
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
  harnesses, the VE-002 bounded Quint recovery-model checks, and the bounded
  VE-005 seed corpora are implemented and recorded.
- **VE-005 bounded corpus:** deterministic trace seeds, parser mutations,
  simulator operation/fault schedules, parity reconstruction, topology
  candidate validation, and envelope mutations are recorded in
  `verification/corpus/`; their tests preserve seeds and producer logic rather
  than payload bytes.
- **Remaining additive evidence:** coverage-guided campaigns, manifest
  mutation/round-trip corpus expansion, and broader schedules remain optional
  strengthening work; no Linux, bridge, or hardware prerequisite applies.
- **Later evidence:** VE-003 concurrency schedules after a real
  executor/job/shutdown seam; VE-004 simulated I/O after that seam exists.
- **Platform-gated:** OS-021/022/023 and OS-030+ remain explicitly unclaimed
  until bridge, Linux, or hardware evidence exists.
## Remaining evidence gates

The current portable record is complete for its declared finite VE-005 slice.
Broader fuzz campaigns and VE-003 concurrency evidence are additive or
dependency-gated, not portable correctness blockers. The retained corpora do
not establish arbitrary-width proofs, exhaustive schedule coverage, concurrent
executor behavior, or platform behavior. No platform-gated claim is being used
to hide a portable gap.