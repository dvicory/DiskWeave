# Portable verification evidence log

Date: 2026-08-08

## Baseline

The pre-remediation portable workflow is recorded in
[`portable-demo-baseline.md`](portable-demo-baseline.md), including the
disposable fixture, commands, outputs, and claim boundary.

## VE-002 focused evidence (historical pre-repair canary)

Date: 2026-08-17

Historical canary source: `models/quint/RecoveryProtocol.qnt`

Authority status: the current `RecoveryProtocol` source is
`models/quint/RecoveryProtocol.qnt`. The historical canary below records the
first Quint model exercise; the current authority is the repaired relation
described in the evidence section below. The separate
`RecoveryProtocolAnalysis` module at
`verification/quint/RecoveryProtocolAnalysis.qnt` binds finite evidence and
its runs are verification evidence, not semantic authority. The archived
`replace-ve002-tla-with-quint` and `reconcile-ve002-quint-ownership` changes
remain historical provenance. Quint does not own typed evidence, topology,
persistence, operation-slot lifetime, frontend delivery, or production
recovery authority.

Toolchain: `mise.toml` pins `@informalsystems/quint` at `0.32.0`.

Commands:

```text
quint typecheck models/quint/RecoveryProtocol.qnt
quint test verification/quint/RecoveryProtocolAnalysis.qnt --main RecoveryProtocolAnalysis
quint run verification/quint/RecoveryProtocolAnalysis.qnt --main RecoveryProtocolAnalysis \
  --max-steps 12 \
  --invariants TypeInvariant \
  --invariants NoFalseClean \
  --invariants MutationRequiresIntent \
  --invariants UncertaintyIsVisible \
  --invariants DurableWorkIsOwned \
  --invariants TerminalRequiresEvidence \
  --invariants FenceAndCheckpointCoverage \
  --witnesses beginReachable \
  --witnesses durableIntentReachable \
  --witnesses mutationReachable \
  --witnesses uncertainIntentReachable \
  --witnesses uncertainHomeReachable \
  --witnesses reconciliationReachable \
  --witnesses terminalReachable \
  --max-samples 10000 --seed 22082026
```

Observed:

- Typecheck and the bounded-assumption test passed.
- The sampled run found no invariant violation across 10,000 traces.
- Witnesses were reached in 10,000/10,000 begin traces, 9,509/10,000
  durable-intent traces, 9,070/10,000 mutation traces, 6,705/10,000
  uncertain-intent traces, 8,417/10,000 uncertain-home traces, 9,998/10,000
  reconciliation-handoff traces, and 196/10,000 terminal traces.
- Same-seed ITF traces were byte-identical after generated timestamps and
  per-trace metadata were removed.
- A disposable copy with the `mutate` durable-intent guard removed was
  rejected by `MutationRequiresIntent`. The mutant is not retained.

The historical Quint evidence is bounded to model `MaxDepth = 8`, CLI
`--max-steps 12`, two affected regions, and two stores. It does not claim
exhaustive model checking, Rust implementation correctness, exact
region/checksum mapping, typed fence admissibility, topology,
persistence-engine behavior, operation-slot lifetime, frontend behavior,
physical durability, or unbounded recovery progress.

## Write-recovery lifecycle delegated-model evidence

Date: 2026-08-17

This section records current evidence for the delegated relation. The
pre-repair record above remains historical canary evidence; the claims below
describe the current model and its bounded verification.

The current relation adds explicit range ownership and release, permits a new
`startWrite` after release, gives pre-write-recovery-record rejection an
aborted outcome, blocks completed-write reuse before release, and rejects
invalid, repeated, and out-of-order transitions without mutating state. It
requires complete represented data/parity-write coverage before durable
data/parity reconciliation. Reconciliation observations are explicit so
repeated observations have no self-loop; the finite analysis adds
release/reuse, aborted-release, interrupted-processing, continuation,
repeated-action, completed-release, durable-data-parity-coverage, and
partial-data-parity witnesses. Regions, stores, and execution depths remain
evidence bounds.

Commands:

```text
quint --version
quint typecheck models/quint/RecoveryProtocol.qnt
quint typecheck verification/quint/RecoveryProtocolAnalysis.qnt
quint typecheck verification/quint/RecoveryProtocolConnect.qnt
quint verify verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 12 \
  --invariants TypeInvariant NoFalseClean DataParityWriteRequiresWriteRecoveryRecord \
    UncertaintyIsVisible UncertaintyIsOwned DurableWorkIsOwned \
    CompletedOrAbortedRequiresRelease CompletedWriteRequiresEvidence \
    DurableDataParityWriteRequiresCoverage PersistenceEvidenceAndRecoveryCleanCoverage
quint test verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis
quint run verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 12 \
  --max-samples 10000 \
  --seed 22082026 \
  --invariants TypeInvariant NoFalseClean DataParityWriteRequiresWriteRecoveryRecord \
    UncertaintyIsVisible UncertaintyIsOwned DurableWorkIsOwned \
    CompletedOrAbortedRequiresRelease CompletedWriteRequiresEvidence \
    DurableDataParityWriteRequiresCoverage PersistenceEvidenceAndRecoveryCleanCoverage \
  --witnesses startWriteReachable durableWriteRecoveryRecordReachable \
    dataParityWriteReachable unknownWriteRecoveryRecordReachable \
    unknownDataParityWriteReachable interruptedProcessingReachable \
    completedAwaitingReleaseReachable releasePendingReachable \
    abortedAwaitingReleaseReachable releasedReachable \
    resumedDataParityWriteReachable reconciledDataParityWriteReachable \
    durableDataParityWriteReachable
cargo test -p dwv-transaction-ref --lib
cargo test -p dwv-transaction-ref --test quint_connect -- --nocapture
```

Observed:

- Quint `0.32.0` typechecked all three sources. The exact `quint verify`
  command above completed with Apalache `0.56.1`, bounded depth 12, and no
  invariant violation across all ten named invariants.
- The analysis reported nine passing tests:
  `boundedAssumptionsTest` passed once; `completedReleaseRequiredTest`,
  `completedWriteStartBlockedTest`, `releasePermitsReuseTest`,
  `volatileAbandonmentContinuationTest`,
  `partialDataParityWriteCannotBeDurableTest`,
  `repeatedAbandonBlockedTest`, and
  `repeatedDataParityWriteReconciliationBlockedTest` each passed 10,000
  randomized cases; `rejectedWriteRecoveryRecordReleaseTest` passed once.
- The sampled invariant run found no violation across 10,000 traces.
  Witnesses cover start, durable write-recovery record, data/parity write,
  unknown write-recovery record, unknown data/parity write, interrupted
  processing, completed-awaiting-release, release-pending,
  aborted-awaiting-release, released, resumed data/parity write, reconciled
  data/parity write, and durable data/parity write.
- A disposable copy with the completed-write start guard removed fails
  `completedWriteStartBlockedTest`; a copy with the durable data/parity
  coverage guard removed fails
  `partialDataParityWriteCannotBeDurableTest`. No mutants are retained.
- The two retained Connect traces, seeds `22082026` and `1`, both execute
  `init`, `startWrite`, `confirmWriteRecoveryRecordDurable`,
  `attemptDataParityWrite`, `confirmDataParityWritesDurable`,
  `commitRecoveryClean`, and `releaseRange`, then a new `startWrite` and
  second lifecycle through `releaseRange`. Connect therefore establishes
  release-followed-by-reuse for this mapped lifecycle.
- The Connect projection is intentionally bounded, not a conformance bridge.
  Quint `attemptDataParityWrite` advances one mapped abstract region while
  Rust batches read completion, parity computation, and write completion. The
  projection uses one mapped region and no abstract stores, so it excludes
  abstract persistence-evidence and data/parity-write reconciliation
  transitions whose concrete evidence is owned outside this projection.
  Rust's typed flush, recovery CLEAN, crash, reconciliation,
  persistence-evidence, watermark, generation, topology, and result-class
  evidence remains outside the delegated projection. Concrete stale-result
  correlation and generation admissibility remain owned by the applicable
  current requirements.

Reproduction recipes:

```text
quint run verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis --max-steps 12 --max-samples 1 \
  --n-traces 1 --seed 22082026 \
  --out-itf /tmp/write-recovery-lifecycle-replay-a-{seq}.itf.json --verbosity 0
quint run verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis --max-steps 12 --max-samples 1 \
  --n-traces 1 --seed 22082026 \
  --out-itf /tmp/write-recovery-lifecycle-replay-b-{seq}.itf.json --verbosity 0
```

The completed-write-start and partial-data/parity mutations are expected to
fail their respective tests with `QNT511`. The trace recipe uses Quint's
`--out-itf` output and compares normalized state records after removing
generated metadata. The current evidence remains bounded to depth 12 for
delegated checks, two affected regions and two stores for the finite analysis
instance, sampled execution, and two Connect seeds covering release followed
by reuse. It does not claim exhaustive arbitrary-width model checking, Rust
implementation correctness, exact region/checksum mapping, persistence-
evidence admissibility, topology, persistence, operation-slot lifetime,
frontend behavior, physical durability, or unbounded recovery progress.

## verify.bounded-arithmetic focused evidence

Source tests:

- `crates/dwv-core/src/topology.rs`
- `crates/dwv-service/src/range.rs`
- `crates/dwv-codec/src/lib.rs`

Commands:

```text
cargo test -p dwv-core --lib bounded_geometry_accepts_only_valid_capacity_and_alignment
cargo test -p dwv-service --lib bounded_split_ranges_preserve_aligned_coverage
cargo test -p dwv-codec --lib bounded_geometry_and_parity_exhaustive
```

Observed: all three focused tests passed. The finite harness covers block
sizes 1, 2, 4, and 512; capacities through four blocks; range splitting
through eight blocks; every bounded data vector over `{0, 1, 255}` for the
selected geometries; every valid parity-update range/replacement vector; and
every single-erasure reconstruction range, including zero-extended tails.
The evidence is bounded to those domains and does not claim arbitrary-width
formal proof, P/Q semantics, I/O, concurrency, or durability.

The bounded suite is complementary evidence, not a substitute for formal
verification. The Kani portfolio is:

```text
cargo kani -p dwv-core --harness byte_range_constructor_matches_checked_add
cargo kani -p dwv-core --harness geometry_512_acceptance_is_exact_and_reachable
cargo kani -p dwv-core --harness geometry_4096_acceptance_is_exact_and_reachable
cargo kani -p dwv-service --harness split_range_math_preserves_aligned_coverage
cargo kani -p dwv-codec --harness full_parity_matches_explicit_xor
cargo kani -p dwv-codec --harness incremental_update_matches_full_recomputation
cargo kani -p dwv-codec --harness fixed_single_erasure_reconstructs_exactly
```

