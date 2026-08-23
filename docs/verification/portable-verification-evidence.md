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
=> all ten exited 0 with no output
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
and two coded units; its strongest completed bound is depth 6.

The attempted capture depth 7 and composition depth 13 checks timed out
without a result (300 seconds and 900 seconds, respectively) and are not
claimed. No deeper bound is claimed.

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
  capture, uncertainty, and composition. Phase-cut bounds are depth 6 each.
  Timed-out depth 7, depth 10, and depth 13 attempts are not successes.
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
operation generation without reconstructing lifecycle predicates.

# CodedRangeClean Connect projection (focused, bounded)

Date: 2026-08-23

Source:

- `crates/dwv-service/src/service/tests/coded_range_clean_connect.rs`
- `verification/quint/CodedRangeCleanConnect.qnt`
- `crates/dwv-transaction-ref/src/coded.rs`
- `crates/dwv-recovery/src/coded_clean.rs`
Canonical `models/quint/CodedRangeClean.qnt` SHA-256:
`dc75a93cba0724d12235e5a73a2c7b237b45330aa9a715714ceb75cbd8cf3b9d`.

The in-crate driver uses production `HealthyPortableService`,
`OperationAdmission`, `CodedRangeAuthority`, and
`CodedCaptureCoordinator` instances. `BridgeState` projects model state from
exact generation-qualified operation tokens and production coordinator
snapshots. Ordinary overlap and capture exclusion are normalized
non-terminal outcomes. Lifecycle authorization is an external typed input;
basis coherence remains separate provider/conformance evidence and is not a
CodedRangeClean bridge input. The bridge does not reconstruct either policy,
retain a model-shaped coordinator, or use a second semantic state machine.

Projection table:

| Model distinction | Direct provider correspondence | Typed external owner input |
|---|---|---|
| Exact operation identity, complete coded claim admission, disjoint coexistence, and effect phase | `BridgeDriver::admit` → `reserve` → `coded_admit`; the service requires the exact slot to remain `Reserved` at coded admission; active phase/claims project from exact-token `CodedRangeAuthority`; released model claim fields retain only admitted claim evidence needed for the model comparison; `permit_effect` → `coded_permit_effect` | Complete validated claims are typed external mapping inputs; ordinary overlap is `CodedAdmissionOutcome::Contended`; blocker identity and scheduling are not part of this result |
| Capture scope, Included/Later membership, and capture phase | `start_capture` → `coded_start_capture`; `bridge_state` → `CodedCaptureSnapshot` | Complete validated `CodedCaptureScopeInput` with covered lower frontier |
| Capture-wide accepted/rejected decision and no-false-CLEAN gating | `request_clean` → `CodedCaptureCoordinator::request_clean` | `accept_capture`/`reject_capture` supply `CodedCaptureDecision` |
| CLEAN Durable/Rejected/Unknown and authoritative reconciliation | `clean_commit` → `observe_clean_commit`; `reconcile_clean` → `reconcile_clean_commit` | `CodedCleanCommitObservation` and `CodedCleanReconciliation` are recovery-owner observations |
| Later durable-cut ordering, Unknown blocking, and reconciliation | `later_cut`/`later_cut_reconcile` call the production coordinator; effect attempts call `coded_permit_effect` and return `BlockedByCapture` when excluded | Later-cut commit and reconciliation observations are typed owner evidence |
| Exact release authorization and coded claim removal | `release_operation` passes an exact typed `ReleaseAuthorization` to `coded_release_claim`; the returned exact generation is retained as bounded observed ledger evidence | LifecycleRelease provider correctness, physical cleanup ordering, lifecycle predicates, and basis coherence are external evidence; this bridge does not reconstruct them |

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
=> 4 passed

cargo test -p dwv-service coded_range_connect -- --nocapture
=> 15 passed

cargo test -p dwv-service coded_release_certificate -- --nocapture
=> 1 passed

cargo test -p dwv-service coded_capture_and_admission -- --nocapture
=> 1 passed

cargo test -p dwv-service coded_same_generation_cannot_be_readmitted_after_release_before_slot_cleanup -- --nocapture
=> 1 passed
```

Non-claims:

- This is bounded model-to-production correspondence evidence, not arbitrary-
  width concurrency proof, production throughput evidence, or hardware
  durability evidence.
- Typed external owner inputs do not prove the owner semantics they represent.
  LifecycleRelease provider correctness, physical cleanup ordering, basis
  coherence, topology/profile mapping, request/store/dirty/checksum geometry,
  persistence admissibility, startup, shutdown, publication, currentization,
  and Linux behavior remain separate evidence or later work.