Observed: all seven harnesses completed with no failed checks. Geometry
reachability covers for valid and invalid inputs were satisfied. The direct
`Vec`-backed service harness was canceled after symbolic execution became
pathological; the replacement fixed-array arithmetic harness completed in
2.66 seconds and supports `verify.claim.checked-addressing` bounded
list/allocation and error-formatting behavior remains covered by the bounded
Rust tests and later property/fuzz layers. Kani diagnostics for
`caller_location` and foreign functions were emitted as successful checks,
not proof failures.

Kani claim map: the core and service harnesses support
`verify.claim.checked-addressing`; the codec parity, incremental-update, and
single-erasure harnesses support `verify.claim.parity-exact`.
These runs do not establish arbitrary-width `u64` proof, P/Q behavior,
concurrency, recovery ordering, filesystem/device I/O, or physical durability.

## verify.sustained-fuzz-corpus initial structured corpus evidence

Source: `verification/corpus/trace-seeds.json` and the seeded
`dwv-trace` parser-mutation test.

Command:

```text
cargo test -p dwv-trace --lib seeded_trace_corpus_round_trips_and_rejects_or_normalizes_mutations
```

Observed: 24 retained seeds generate valid traces across 512-byte through
16 KiB fixtures with bounded writes and optional read, degraded-read, rebuild,
checksum, and terminal-outcome events. Each generated trace round-trips through
JSON. Deterministic byte mutations are accepted only when the parser produces
a trace that also canonical-round-trips; malformed or invalid mutations are
refused. The corpus contains seeds, not raw payloads.

This is initial `verify.claim.hostile-input-safe-formats` and
`verify.claim.reproducible-failure-evidence` evidence. It does not claim broad
simulator operation/fault/crash/topology schedules, parity generation,
concurrency, filesystem behavior, or hardware durability.

## verify.sustained-fuzz-corpus seeded simulator schedule corpus evidence

Source: `verification/corpus/simulator-schedule-seeds.txt` and the seeded
`dwv-sim` schedule/replay test.

Command:

```text
cargo test -p dwv-sim --lib seeded_operation_and_fault_schedules_replay_stably
```

Observed: 16 retained seeds generate bounded simulator schedules with
operation submissions, delivery, write faults, recovery/envelope commits,
latent corruption, daemon/controller interruption, power loss, and store
disappearance/reappearance. Every schedule round-trips through the simulator
reproducer format, replays deterministically, drains pending work, and ends
with the store available. The corpus retains seeds and producer logic, not
payload data.

This supports initial `verify.claim.uncertainty-remains-explicit`,
`verify.claim.conservative-deterministic-recovery`,
`verify.claim.hostile-input-safe-formats`, and
`verify.claim.reproducible-failure-evidence` evidence. It does not claim
exhaustive schedule coverage, topology generation, parity equivalence,
concurrency, filesystem behavior, or hardware durability.

## verify.sustained-fuzz-corpus bounded parity, topology, and envelope corpus evidence

Sources: `verification/corpus/parity-seeds.txt`,
`verification/corpus/topology-seeds.txt`, and
`verification/corpus/envelope-mutation-seeds.txt`.

Commands:

```text
cargo test -p dwv-codec seeded_parity_corpus_matches_reconstruction_equations
cargo test -p dwv-core seeded_topology_candidates_reject_duplicate_assignments
cargo test -p dwv-format seeded_envelope_mutations_fail_closed
```

Observed: eight retained parity seeds drive 128 generated geometries and
single-erasure reconstructions; eight topology seeds drive reordered valid
candidates and duplicate-assignment refusals; eight envelope seeds mutate
magic, version, header, profile, slot, body, and padding offsets, all of which
are rejected. The artifacts retain seed values and producer logic, not payload
bytes.

This extends bounded `verify.claim.parity-exact`,
`verify.claim.checked-addressing`, `verify.claim.topology-binding-stable`,
`verify.claim.hostile-input-safe-formats`, and
`verify.claim.reproducible-failure-evidence` evidence. The corpus remains
finite and does not claim arbitrary-width proof, exhaustive topology space,
concurrency, filesystem behavior, Linux behavior, or hardware durability.


## Integrated portable checkpoint

Disposable root: `/tmp/dwv-v3-followup`

Commands, in order:

```text
cargo run -q --bin dwv -- demo init --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo run --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo status --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo inspect --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo capabilities --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo verify --root /tmp/dwv-v3-followup
cargo run -q --bin dwv -- demo trace-export --root /tmp/dwv-v3-followup --trace verification-trace.json
cargo run -q --bin dwv -- demo trace-replay --root /tmp/dwv-v3-followup --trace verification-trace.json
```

Observed:

- `demo.init`, `demo.run`, `demo.status`, `demo.inspect`, `demo.capabilities`,
  `demo.verify`, `demo.trace-export`, and `demo.trace-replay` all succeeded.
- Healthy write/read/flush/reopen matched.
- Degraded read matched the reference; four rebuild chunks completed; final
  verification passed; replacement bytes equaled the reference; source parity
  remained unchanged.
- Exhaustive verification reported clean with zero writes.
- Trace export/render/replay remained equivalent with 11 events.
- The CLI continued to report live bridge, Linux, physical durability, P/Q,
  and degraded writes as unsupported.

## Post-verify.bounded-arithmetic integrated checkpoint

Disposable root: `/tmp/dwv-v3-ve001`

Commands, in order:

```text
cargo run -q --bin dwv -- demo init --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo run --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo status --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo inspect --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo capabilities --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo verify --root /tmp/dwv-v3-ve001
cargo run -q --bin dwv -- demo trace-export --root /tmp/dwv-v3-ve001 --trace verification-trace.json
cargo run -q --bin dwv -- demo trace-replay --root /tmp/dwv-v3-ve001 --trace verification-trace.json
```

Observed: all eight commands succeeded. Healthy read/write/flush/reopen
matched; degraded read and four-chunk rebuild matched the reference; final
verification passed; replacement bytes matched; source parity remained
unchanged; exhaustive verification reported clean with zero writes; and
trace replay was equivalent across 11 events. Unsupported bridge, Linux,
physical durability, P/Q, and degraded-write claims remained explicit.

## Repository checks


```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo metadata --format-version 1 --no-deps
openspec validate --all --strict --json
```

Observed: 232 tests passed across 29 suites with one ignored; the three new
corpus tests and lifecycle-focused suites passed; formatting, workspace
Clippy, metadata, and strict OpenSpec validation passed. OpenSpec validation
reported 21/21 items valid; informational long-requirement notices are not
failures.


## v0.8 simulator-layer correction checkpoint

Date: 2026-08-08

Change under test: `crates/dwv-sim/src/media.rs` now contains the
role-neutral `MediaSimulator`, `MediaSchedule`, and `MediaTrace` state. The
DiskWeave-specific `Simulator` in `crates/dwv-sim/src/lib.rs` adapts protocol
schedules and retains recovery-database and parity-envelope fixture state.

Focused commands:

```text
cargo test -p dwv-sim
cargo check --workspace
```

Observed: 28 `dwv-sim` tests passed, including
`media_model_replays_without_protocol_state`; workspace compilation passed.
The focused media test exercises volatile write acknowledgement, flush
persistence, zero pending operations, and deterministic replay without
constructing protocol recovery or envelope state.

Integrated commands:

```text
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo metadata --format-version 1 --no-deps
openspec validate --all --strict --json
```

Observed: 233 workspace tests passed with one ignored; Clippy, formatting,
metadata, and strict OpenSpec validation passed. OpenSpec validation reported
21/21 items valid with informational long-requirement notices only.

Portable CLI evidence used disposable root `/tmp/dwv-v4.KqoBWQ`. `demo init`,
`demo run`, `demo status`, `demo inspect`, `demo capabilities`, `demo verify`,
`demo trace-export`, `demo trace-render`, and `demo trace-replay` all
succeeded. Healthy read/write/flush/reopen, degraded read, four-chunk
separate-target rebuild, exhaustive clean verification, and 11-event
trace-render/replay equivalence remained intact. The output continued to
bound claims to portable ordinary-file behavior and explicitly exclude live
bridges, Linux frontends, physical power-loss durability, P/Q, and degraded
writes.

## LifecycleRelease model-to-Rust projection

This table records the production source for each of the seven delegated
observations. The exact generation is always the
`OperationSlotToken { index, generation }` carried by
`ReleaseReconciliation.operation` and `ReleaseAuthorization.operation`.
Applicability is a separate admission-scope binding, not one of the seven observations.
`HealthyPortableService` records `ReleaseScope::Outside` for the exact reserved
generation and changes it to `ReleaseScope::InScope` only after the transaction
machine accepts its explicit `RangeAcquired` owner result; routine reads and
flushes never make that transition. The current portable service supplies the
typed transaction result at this composition seam and does not claim a concrete
range-lock implementation. Missing or generation-mismatched scope state is
`Unknown` and fails closed.

| Model observation | Final owner | Exact-generation binding | Production observation/type/path | Missing or uncertain evidence | Why the model consumes it |
| --- | --- | --- | --- | --- | --- |
| Operation/media effect terminal or authoritatively reconciled | `dwv-store::OperationSlotTable` plus service reconciliation | `OperationEffectObservation.operation` / `SlotSnapshot.token` | Normal terminality comes from `HealthyPortableService::operation_effect_observation`, which projects the exact-token slot snapshot to `Terminal` or `Unresolved`; `AuthoritativelyReconciled` remains an explicit external reconciliation input | Nonterminal or uncertain child evidence yields `Unresolved`; stale generation lookup fails; missing authoritative reconciliation cannot be promoted to terminality | The normal service and Connect paths consume one named owner projection instead of inferring terminality from frontend success or injecting `Terminal` |
| Children terminal | `store-operation-contracts` / `OperationSlotTable` | `SlotSnapshot.token` and each `ChildOperationSnapshot.operation_id` | `SlotSnapshot.children[*].terminal`, set by `OperationSlotTable::apply_completion` and exposed through `OperationAdmission::snapshot` | Any nonterminal, unknown, stale, or uncertain child remains unresolved | The lifecycle seam consumes child terminal evidence rather than counting children or inferring completion from a parent result |
| Required reconciliation recorded | `store-operation-contracts` | `SlotSnapshot.token` | `SlotSnapshot.reconciliation` from `OperationAdmission::record_reconciliation` | `None`, `UncertainRetained`, or `Invalidated` is not the required durable observation | `authorization_candidate` consumes the exact `ReconciliationOutcome::Durable` record |
| Safe generation-qualified `Reclaimable` | `store-operation-contracts` | `SlotSnapshot.token` | `SlotSnapshot.state == SlotState::Reclaimable`, established by `OperationSlotTable::record_reconciliation` | Any other slot state, required drain, or stale token fails closed; zero children are valid when the operation entered release scope before child registration | The service consumes the owner state; physical `release()` is separate and cannot create or revoke authorization |
| Applicable transaction-release requirement satisfied | `explicit-transaction-machine` / `RecoveryProtocol` where applicable | The typed requirement is bound to `ReleaseReconciliation.operation`; only an exact generation already marked `ReleaseScope::InScope` can consume it | `ReleaseRequirement`; successful traces pass through `transaction_requirement` and affirmative `transaction_release_complete`; authoritative reconciliation supplies `TransactionSatisfied` | `TransactionUnresolved`, missing trace, reconciliation-required trace, or absent affirmative release result withholds authorization; `NotApplicable` is used only by operations outside the lifecycle-release scope | The model consumes an owner-approved satisfaction value, not raw trace presence, operation kind, child count, or a generic `transaction_required` flag |
| Authoritative recovery-owned reconciliation | `recovery-state-semantics` | `ReleaseReconciliation.operation` plus the retained slot snapshot | `RecoveryReconciliation::Authoritative` from `recovery_reconciliation_observation` or an explicit owner-approved reconciliation input | Unhealthy recovery, failed snapshot, topology/generation mismatch, or `RecoveryReconciliation::Unresolved` fails closed | Recovery authority is supplied at the lifecycle seam; the consumer does not inspect recovery internals or reconstruct the decision |
| Healthy-service basis conformance | healthy-service composition | Stored as `(OperationSlotToken, BasisConformance)` in the bounded service observation table | `BasisConformance::{Consumed, Discarded, Reconciled}`; successful write records `Consumed`, and authoritative reconciliation supplies the typed observation; `basis_conformance_observation` performs exact-token lookup | No recorded observation or `BasisConformance::Unresolved` withholds authorization; recovery/topology/checksum-generation coherence remains a separate fact | The model consumes an explicit basis-lifetime observation rather than deriving it from synchronous control flow or recovery-generation equality |

Focused service evidence covers routine out-of-scope operations, missing and
generation-mismatched scope state, in-scope zero-child reconciliation, failed
pre-child admission, and authorization retention during unrelated work.
This table is a bounded owner-observation projection record, not an
implementation-conformance claim by itself. The Connect projection below
compares the same facts with real service results without manufacturing any
missing owner fact.
### LifecycleRelease Connect projection

`crates/dwv-service/src/service/tests/lifecycle_connect.rs` is a
`cfg(test)` in-crate correspondence driver. It keeps a normalized
exact-generation ledger only for facts already witnessed during its bounded
trace. The ledger is verification state, not a production lifecycle store.

The driver maps `enterApplicable` to a real transaction-owner
`RangeAcquired` result followed by the exact-token `ReleaseScope::InScope`
observation. On normal paths it retains that exact transaction machine through
`ReleaseRange` and derives `TransactionSatisfied` through the production
`transaction_requirement(trace)` mapping. It completes a registered/submitted
child before observing normal media terminality through
`operation_effect_observation`; child terminality itself is checked from the
store-owned `SlotSnapshot`. Required reconciliation is recorded through the
real admission table, and `Reclaimable` is observed only from a later
exact-token snapshot. `establishReleaseAllowed` invokes the real reconciliation
path and requires the returned or retained `ReleaseAuthorization` to carry the
same exact token.

The Connect boundary distinguishes direct provider correspondence from external
typed owner inputs:

| LifecycleRelease input | Connect classification |
| --- | --- |
| applicability / acquisition | direct provider correspondence from `RangeAcquired` plus exact-token `ReleaseScope` |
| normal media terminality | direct provider correspondence through exact-token `operation_effect_observation` |
| authoritative media reconciliation | external typed owner input; this bridge does not claim its upstream proof |
| children terminal | direct provider correspondence from registered/submitted/completed child state and `SlotSnapshot`; a separate reconciled path proves zero children remain valid |
| required reconciliation / `Reclaimable` | direct provider correspondence from `OperationAdmission` and exact-token `SlotSnapshot` |
| transaction release satisfied | direct provider correspondence from the retained transaction trace on normal paths; external typed owner input after authoritative reconciliation |
| recovery authoritative | direct provider correspondence from `recovery_reconciliation_observation` on the normal service seam |
| basis conformance | typed owner input consumed by Connect; focused service evidence separately proves normal successful write records exact-token `BasisConsumed` |
| `ReleaseAuthorization` | direct provider correspondence from the real lifecycle composition path |
| cleanup request/result | direct provider correspondence from the real cleanup call/result, retained only in the bounded verification ledger |

Passing an external enum into lifecycle composition is not treated as proof that
the upstream owner produced that enum correctly.

The service currently combines authorization with the physical cleanup call.
The driver retains that actual success or failure result, then reveals cleanup
request and result as separate model observations. A successful cleanup is not
re-queried after the live slot, scope, and basis disappear. A failed cleanup
rechecks only the still-live exact token, slot, and retained certificate.

The projection includes successful in-scope authorization and cleanup with
a non-vacuous registered/submitted/completed child, separate zero-child
reconciliation, incomplete owner facts, isolated unresolved-recovery and
unresolved-basis probes, a pre-acquisition write, a genuinely out-of-scope
routine read, and stale-generation rejection. It excludes abstract simultaneous
same-operation generation coexistence and historical authorization retention
after physical slot reuse. Those remain canonical-model and focused-Rust
evidence; they are not production claims made by this Connect projection.

## LifecycleRelease bounded evidence

Commands:

```text
quint typecheck models/quint/LifecycleRelease.qnt
quint typecheck verification/quint/LifecycleReleaseAnalysis.qnt
quint typecheck verification/quint/LifecycleReleaseMutants.qnt
quint typecheck verification/quint/LifecycleReleasePrefix.qnt
quint typecheck verification/quint/LifecycleReleaseTwoGenerationPrefix.qnt
quint typecheck verification/quint/LifecycleReleaseIndependentPrefix.qnt
quint typecheck verification/quint/LifecycleReleaseConnect.qnt
cargo test -p dwv-service --lib lifecycle_release_connect
quint test verification/quint/LifecycleReleaseAnalysis.qnt
quint test verification/quint/LifecycleReleaseMutants.qnt
quint test verification/quint/LifecycleReleaseTwoGenerationPrefix.qnt \
  --main LifecycleReleaseTwoGenerationPrefix \
  --match '^authorizationSurvivesSecondGeneration$' --max-samples 1
quint test verification/quint/LifecycleReleaseIndependentPrefix.qnt \
  --main LifecycleReleaseIndependentPrefix \
  --match '^authorizationSurvivesUnrelatedProgress$' --max-samples 1
quint verify verification/quint/LifecycleReleasePrefix.qnt \
  --main LifecycleReleasePrefix --max-steps 9 \
  --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity
quint verify verification/quint/LifecycleReleasePrefix.qnt \
  --main LifecycleReleasePrefix --max-steps 11 \
  --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity
quint verify verification/quint/LifecycleReleaseTwoGenerationPrefix.qnt \
  --main LifecycleReleaseTwoGenerationPrefix --init profileInit \
  --step profileStep --max-steps 10 \
  --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity
quint verify verification/quint/LifecycleReleaseTwoGenerationPrefix.qnt \
  --main LifecycleReleaseTwoGenerationPrefix --init profileInit \
  --step profileStep --max-steps 18 \
  --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity
quint verify verification/quint/LifecycleReleaseIndependentPrefix.qnt \
  --main LifecycleReleaseIndependentPrefix --init profileInit \
  --step profileStep --max-steps 12 \
  --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity
quint run verification/quint/LifecycleReleaseAnalysis.qnt \
  --main LifecycleReleaseAnalysis --max-steps 26 --max-samples 100000 \
  --seed 22082026 --invariants TypeInvariant AuthorizationRequiresApplicability \
    AuthorizationRequiresAllOwnerFacts MissingOrIncompleteEvidenceWithholds \
    CleanupRequestPreservesAuthorization CleanupFailurePreservesAuthorization \
    CleanupSuccessPreservesAuthorization CleanupResultsRequireAuthorization \
    AuthorizationUsesExactIdentity \
  --witnesses AuthorizationReachable CleanupRequestReachable \
    CleanupFailureReachable CleanupSuccessReachable \
    AuthoritativeReconciliationReachable GenerationIsolationReachable \
    UnrelatedOperationReachable
```

Observed:

- The analysis suite passed 11/11 scenarios; the mutant suite passed 12/12
  negative scenarios. Each focused phase-cut profile witness passed 1/1
  deterministic run.
- Both existing one-generation exhaustive profiles returned `NoError` at
  max-steps 9 and 11.
- The unconstrained two-generation profile did not complete at max-steps 10
  within 1,200 seconds. The replacement profile constrains only verification
  reachability with `profileInit`/`profileStep`, starts from canonical `init`,
  and reaches the prefix through imported model actions. It returned `NoError`
  at max-steps 10 (one authorization plus the second active generation) and
  18 (both exact generations authorized). The two-operation phase-cut profile
  returned `NoError` at max-steps 12 after unrelated work entered and
  progressed.
- The sampled run returned no invariant violation across 100,000 traces.
  Witness counts were: authorization 728/100,000; cleanup request
  52/100,000; cleanup failure 2/100,000; cleanup success 2/100,000;
  authoritative reconciliation 91,116/100,000; generation isolation
  494/100,000; unrelated operation 727/100,000.
- The repaired LifecycleRelease Connect projection passed all three
  deterministic paths (one sample each): successful in-scope authorization
  and cleanup with a registered/submitted/completed child; injected cleanup
  failure with retained exact-generation authorization; and negative probes
  for pre-acquisition write, routine out-of-scope read, stale generation,
  incomplete facts, isolated unresolved recovery, and isolated unresolved
  basis.
- Focused provider regressions distinguish a known failed terminal child
  (`OperationEffect::Terminal`) from an uncertain child
  (`OperationEffect::Unresolved`). The normal successful write path publishes
  exact-generation `OperationEffect::Terminal` and `BasisConsumed`; normal
  Connect transaction satisfaction comes from the retained machine trace
  through `transaction_requirement(trace)`.
- The model evidence is portable bounded-model evidence. It is not Rust
  implementation-conformance or hardware/durability proof. Abstract
  same-operation generation coexistence tests exact correlation only; it does
  not model physical slot reuse ordering.


# CodedRangeClean delegated-model evidence (executed, bounded)

Date: 2026-08-20

Sources:

- `models/quint/CodedRangeClean.qnt`
- `verification/quint/CodedRangeCleanAnalysis.qnt`
- `verification/quint/CodedRangeCleanReplay.qnt`
- `verification/quint/CodedRangeCleanMutants.qnt`
- `verification/quint/CodedRangeCleanConflictExhaustive.qnt`
- `verification/quint/CodedRangeCleanCaptureExhaustive.qnt`
- `verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt`
- `verification/quint/CodedRangeCleanCompositionExhaustive.qnt`
- `verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt`
- `verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt`
- `verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt`

Semantic and delegation repairs:

- The canonical relation stores one capture-wide
  `CaptureSatisfactionObservation` supplied by the CLEAN owner. It does not
  retain or recompute a generic handoff, per-operation satisfaction map, or
  separate `CleanEvidence` axis. `DispositionDurableHandoff` alone has no
  CLEAN meaning.
- `CaptureCommitUnknown` is the sole stored CLEAN-commit uncertainty phase.
  `CleanCommitObservation` is an action input; durable and rejected outcomes
  become `CaptureCleanKnown` and `CaptureRefused`, respectively. Reconciliation
  is a separate authoritative action.
- Later-cut outcomes remain orthogonal membership statuses. Later-cut Unknown
  is distinct from CLEAN-commit Unknown and blocks active later effects. Before
  durable CLEAN it may reconcile only to a stale/refused boundary or rejection;
  after durable CLEAN it may reconcile only to a durable-after-CLEAN cut or
  rejection, preserving `CaptureCleanKnown`.
- Definitive `CaptureRefused` retains conservative dirty/stale consequences
  and stops future-admission classification and durable-cut obligations. No
  retirement/compaction state is modeled because no current or proposed owner
  requirement establishes one.
- Exact external `ReleaseAllowed` is the sole coded-removal authority.
  Capture uncertainty neither supplies nor negates that external fact.
  `RecoveryProtocol.qnt`, its analysis, and its Connect evidence remain
  separate and unchanged.
- `init` is setup rather than an ordinary protocol step; repeated/no-op
  observations are rejected; nondeterministic identifiers are scoped to
  consuming actions; redundant stored facts were removed only where the
  canonical phase/membership or external owner already carries the exact
  meaning.

The declaration-level projection map in
`openspec/changes/define-coded-range-clean-coordination/design.md` names every
model distinction and its implemented Connect seam or external owner. The
Connect bridge is covered by the separate projection record below; it does
not promote the bounded model into a second production authority.

Static checks:

```text
quint typecheck models/quint/CodedRangeClean.qnt
quint typecheck verification/quint/CodedRangeCleanAnalysis.qnt
quint typecheck verification/quint/CodedRangeCleanReplay.qnt
quint typecheck verification/quint/CodedRangeCleanMutants.qnt
quint typecheck verification/quint/CodedRangeCleanConflictExhaustive.qnt
quint typecheck verification/quint/CodedRangeCleanCaptureExhaustive.qnt
quint typecheck verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt
quint typecheck verification/quint/CodedRangeCleanCompositionExhaustive.qnt
quint typecheck verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt
quint typecheck verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt
quint typecheck verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt
=> all eleven exited 0 with no output
```

Deterministic scenarios:

```text
quint test verification/quint/CodedRangeCleanAnalysis.qnt --main CodedRangeCleanAnalysis \
  --match '^(boundedAssumptionsTest|disjointClaimsCanCoexist|sameCodedClaimConflicts|nonTransitiveOverlapLeavesDisjointClaimsAvailable|incompleteClaimCannotReachEffect|unadmittedClaimCannotReachEffect|captureMembershipIsIncludedThenLater|includedOperationCanCommitAndClean|missingCleanDecisionCannotClean|cleanOwnerDecisionEnablesClean|rejectedCleanDecisionRefusesCapture|unknownCleanDoesNotNegateExternalRelease|unknownCleanCanReconcileDurably|laterMutationUsesDurableAfterCleanCut|unknownLaterCutAfterDurableCleanReconcilesDurably|unknownLaterCutBeforeDurableCleanBlocksCleanCommit|unknownLaterCutAfterDurableCleanReconcilesRejected|unknownLaterCutAfterDurableCleanCannotReconcileStale|staleLaterCutAfterDurableCleanIsRejected|newerDurableBoundaryStalesOlderClean|resolvingOneCaptureDoesNotDischargeAnother|unknownLaterCutBeforeCleanCannotReconcileAfterClean|rejectedLaterCutRefusesEffect|removalNeedsExactExternalAuthorization|releasedHistoryNeedNotRemainEnumerable|activeHistoryRemainsEnumerable)$' \
  --max-samples 1 --seed 22082026
=> 26 passing

quint test verification/quint/CodedRangeCleanConflictExhaustive.qnt \
  --main CodedRangeCleanConflictExhaustive \
  --match 'bridgeRefusedWhileBothConflictsHeld|bridgeRefusedUntilUnit0ConflictReleases|nonTransitiveBridgeWaitsForBothConflicts' \
  --max-samples 1 --seed 22082026
=> 3 passing

quint test verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt \
  --main CodedRangeCleanCleanCommitPhaseCut \
  --match 'durableCommitPhaseCut|rejectedCommitPhaseCut' \
  --max-samples 1 --seed 22082026
=> 2 passing

quint test verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt \
  --main CodedRangeCleanLaterCutPhaseCut \
  --match 'durableAfterCleanLaterCutPhaseCut|rejectedLaterCutPhaseCut' \
  --max-samples 1 --seed 22082026
=> 2 passing

quint test verification/quint/CodedRangeCleanCompositionExhaustive.qnt \
  --main CodedRangeCleanCompositionExhaustive \
  --match 'compositionDurablePath|compositionRejectedPath' \
  --max-samples 1 --seed 22082026
=> 2 passing
```

These scenarios cover coded overlap/disjoint coexistence, complete and
partial admission, exhaustive Included/Later membership, one external
capture-wide CLEAN decision, definitive refusal, CLEAN-commit durable/
rejected/unknown paths, independent external release during CLEAN-commit
Unknown, later-cut durable/rejected/unknown paths, the valid post-CLEAN
Unknown interval, refusal to commit CLEAN over a pre-existing later-cut
Unknown, rejection of reconciliation results from the wrong side of durable
CLEAN, independent obligations across two captures, stale refusal, exact
removal authorization, and the full composition predicates.

Small exhaustive profiles use the same imported canonical relation. Domains
and exact completed bounds are:

```text
quint verify verification/quint/CodedRangeCleanConflictExhaustive.qnt \
  --main CodedRangeCleanConflictExhaustive --max-steps 3 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 3

quint verify verification/quint/CodedRangeCleanCaptureExhaustive.qnt \
  --main CodedRangeCleanCaptureExhaustive --max-steps 5 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 5

quint verify verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt \
  --main CodedRangeCleanUncertaintyExhaustive --max-steps 4 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 4

quint verify verification/quint/CodedRangeCleanCompositionExhaustive.qnt \
  --main CodedRangeCleanCompositionExhaustive --max-steps 6 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 6

quint verify verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt \
  --main CodedRangeCleanTwoCapturePhaseCut --init profileInit --max-steps 14 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 14
```

The uncertainty profile also produced direct sampled witness evidence at the
completed depth:

```text
quint run verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt \
  --main CodedRangeCleanUncertaintyExhaustive --max-steps 4 \
  --max-samples 100000 --seed 22082026 --invariants ProfileInvariants \
  --witnesses cleanCommitUnknownAtCompletedBoundReachable \
    laterCutUnknownAtCompletedBoundReachable --verbosity 1
=> no violation; 100000 traces explored
=> cleanCommitUnknownAtCompletedBoundReachable: 683 traces
=> laterCutUnknownAtCompletedBoundReachable: 4221 traces
```

The conflict profile has three operation identities (`opA`, `opBridge`,
`opB`), one capture identity, and two coded units. The capture profile has
two operations, one capture, and one coded unit; its strongest completed
bound is depth 5. The uncertainty profile has two operations, one capture,
and one coded unit; both `CleanCommitUnknown` and `LaterCutUnknown` are
witnessed from canonical `init` at its completed depth 4. That bound does not
reach every deeper reconciliation prefix, so the explicit phase-cut profiles
carry that evidence. The composition profile has two operations, one capture,
and two coded units; its strongest completed bound is depth 6. The two-capture
phase-cut profile has two operations, two captures, and one coded unit; its
fixed `profileInit`/`step` phase cut completed at depth 14 and requires both
applicable durable later cuts before the later effect.

The attempted capture depth 7, composition depth 13, and unconstrained
two-capture exploration timed out without a result and are not claimed. This
two-capture result is only the declared fixed phase cut; no broader
multi-capture bound is claimed.

Phase-cut profiles were checked with their actual `phaseStep` action:

```text
quint verify verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt \
  --main CodedRangeCleanCleanCommitPhaseCut --step phaseStep --max-steps 6 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 6

quint verify verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt \
  --main CodedRangeCleanLaterCutPhaseCut --step phaseStep --max-steps 6 \
  --invariants ProfileInvariants --verbosity 0
=> no violation found; completed depth 6
```

The two-capture necessity checks also cover the negative prefixes for two overlapping captures:

```text
quint test verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt \
  --main CodedRangeCleanTwoCapturePhaseCut \
  --match '^(noCaptureCutDoesNotPermit|oneCaptureCutDoesNotPermit)$' \
  --max-samples 1 --seed 22082026
=> 2 passing; effect is refused before either cut and after only capture0's cut

cargo test -p dwv-recovery every_applicable_capture_must_satisfy_its_later_cut_before_effect -- --nocapture
=> 1 passed; production coordinator requires both applicable overlapping cuts

cargo test -p dwv-recovery disjoint_active_capture_does_not_gate_later_effect -- --nocapture
=> 1 passed; a disjoint active capture creates no membership or effect gate
```

The Quint checks use the existing task-4.1 relation and the task-4.3
capture/frontier relation. The production task-4.3 path separately persists
capture state, integrates durable completion, and commits `MarkRegionClean`
with the accepted CLEAN decision.

The later phase-cut depth 10 attempt timed out at 300 seconds and is not
claimed. Both profiles begin at canonical `init` and establish their modeled
prefixes; they do not establish external lifecycle preconditions.

Sampled invariant and witness analysis:

```text
quint run verification/quint/CodedRangeCleanAnalysis.qnt --main CodedRangeCleanAnalysis \
  --max-steps 24 --max-samples 100000 --seed 22082026 \
  --invariants AllBoundedInvariants \
  --witnesses completeClaimAdmissionReachable overlapConflictReachable disjointClaimsCoexistReachable captureIncludedReachable captureLaterReachable captureSatisfactionAcceptedReachable captureSatisfactionRejectedReachable effectPossibleReachable releasedReachable cleanCommitPendingReachable cleanKnownReachable cleanRefusedReachable cleanUnknownReachable cleanReconciledReachable laterCutAfterCleanReachable laterCutStalesCleanReachable laterCutRejectedReachable laterCutUnknownReachable laterCutReconciledRejectedReachable laterCutReconciledAfterCleanReachable conservativeDirtyReachable releasedHistoryOmittedReachable activeHistoryRetainedReachable externalRemovalUnderCleanUnknownReachable \
  --verbosity 1
=> no violation; 100000 traces explored
```

All 24 requested witnesses were non-zero:

| Witness | Count |
|---|---:|
| `completeClaimAdmissionReachable` | 100000 |
| `overlapConflictReachable` | 52834 |
| `disjointClaimsCoexistReachable` | 25567 |
| `captureIncludedReachable` | 17014 |
| `captureLaterReachable` | 27523 |
| `captureSatisfactionAcceptedReachable` | 45984 |
| `captureSatisfactionRejectedReachable` | 46010 |
| `effectPossibleReachable` | 36213 |
| `releasedReachable` | 99999 |
| `cleanCommitPendingReachable` | 38839 |
| `cleanKnownReachable` | 15872 |
| `cleanRefusedReachable` | 84127 |
| `cleanUnknownReachable` | 11899 |
| `cleanReconciledReachable` | 15872 |
| `laterCutAfterCleanReachable` | 2482 |
| `laterCutStalesCleanReachable` | 7614 |
| `laterCutRejectedReachable` | 10230 |
| `laterCutUnknownReachable` | 6672 |
| `laterCutReconciledRejectedReachable` | 10230 |
| `laterCutReconciledAfterCleanReachable` | 2482 |
| `conservativeDirtyReachable` | 84127 |
| `releasedHistoryOmittedReachable` | 82986 |
| `activeHistoryRetainedReachable` | 17014 |
| `externalRemovalUnderCleanUnknownReachable` | 7396 |

Deterministic replay:

```text
quint test verification/quint/CodedRangeCleanReplay.qnt --main CodedRangeCleanReplay \
  --match replaySeededPath --max-samples 1 --seed 22082026
=> replaySeededPath passed 1 test

quint run verification/quint/CodedRangeCleanReplay.qnt --main CodedRangeCleanReplay \
  --max-steps 12 --max-samples 1 --n-traces 1 --seed 22082026 \
  --out-itf /tmp/coded-range-clean-replay-a.itf.json --verbosity 0
quint run verification/quint/CodedRangeCleanReplay.qnt --main CodedRangeCleanReplay \
  --max-steps 12 --max-samples 1 --n-traces 1 --seed 22082026 \
  --out-itf /tmp/coded-range-clean-replay-b.itf.json --verbosity 0
=> normalized traces byte-identical after generated metadata removal; both
   SHA-256 = 3d52891d2c5c52b639dd0e622b08ab76d24a658690ea7c0240b2364e4e3b19ad
```

Negative mutants:

```text
quint test verification/quint/CodedRangeCleanMutants.qnt \
  --main CodedRangeCleanMutants \
  --match 'perMemberInsteadOfCodedConflictFails|connectedComponentSerializationKillsDisjointWitness|partialAdmissionBeforeEffectFails|unclassifiedAdmissionFailsExhaustiveness|disappearingIncludedOperationFailsRetention|laterEffectWithoutCutFails|unsatisfiedIncludedWorkCannotBeClean|rejectedSatisfactionCannotRemainActive|unknownCleanCannotBecomeKnown|unknownLaterCutCannotAllowEffect|staleCaptureCannotRemainClean|removalWithoutExternalReleaseAllowedFails' \
  --max-samples 1 --seed 22082026
=> all 12 deliberately bad paths failed their expected safety predicate
   (`QNT508`); command exit is intentionally non-zero for this negative suite
```

Mutant mappings are coded overlap, disjoint-claim serialization, admission
completeness, exhaustive membership, later durable-cut gating, no-false-CLEAN,
rejected decision refusal, CLEAN-commit Unknown reconciliation, later-cut
Unknown gating, stale-capture refusal, and removal without exact external
`ReleaseAllowed`.

Non-claims and unresolved seams:

- This is bounded delegated-model evidence, not arbitrary-width proof, Rust
  correctness, Connect conformance, or production concurrency evidence.
- Completed end-to-end bounds are exactly depths 3, 5, 4, and 6 for conflict,
  capture, uncertainty, and composition; the fixed two-capture phase bound is
  depth 14. Phase-cut bounds are depth 6 each.
- Timed-out depth 7, depth 10, depth 13, and unconstrained two-capture
  attempts are not successes.
- Phase cuts begin at canonical `init` and establish their modeled prefixes;
  they do not establish external lifecycle preconditions or end-to-end
  production coverage.
- The evidence does not establish topology/profile mapping,
  request/store/dirty/checksum geometry, persistence admissibility,
  recovery-adapter correctness, lifecycle terminality, sessions, startup,
  shutdown, publication, currentization, physical locking, or hardware
  durability.
- `RecoveryProtocol.qnt` and its existing analysis/Connect evidence remain
  separate and are not evidence for this relation.
The model evidence remains bounded delegated-model evidence; the Connect
correspondence and Rust focused tests are recorded in the next section.
OpenSpec tasks 2.4 and 2.5 are complete. Task 4.1 is complete: the repaired
coded/CLEAN Connect candidate preserves non-terminal contention, consumes
opaque lifecycle authorization, and binds coded admission to a live pre-I/O
operation generation without reconstructing lifecycle predicates. Task 4.3 is
complete: production capture scope is derived from the selected dirty regions
and checksum extents under captured topology/profile geometry; CLEAN decisions,
durable commit outcomes, later write-recovery cuts, release authorization,
bounded retention, and unresolved reopen state are persisted through the
recovery owner.

# CodedRangeClean Connect projection (focused, bounded)

Date: 2026-08-23

Source:

- `crates/dwv-service/src/service/tests/coded_range_clean_connect.rs`
- `verification/quint/CodedRangeCleanConnect.qnt`
- `crates/dwv-transaction-ref/src/coded.rs`
- `crates/dwv-recovery/src/coded_clean.rs`
Canonical `models/quint/CodedRangeClean.qnt` SHA-256:
`7df99510cdfe985000ecca5cf5a66636252af67053f960fb16cbe6de06056873`.

The in-crate driver uses production `HealthyPortableService`,
`OperationAdmission`, `CodedRangeAuthority`, and
`CodedCaptureCoordinator` instances. `BridgeState` projects model state from
exact generation-qualified operation tokens and production coordinator
snapshots. The same topology/profile geometry mapping produces per-write coded
claims and future-inclusive capture scopes. The service holds a process-local
issuer bound to its exact coordinator and produces Included lifecycle evidence
only after validating the exact live slot/finalization/certificate or persisted
release receipt. Accepted Connect profiles use the persisted release-owner
path; they do not mint live lifecycle evidence from model state. Ordinary
overlap and capture exclusion are normalized non-terminal outcomes. Basis
coherence remains separate provider/conformance
evidence and is not a CodedRangeClean bridge input. The bridge does not
reconstruct either policy, retain a model-shaped coordinator, or use a second
semantic state machine.

Projection table:

| Model distinction | Direct provider correspondence | Typed external owner input |
|---|---|---|
| Exact operation identity, complete coded claim admission, disjoint coexistence, and effect phase | `BridgeDriver::admit` → `reserve` → `coded_admit`; production maps the requested range to exact coded units and requires the exact slot to remain `Reserved`; active phase/claims project from exact-token `CodedRangeAuthority`; released model claim fields retain only admitted claim evidence needed for comparison; `permit_effect` → `coded_permit_effect` | Complete claims are topology/profile geometry outputs; ordinary overlap is `CodedAdmissionOutcome::Contended`; blocker identity and scheduling are not part of this result |
| Capture scope, Included/Later membership, capture phase, and owner boundary | `start_capture` → `coded_start_capture`; production capture allocation derives the complete union of selected dirty regions and checksum extents under the same topology/profile geometry and durably persists the capture boundary; `bridge_state` → revalidated `CodedCaptureSnapshot` | `ValidatedCodedCaptureScope` binds current topology, recovery generation, checksum profile/set, and selected dirty/checksum geometry; `CodedCaptureEstablishment` binds owner-issued history and the atomic admission cut |
| Capture-wide accepted/rejected decision and no-false-CLEAN gating | The Connect-only transition adapter invokes test-sealed raw model actions. Production consumes `RecoveryCleanPermit` or `RecoveryCleanRefusalPermit` from `evaluate_recovery_clean`; accepted CLEAN and refusal are separately prepared and installed only under the exact `DurableRecoveryCommit` | Recovery CLEAN policy, fence/checksum/write-recovery coverage, and durable commit authority remain external owner facts |
| CLEAN commit uncertainty and authoritative reconciliation | Connect exercises Durable/Rejected/Unknown model transitions through the sealed test adapter. Production cannot pass those raw observations; uncertain persisted state changes only through `CodedCaptureReconciliationReceipt` issued by recovery inspection | Rejected or unknown outcomes do not authorize CLEAN; the receipt binds the expected uncertain snapshot and one exact resolved snapshot |
| Later durable-cut ordering, Unknown blocking, and reconciliation | Production prepares a cut from the admitted claim and persists it with the write-recovery transaction before permitting media effect. Accepted Connect paths call the same `ChecksumAuthority::invalidate_with_write_recovery_record_and_coded_transitions` producer; a mismatched-target negative control must fail there. The owner result is installed only under the matching `DurableRecoveryCommit`; uncertain cuts change only through `CodedCaptureReconciliationReceipt`. | Effect attempts return `BlockedByCapture` for unresolved `Later`/`LaterUnknown`; exact membership and cut evidence survive reopen. |
| Exact release authorization, capture-wide lifecycle evidence, membership compaction, and phase-specific capture cleanup | `release_operation` consumes exact lifecycle-owned `ReleaseAuthorization`; `CodedRangeAuthority::release` requires that capability in addition to `OperationReleasePermit`, verifies the service-held owner domain, then returns `CodedClaimRelease`, which production durably records before claim removal. The service supplies owner-domain-bound live/released `IncludedLifecycleAuthorization` only after validating exact write-driver finalization or persisted release receipts. The former process-global positive issuers are absent, and a foreign owner domain cannot authorize release or CLEAN. `PreparedCodedMembershipCompaction` forgets only resolved receipt-authorized membership. Distinct refused, clean-known, and inherited-clean-abandonment preparations remove only their exact predecessor after durable commit. Every process-local transition installs only after its matching durable receipt. | Lifecycle, recovery, geometry, retention, and capture-lifecycle authority remain distinct; no token, permit, capture identity, phase, foreign owner domain, or deserialized snapshot grants release, positive Included evidence, compaction, or cleanup authority. |

Authority table:

| Correctness assumption | Consumed capability or receipt | Producer | Consumer validation | Failure or uncertainty behavior |
|---|---|---|---|---|
| Per-write claims and capture invalidation scope cover the same complete coded geometry | Exact coded-unit sets and `ValidatedCodedCaptureScope` | Topology/profile range mapper and `CodedCaptureOwnerFacts::selected_invalidation_scope` | Independent brute-force oracles compare exact 512-byte coded units, 4-KiB dirty regions, 4-MiB checksum extents, boundaries, shared containers, and disjoint cases | Mapping overflow, incompleteness, empty scope, or owner-binding mismatch fails before admission or capture publication |
| Capture history includes every earlier active coded admission and no concurrent admission crosses the cut | `CodedCaptureEstablishment` | `CodedRangeAuthority::capture_boundary` | Scope binding is complete and non-empty; lower/capture frontiers share the capture recovery generation; admission sequences are monotonic and non-exhausted | Capture creation fails before durable publication |
| A coded claim may be removed only after the exact operation generation satisfies the complete lifecycle policy | Owner-domain-bound `ReleaseAuthorization` → `OperationReleasePermit` → `CodedClaimRelease` | LifecycleRelease composition; `OperationAdmission::release_permit`; `CodedRangeAuthority::release` | Authorization binds all required lifecycle observations to the exact generation and the verifier domain retained by the service; the prerequisite permit is opaque; coded authority requires a currently active matching claim. | Claim remains active; foreign authority domain, missing facts, stale generation, known non-commit, or lost acknowledgement fails closed until reopen reconciles. |
| CLEAN policy covers the exact capture regions, integrity extents, topology, fence, and recovery generation | `RecoveryCleanPermit` or `RecoveryCleanRefusalPermit` | `evaluate_recovery_clean` | `prepare_clean_commit` retains the permit's exact aggregate certificate and emits a complete transition with no caller-supplied evidence; `prepare_clean_refusal` matches complete owner facts, exact predecessor, and a closed mutation set. | Refusal persists conservatively; missing or mismatched evidence cannot become `CleanKnown`. |
| Every Included operation has an exact owner-observed lifecycle disposition and aggregate persistence proof | Owner-domain-bound `IncludedLifecycleAuthorization` plus its exact `FenceCertificate` and `RecoveryCleanPermit` | `HealthyPortableService` after exact write-driver finalization or persisted release-receipt validation | `CodedCaptureCoordinator::authorize_clean` requires the exact Included-operation set and rejects foreign authority domains, mismatched operations, stale topology/fence domains, uncovered certificates, and released/live dispositions that disagree with persisted owner state. Compile-fail checks cover the removed process-global positive issuers; runtime controls reject same-token capabilities from another owner domain. | The capture remains Open; no `CodedCaptureCleanAuthorization` or prepared CLEAN transition is issued. |
| A prepared transition became durable in the exact recovery transaction | `DurableRecoveryCommit` | `RecoveryStateStore::commit_durable_receipt` | CLEAN confirmation compares the exact committed `CodedCaptureTransition`, including its owner-approved aggregate certificate; other phase-specific confirmations match their exact committed update or removal plus generation and topology. | The process-local candidate is discarded; certificate substitution fails confirmation, and a lost acknowledgement installs neither candidate until exact predecessor/successor reopen reconciliation. |
| An uncertain CLEAN commit or later cut has one authoritative exact resolution | `CodedCaptureReconciliationReceipt` | Recovery inspection reconciliation | Expected snapshot must equal the live uncertain snapshot; proposed snapshot must be internally valid and match capture identity/topology. | Reconciliation fails without mutating durable state. |
| Reopen preserves every future-exclusion and release-retention obligation | Owner-bound reconstruction from persisted `CodedCaptureSnapshot` facts | Durable recovery store plus current topology/profile/checksum owner | Scope and lower-frontier coverage are recomputed; serialized proof flags are ignored; exact unresolved membership, cuts, release receipts, generations, and frontiers must agree. Restart never derives retained-history authority from `retained_frontier == lower_frontier`; no inherited retained-history assertion is trusted without a separate current owner witness. | Every inherited capture keeps the service Recovering until durable retirement. Because the current implementation cannot recreate that live witness after process loss, a settled inherited capture cannot authorize further compaction or clean cleanup and is conservatively abandoned by invalidating selected dirty/integrity state before removal. |
| Released membership may be forgotten and a settled capture removed only under exact owner authority | `PreparedCodedMembershipCompaction`, `PreparedCodedRefusedCleanup`, `PreparedCodedCleanKnownCleanup`, or `PreparedCodedInheritedCaptureAbandonment` | `CodedCaptureCoordinator` after current owner revalidation | Same-process compaction requires exact persisted release receipts and complete retained history. Refused cleanup binds the exact predecessor; inherited Open cleanup additionally consumes a current exact refusal; clean cleanup requires an empty owner-compacted durable-`CleanKnown` predecessor whose retained history remains live-owner-revalidated. Any predecessor reopened after process loss lacks that live history witness and takes inherited abandonment. | Membership or capture remains durably retained; known non-commit preserves the predecessor, lost acknowledgement installs neither local candidate, repeated crash before cleanup remains admission-blocking, and unverifiable inherited history is invalidated rather than treated as clean. |

The profiles cover complete/disjoint/overlapping and non-transitive coded
claims; ordinary overlap as non-terminal contention; incomplete and
unvalidated admission; stale-generation and unauthorized removal; capture
refusal; missing decision; CLEAN Durable/Rejected/Unknown and reconciliation;
later-cut ordering, direct and reconciled rejection, Unknown blocking, both
durable orderings, and independent release under unresolved capture state.

Focused checks:

```text
quint typecheck verification/quint/CodedRangeCleanConnect.qnt
=> exited 0 with no output

cargo test -p dwv-recovery coded_clean -- --nocapture
=> 10 passed

cargo test -p dwv-service coded_range_clean_connect -- --nocapture
=> 25 passed

cargo test -p dwv-service coded_capture -- --nocapture
=> 3 passed

cargo test -p dwv-service coded_same_generation_cannot_be_readmitted_after_release_before_slot_cleanup -- --nocapture
=> 1 passed
```

Repair verification on 2026-08-27:

```text
cargo test -p dwv-recovery
=> 58 passed

cargo test -p dwv-recovery-sqlite
=> 18 passed

cargo test -p dwv-service --lib
=> 108 passed

cargo test -p dwv-service --lib coded_range_clean_connect -- --nocapture
=> 25 passed

cargo test -p dwv-frontend-ublk fixture::tests::
=> 6 passed

quint typecheck models/quint/CodedRangeClean.qnt
=> exited 0 with no output

openspec validate --all --strict
=> 32 passed, 0 failed
```


Completion repair verification on 2026-08-29:

```text
quint typecheck models/quint/CodedRangeClean.qnt
quint typecheck verification/quint/CodedRangeCleanAnalysis.qnt
quint typecheck verification/quint/CodedRangeCleanConnect.qnt
quint test models/quint/CodedRangeClean.qnt
=> all exited 0

quint test verification/quint/CodedRangeCleanAnalysis.qnt \
  --main CodedRangeCleanAnalysis \
  --match '^(boundedAssumptionsTest|disjointClaimsCanCoexist|sameCodedClaimConflicts|nonTransitiveOverlapLeavesDisjointClaimsAvailable|incompleteClaimCannotReachEffect|unadmittedClaimCannotReachEffect|captureMembershipIsIncludedThenLater|includedOperationCanCommitAndClean|missingCleanDecisionCannotClean|cleanOwnerDecisionEnablesClean|rejectedCleanDecisionRefusesCapture|unknownCleanDoesNotNegateExternalRelease|unknownCleanCanReconcileDurably|laterMutationUsesDurableAfterCleanCut|unknownLaterCutAfterDurableCleanReconcilesDurably|unknownLaterCutBeforeDurableCleanBlocksCleanCommit|unknownLaterCutAfterDurableCleanReconcilesRejected|unknownLaterCutAfterDurableCleanCannotReconcileStale|staleLaterCutAfterDurableCleanIsRejected|newerDurableBoundaryStalesOlderClean|resolvingOneCaptureDoesNotDischargeAnother|unknownLaterCutBeforeCleanCannotReconcileAfterClean|rejectedLaterCutRefusesEffect|removalNeedsExactExternalAuthorization|releasedHistoryNeedNotRemainEnumerable|activeHistoryRemainsEnumerable|openCaptureRetainsReleasedMembership|unknownCaptureRetainsReleasedMembership)$' \
  --max-samples 1 --seed 22082026
=> 28 passing

quint verify verification/quint/CodedRangeCleanConflictExhaustive.qnt --main CodedRangeCleanConflictExhaustive --max-steps 3 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanCaptureExhaustive.qnt --main CodedRangeCleanCaptureExhaustive --max-steps 5 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt --main CodedRangeCleanUncertaintyExhaustive --max-steps 4 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanCompositionExhaustive.qnt --main CodedRangeCleanCompositionExhaustive --max-steps 6 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt --main CodedRangeCleanTwoCapturePhaseCut --init profileInit --max-steps 14 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt --main CodedRangeCleanCleanCommitPhaseCut --step phaseStep --max-steps 6 --invariants ProfileInvariants --verbosity 1
quint verify verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt --main CodedRangeCleanLaterCutPhaseCut --step phaseStep --max-steps 6 --invariants ProfileInvariants --verbosity 1
=> no violation found at every declared bound

quint test verification/quint/CodedRangeCleanMutants.qnt --main CodedRangeCleanMutants \
  --match 'perMemberInsteadOfCodedConflictFails|connectedComponentSerializationKillsDisjointWitness|partialAdmissionBeforeEffectFails|unclassifiedAdmissionFailsExhaustiveness|disappearingIncludedOperationFailsRetention|laterEffectWithoutCutFails|unsatisfiedIncludedWorkCannotBeClean|rejectedSatisfactionCannotRemainActive|unknownCleanCannotBecomeKnown|unknownLaterCutCannotAllowEffect|staleCaptureCannotRemainClean|removalWithoutExternalReleaseAllowedFails' \
  --max-samples 1 --seed 22082026
=> all 12 deliberately bad paths failed their expected safety predicate
   (`QNT508`); the negative-suite command exit is intentionally non-zero

cargo test -p dwv-recovery -p dwv-transaction-ref -p dwv-service --lib
=> 205 passed

cargo test -p dwv-service coded_range_clean_connect -- --nocapture
=> 32 passed

cargo test -p dwv-service generated_coded_operation_histories -- --nocapture
=> 10,000 deterministic histories passed with fixed seed 22082026

cargo test -p dwv-recovery capture_scope_matches_independent_block_overlap_oracle
cargo test -p dwv-service coded_range_connect_clean_retirement
cargo test -p dwv-service persisted_refused_cleanup_handles_known_noncommit_and_lost_ack
cargo test -p dwv-service inherited_clean_known_abandonment_handles_known_noncommit_and_lost_ack
cargo test -p dwv-service repeated_crash_before_empty_clean_cleanup_blocks_admission_until_retired
cargo test -p dwv-service coded_lifecycle_lost_ack_reconciles_release_compaction_and_cleanup_on_reopen
cargo test -p dwv-service reopened_refusal_lost_ack_installs_neither_candidate_until_reopen
cargo test -p dwv-service rejected_coded_release_commit_retries_before_slot_reuse
cargo test -p dwv-recovery coded_clean::tests -- --nocapture
cargo test -p dwv-service coded_ -- --nocapture
=> 19 recovery lifecycle tests and 37 coded service/Connect tests passed

=> each focused regression passed

cargo test --workspace --all-features -- --test-threads=1
=> 463 passed, 1 ignored

cargo check --workspace --all-targets --all-features
cargo clippy -p dwv-recovery -p dwv-service --all-targets --all-features -- -D warnings
cargo fmt --all --check
=> all exited 0

openspec validate --all --strict
=> 33 passed, 0 failed

cargo xtask docs knowledge readiness
=> ready: true; 169 requirements; every gate count 0

cargo xtask docs check
cargo xtask docs build
=> both exited 0; 169 documentation objects built
```

The full workspace test command ran with one test thread so retained global
fixtures could not contend. Full-workspace Clippy currently stops in unchanged
`xtask/src/knowledge.rs:3835` on Rust 1.97's `clippy::type_complexity`; the
changed-crate command above passed with all warnings denied. This check does not
claim that unrelated workspace lint is clean.

Non-claims:

- This is bounded model-to-production correspondence evidence, not arbitrary-
  width concurrency proof, production throughput evidence, or hardware
  durability evidence.
- Typed external owner inputs do not prove the owner semantics they represent.
  LifecycleRelease provider correctness, physical cleanup ordering, basis
  coherence, topology/profile mapping, request/store/dirty/checksum geometry,
  persistence admissibility, startup, shutdown, publication, currentization,
  and Linux behavior remain separate evidence or later work.

# Verification-preparation machinery (bounded, 2026-08-25)

The existing Connect bridge, retained-operation harness, `dwv-sim` schedules,
and recovery manifest/reopen APIs were reused. No second CodedRangeClean
state machine, timing scheduler, or production mutation framework was added.

Connect sampling (bounded diversity/replay) and false-projection controls:

```text
cargo test -p dwv-service coded_range_connect_sampled_operation_capture_paths -- --nocapture
=> 1 passed; 16 seeded legal traces, max-steps 12; deterministic
   diversity/replay evidence only

cargo test -p dwv-service coded_range_connect_rejects_false_projections -- --nocapture
=> 1 passed; four deliberately false projections were rejected
```

The false projections are premature effect permission, a dropped active claim,
swapped capture membership, and suppressed release. Production state still
comes from service/coordinator observations; the model action only selects the
requested transition.

The 16 sampled seeds do not carry an aggregate category-coverage or witness-
frequency claim. Explicit Connect profiles and the bounded model profiles own
the operation-admission, capture, effect, release, and uncertainty witnesses.

Generated stateful operation evidence:

```text
cargo test -p dwv-service generated_coded_operation_histories -- --nocapture
=> 1 passed; 64 generated cases, 1-23 transitions per case, including legal
   no-active-operation crash/restart transitions

cargo test -p dwv-service generated_operation_identity_is_independent_of_target_member -- --nocapture
=> 1 passed; operation identity is independent of target-member input

cargo test -p dwv-service minimized_coded_operation_history_replays_deterministically -- --nocapture
=> 1 passed; the retained history includes two crash/restart boundaries

cargo test -p dwv-service generated_property_rejects_corrupted_observation_and_replays -- --nocapture
=> 1 passed; the ordinary property oracle rejected a hidden active claim,
   proptest shrank it to one transition, and the minimized history replayed
   with the same failure
```

The generated machine uses independent operation identities and target-member
inputs. Operation identity keys reference and release state; target-member
input only feeds the request. A restart is legal only with no active operation;
the production reopen path atomically invalidates inherited capture regions and
checksum extents before removal, then starts a new bounded capture. The
deterministic retained history exercises two such boundaries. The ordinary
invariant/correspondence checker consumes a test-only observation that hides an
actually active claim. Its normal failure path is what shrinks and replays; no
second CodedRangeClean machine or custom property assertion is substituted.

Deterministic capture and crash/reopen boundaries:

```text
cargo test -p dwv-service protected_write_contention_parks_until_capture_cut_progresses -- --nocapture
=> 1 passed; accepted work pauses, capture starts, a later operation is admitted,
   capture blocks its effect, and the retained operation resumes after a durable cut

cargo test -p dwv-recovery durable_reopen_cut_discards_uncommitted_owner_state -- --nocapture
=> 1 passed

cargo test -p dwv-sim daemon_crash_and_power_loss_have_distinct_effects -- --nocapture
=> 1 passed
```

`RecoveryReopenCut` captures only durable exported owner state. `dwv-sim`
continues to own volatile-media and power-loss behavior. These checks prepare
the capture/reopen handoff boundary without defining durable CLEAN clearing or
healthy-service persistence transitions.

# Protected-write coded authority integration (focused)

Source:

- `crates/dwv-service/src/service.rs`
- `crates/dwv-service/src/service/tests.rs`
- `crates/dwv-service/src/service/tests/coded_range_clean_connect.rs`

The retained protected-write driver now derives complete coded-unit coverage
from the validated captured topology's logical codeword blocks. The mapping
does not use member or store identity, so operations on different data members
that affect one codeword contend while disjoint codeword ranges can progress
independently.

The independent overlap oracle test enumerates every non-empty contiguous
request within an eight-codeword geometry under every fixed-size chunking of
that request. It derives expected coded units from half-open interval overlap,
not from the production mapping loop, and compares the complete unit set.

The retained path observes coded admission before the first basis work, keeps
the claim through both basis results, permits the consuming effect only after
the basis phase, and consumes the exact lifecycle `ReleaseAuthorization` before
removing the coded claim. Ordinary contention returns a non-terminal wait and
the same `PortableWriteSubmission` resumes after the holder releases. Capture
blocking remains a non-terminal wait without coded re-admission. Failed or
uncertain basis work retains the claim until matching owner-approved
reconciliation supplies the exact release authorization. The blocking `write`
facade drives this same retained path; a pre-admission coded contender is
released locally and reported as explicit retry/backpressure, not I/O failure.
The current ublk profile serializes `execute_service` under the opened-service
mutex, so ordinary coded contention is unreachable between Linux requests.
`Retry` remains a portable retained-execution result; this profile does not
claim ordinary Linux request parking or `EAGAIN` completion semantics.

Focused evidence:

```text
cargo test -p dwv-service protected_write_ -- --nocapture
=> 5 passed
cargo test -p dwv-service blocking_write_ -- --nocapture
=> 2 passed

cargo test -p dwv-service blocking_contention_rejects_only_the_unstarted_request -- --nocapture
=> 1 passed

cargo test -p dwv-frontend-ublk coded_contention_maps_to_retryable_result -- --nocapture
=> 1 passed

cargo test -p dwv-service blocking_cancel_refuses_after_transaction_range_acquired -- --nocapture
=> 1 passed

cargo test -p dwv-service failed_child_admission_is_a_pre_transaction_bounded_refusal -- --nocapture
=> 1 passed

cargo test -p dwv-transaction-ref coded_claim_supports_unit_above_u32_boundary -- --nocapture
=> 1 passed

cargo test -p dwv-service coded_claim_supports_unit_above_u32_boundary_without_large_fixture -- --nocapture
=> 1 passed

cargo test -p dwv-service coded_claim_mapping_matches_independent_codeword_overlap_oracle -- --nocapture
=> 1 passed

cargo test --bin dwv -- --nocapture
=> 3 passed

cargo test -p dwv-service coded_range_connect_protected_write_holds_claim_through_basis_and_write -- --nocapture
=> 1 passed
```

Non-claims:

- These are focused Rust and production-side Connect observations, not
  arbitrary-width concurrency proof, basis-content correctness, durability
  certification, or hardware evidence.
- The CodedRangeClean model remains bounded and does not model basis lifecycle,
  topology mapping, persistence admissibility, or recovery-owner correctness.
- Capture/frontier production, durable completion, `MarkRegionClean`, and
  owner-approved bounded retention are integrated here. This evidence does not
  establish arbitrary-width liveness, production hardware durability, or
  future retention-policy adequacy.

# Coded CLEAN structural-authority repair (focused, bounded, Round 18, 2026-08-30)

The corrective implementation makes one opaque coded semantic transition own
each durable capture successor and every required selected-state side effect.
Production coded claims are issued by the geometry owner for the exact admitted
operation and request. Retained capture-capacity exhaustion now parks the
existing admitted write until durable retirement makes capacity available.

The Round-9 repair moves final lifecycle composition back to
`HealthyPortableService`. The service wraps lower process-domain signing
primitives in service-owned opaque release and Included types. The generic
`CodedLifecycleAuthority` consumer is specialized on those exact types and
exposes no lifecycle fact issuer; raw range release and capture CLEAN
authorization are crate-private. A lower lifecycle capability, slot permit,
capture identity, coordinator, or fence certificate cannot be promoted into
the service-owned capability type. Released Included evidence is produced only
inside the service path after the installed capture snapshot carries the exact
durably confirmed release operation and certificate.

The Round-10 repair binds every `CodedClaimInput` and `CodedRangeAuthority` to
the same unforgeable geometry-authority identity. The paired range rejects a
claim issued by a separately constructed geometry owner before stamping a
local admission, even when topology and request identity remain valid. The
existing admission-authority identity separately prevents an admission from
one range from entering another capture graph. Admission observation and
later-cut preparation therefore accept only the paired geometry/range graph
before membership or target authority can emerge. Later cuts still derive
their invalidation target from the accepted admission, and confirmation still
requires the exact committed target-bearing transition. Prepared CLEAN retains
the owner-approved aggregate certificate and requires its exact committed
transition. Restart still refuses to reconstruct retained-history authority
from frontier equality, and Connect later cuts still use the production
write-recovery producer.

The Round-11 repair closes the Round-10 external P1 by moving capture
establishment behind one mutable `CodedLifecycleAuthority` operation. That
owner derives the capture boundary from its current paired range authority and
the future-inclusive scope from its paired geometry owner. The raw boundary
issuer and coordinator acceptor are crate-private, and the former public
capture-establishment types are no longer exported. A stale range clone can no
longer issue a cut, and a foreign geometry owner can no longer submit a scope.
The maintained regressions retain the exact stale-clone and foreign-geometry
constructions while verifying that the installed capture contains the live
admission and the paired geometry scope.

The Round-12 repair makes `CodedLifecycleAuthority` the only ordinary producer
of media-effect authority. The raw range permit issuer is crate-private. The
composed owner requires the exact active range claim and checks its coded units
against every active capture: missing membership and unresolved `Later`
membership both block effect authority, while only owner-recorded permitted
membership can reach the raw issuer.

The Round-13 repair closes the Round-12 external P1 by making that composition
exclusive rather than another mutable authority path. The lifecycle owner no
longer exposes mutable range or capture owners through accessors or dereference,
and neither lower mutable owner is cloneable. Admission prepares candidate
range and capture successors from one predecessor and installs both together
only after exact capture persistence is confirmed. Capture-start scope and cut
come from the same paired owner. Every prepared capture successor carries its
issuing owner identity through CLEAN and later-cut uncertainty handling, so a
different live owner rejects it before installation. Runtime regressions cover
stale range preparations, cross-owner prepared capture successors, capture
ordering bypasses, and paired capture scope. Compile-fail barriers cover owner
extraction, replacement, cloning, raw effect issuance, raw capture start, and
raw prepared-transition installation. No Round-13 change alters the delegated
Quint model.

The corrected Round-14 external P1 was not accepted on review authority alone.
A temporary regression independently reproduced the claimed sequence: the
capture transition became durable, the caller returned an error, and the old
process-local owner still issued media-effect authority. The repair removes
caller-supplied capture persistence. `start_capture` now constructs and submits
the exact recovery transaction itself while retaining its exclusive owner
borrow. An explicit rejected observation leaves the exact predecessor usable.
A lost, corrupt, or otherwise uncertain observation invalidates the complete
process-local coded authority and forces the service into `Recovering`; every
dependent admission, effect, release, CLEAN, later-cut, compaction, and cleanup
transition then fails closed until durable reopen constructs a new owner.
Durable success installs only the exact receipt-validated capture and range
successors. A mismatched post-durability receipt also invalidates the owner.
Permanent regressions cover rejected, lost-acknowledgement, and mismatched
durable-receipt branches.

The Round-15 external P1 was also reproduced independently before repair. An
emitted but unaccepted physical write retained an earlier coded effect permit,
then crossed backend acceptance after a lost capture acknowledgement had
invalidated the complete process-local lifecycle owner. Retained write driving
now emits no new work while the service or lifecycle owner requires
reconciliation, and backend acceptance rechecks both conditions. The accepted
side of that boundary is also structural: `accept_write_work` returns one
opaque, non-cloneable `AcceptedPortableWriteWork`, and both normalized physical
execution and `PortableWriteResult::new` consume that proof. Raw emitted work
cannot invoke those paths. Work accepted before invalidation may complete under
the existing conservative in-flight rules; emitted-but-unaccepted, planned,
and fresh work cannot cross a new backend-submission boundary. The permanent
regression distinguishes all four cases, and compile-fail checks prevent
ordinary callers from forging accepted-work proof or reporting raw emitted
work as completed.

The corrected Round-16 external P1 was independently reproduced across the
production sibling transition graph before repair. Lost acknowledgements for
coded admission, write-recovery later cut, CLEAN commit, release, membership
compaction, refused and `CleanKnown` cleanup, and inherited abandonment could
leave the exact process-local predecessor installed and still usable after the
durable store accepted the successor. The service now routes every
`ApplyCodedTransition` persistence result through one conservative boundary.
Only explicit `Rejected` preserves the predecessor. Lost, corrupt, or any
other non-rejection observation invalidates the complete lifecycle authority
and moves the service to `Recovering`. Every post-durability confirmation,
including receipt-generation or predecessor mismatch, passes through the same
invalidation rule. Capture start retains its existing owner-internal equivalent.
Permanent regressions cover explicit rejection, lost and corrupt
acknowledgements, admission, later cut, CLEAN commit, release, compaction,
phase-specific cleanup, inherited abandonment, exact durable reopen, and a
mismatched durable compaction receipt.

The Round-17 final-angle external P2 identified one remaining ordinary
production escape: `HealthyPortableService::recovery_mut` returned the raw
mutable recovery owner, allowing a caller to bypass the service-mediated coded
transition boundary and submit arbitrary recovery mutations. Repository-wide
references confirmed that only an internal service test used the accessor.
The repair deletes it; that descendant test reaches the service's private
recovery field only to inject recovery-health failure. A compile-fail barrier
proves that an ordinary consumer cannot obtain the mutable recovery owner.

Fresh focused results:

```text
cargo test -p dwv-recovery --lib
=> 80 passed

cargo test -p dwv-service coded_range_clean_connect
=> 49 passed

cargo test -p dwv-service lifecycle_owner_withholds_authority_from_every_incomplete_fact_set
cargo test -p dwv-service coded_release_rejects_authorization_from_foreign_owner_domain
cargo test -p dwv-recovery foreign_admission_cannot_enter_another_capture_authority_graph
cargo test -p dwv-service paired_range_rejects_claim_from_foreign_geometry_owner
=> 1 passed each

cargo test -p dwv-service coded_capture_and_admission_linearize_at_service_boundary
cargo test -p dwv-service capture_start_uses_the_paired_geometry_scope
=> 1 passed each

cargo test -p dwv-service lifecycle_effect_authority_rejects_capture_ordering_bypasses
=> 1 passed

cargo test -p dwv-service lifecycle_owner_rejects_
=> 2 passed

cargo test -p dwv-service rejected_capture_persistence_keeps_the_live_range_predecessor -- --test-threads=1
cargo test -p dwv-service lost_capture_ack_invalidates_process_local_coded_authority -- --test-threads=1
cargo test -p dwv-service mismatched_durable_capture_receipt_invalidates_process_local_coded_authority -- --test-threads=1
=> 1 passed each

cargo test -p dwv-service uncertain_coded_admission_revokes_process_local_authority -- --test-threads=1
cargo test -p dwv-service corrupt_coded_admission_ack_revokes_process_local_authority -- --test-threads=1
cargo test -p dwv-service uncertain_later_cut_revokes_process_local_coded_authority -- --test-threads=1
cargo test -p dwv-service uncertain_clean_commit_revokes_process_local_coded_authority -- --test-threads=1
cargo test -p dwv-service coded_lifecycle_lost_ack_reconciles_release_compaction_and_cleanup_on_reopen -- --test-threads=1
cargo test -p dwv-service mismatched_durable_compaction_receipt_revokes_process_local_authority -- --test-threads=1
=> 1 passed each

cargo test -p dwv-service capture_invalidation_revokes_only_unaccepted_physical_work -- --test-threads=1
=> 1 passed

cargo test -p dwv-recovery disjoint_active_capture_does_not_gate_later_effect -- --test-threads=1
=> 1 passed

cargo test -p dwv-service coded_capture_capacity_parks_admitted_write_until_retirement_progresses -- --nocapture
=> 1 passed

cargo test -p dwv-recovery --doc
=> 13 compile-fail barriers passed

cargo test -p dwv-service --doc
=> 6 compile-fail barriers passed, including the mutable recovery-owner
   boundary and 2 accepted-work barriers

cargo test --workspace --all-features -- --test-threads=1
=> 515 passed, 1 ignored

cargo check --workspace --all-targets --all-features
cargo clippy -p dwv-lifecycle-authority -p dwv-recovery -p dwv-service --all-targets --all-features -- -D warnings
cargo fmt --all --check
openspec validate --all --strict
cargo xtask docs knowledge readiness
cargo xtask docs check
cargo xtask docs build
=> all completed successfully; strict OpenSpec validation passed 33 items,
   knowledge readiness covered 169 requirements with every gate count zero,
   and the documentation build produced 169 objects
```

The Round-8 Quint results remain the current bounded model evidence; Rounds
9-18 changed Rust authority composition and maintained evidence, not the Quint
sources or commands:

```text
quint typecheck models/quint/CodedRangeClean.qnt
quint typecheck verification/quint/CodedRangeCleanAnalysis.qnt
quint typecheck verification/quint/CodedRangeCleanConnect.qnt
quint test models/quint/CodedRangeClean.qnt
=> all completed successfully

quint test verification/quint/CodedRangeCleanAnalysis.qnt \
  --main CodedRangeCleanAnalysis --match <maintained-28-scenario-selector> \
  --max-samples 1 --seed 22082026
=> 28 passing

quint verify verification/quint/CodedRangeCleanConflictExhaustive.qnt ...
quint verify verification/quint/CodedRangeCleanCaptureExhaustive.qnt ...
quint verify verification/quint/CodedRangeCleanUncertaintyExhaustive.qnt ...
quint verify verification/quint/CodedRangeCleanCompositionExhaustive.qnt ...
quint verify verification/quint/CodedRangeCleanTwoCapturePhaseCut.qnt ...
quint verify verification/quint/CodedRangeCleanCleanCommitPhaseCut.qnt ...
quint verify verification/quint/CodedRangeCleanLaterCutPhaseCut.qnt ...
=> no violation found in six profiles; the CompositionExhaustive retry
   reached its 600-second limit without a result

quint test verification/quint/CodedRangeCleanMutants.qnt ...
=> all 12 deliberately bad paths failed their expected safety predicate
   (`QNT508`); the command's non-zero exit is expected
```

The Round-8 rerun executed the seven maintained bounded verification profiles
concurrently. These finite results do not establish arbitrary-width concurrency
or liveness.

Non-claims:

- Complete transition-group validation is deterministic in the in-memory
  recovery adapter. It is not hardware durability certification.
- The Rust and Connect checks exercise production composition at their stated
  finite bounds. They are not arbitrary-width concurrency or liveness proof.
- The Round-8 CompositionExhaustive timeout remains an open input to the
  external completion gate.
- A fresh exact-snapshot external review remains required after the Round-17
  final-angle repair; this evidence does not claim external completion.
