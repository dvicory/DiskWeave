# Portable verification evidence log

Date: 2026-08-08

## Baseline

The pre-remediation portable workflow is recorded in
[`portable-demo-baseline.md`](portable-demo-baseline.md), including the
disposable fixture, commands, outputs, and claim boundary.

## VE-002 focused evidence (historical pre-repair canary)

Date: 2026-08-17

Source model: `models/quint/RecoveryProtocol.qnt`

Authority status: the already-synced `RecoveryProtocol` surface is the
current canonical authority for the bounded VE-002 reference
transaction/recovery relation. The active repair below extends that relation
as a proposed target only; it does not change current OpenSpec authority until
normal verification and sync. The separate `RecoveryProtocolAnalysis` module
at `verification/quint/RecoveryProtocolAnalysis.qnt` binds finite evidence
instances and its runs are verification evidence, not semantic authority.
The archived `replace-ve002-tla-with-quint` and
`reconcile-ve002-quint-ownership` changes established this boundary. Quint
does not own typed evidence, topology, persistence, operation-slot lifetime,
frontend delivery, or production recovery authority.

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

The Quint evidence is bounded to `MaxDepth = 8`, two affected regions, and
two stores. It does not claim exhaustive model checking, Rust implementation
correctness, exact region/checksum mapping, typed fence admissibility,
topology, persistence-engine behavior, operation-slot lifetime, frontend
behavior, physical durability, or unbounded recovery progress.

## VE-002 authority repair (proposed)

Date: 2026-08-17

Active change: `repair-ve002-quint-authority`

This section records target evidence for the active repair. It does not
change the current OpenSpec authority until the change is verified and
synced. The pre-repair record above is retained as historical canary
evidence; the claims below describe the proposed target.

The target relation adds explicit range ownership and release, permits a new
`begin` after release, gives pre-mutation rejection an aborted outcome,
blocks terminal reuse before release, preserves invalid/repeated transition
absence, and requires complete represented mutation coverage before durable
home reconciliation. The finite analysis adds release/reuse,
aborted-release, volatile-abandonment, continuation, and terminal-release
witnesses. Regions, stores, and depth remain evidence bounds.

Commands:

```text
quint typecheck models/quint/RecoveryProtocol.qnt
quint typecheck verification/quint/RecoveryProtocolAnalysis.qnt
quint typecheck verification/quint/RecoveryProtocolConnect.qnt
quint verify verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 8 \
  --invariants TypeInvariant,NoFalseClean,MutationRequiresIntent,UncertaintyIsVisible,DurableWorkIsOwned,TerminalRequiresRelease,TerminalRequiresEvidence,DurableHomeRequiresCoverage,FenceAndCheckpointCoverage
quint test verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis
quint run verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 12 \
  --max-samples 10000 \
  --seed 22082026 \
  --invariants TypeInvariant \
  --invariants NoFalseClean \
  --invariants MutationRequiresIntent \
  --invariants UncertaintyIsVisible \
  --invariants DurableWorkIsOwned \
  --invariants TerminalRequiresRelease \
  --invariants TerminalRequiresEvidence \
  --invariants DurableHomeRequiresCoverage \
  --invariants FenceAndCheckpointCoverage \
  --witnesses beginReachable \
  --witnesses durableIntentReachable \
  --witnesses mutationReachable \
  --witnesses uncertainIntentReachable \
  --witnesses uncertainHomeReachable \
  --witnesses reconciliationReachable \
  --witnesses terminalReachable \
  --witnesses terminalPendingReleaseReachable \
  --witnesses abortedReachable \
  --witnesses releasedReachable \
  --witnesses resumedMutationReachable \
  --witnesses durableHomeCoverage
cargo test -p dwv-transaction-ref --lib
cargo test -p dwv-transaction-ref --test quint_connect -- --nocapture
```

Observed:

- All three Quint sources typechecked. `quint verify` found no invariant
  violation at `MaxDepth = 8`.
- The Quint analysis reported five passing tests:
  `boundedAssumptionsTest` passed once, and
  `terminalReleaseRequiredTest`, `terminalBeginBlockedTest`,
  `releaseEndsLifecycleTest`, and `partialHomeCannotBeDurableTest` each
  passed 10,000 randomized cases. `releaseEndsLifecycleTest` verifies that
  release permits the next `begin`.
- The sampled run found no invariant violation across 10,000 traces.
- Witnesses were reached in 10,000/10,000 begin traces, 9,169/10,000
  durable-intent traces, 8,619/10,000 mutation traces, 6,739/10,000
  uncertain-intent traces, 7,931/10,000 uncertain-home traces,
  9,997/10,000 reconciliation-handoff traces, 73/10,000 terminal and
  terminal-pending-release traces, 3,344/10,000 aborted traces,
  3,392/10,000 released traces, 7,931/10,000 resumed-mutation traces, and
  10,000/10,000 durable-home-coverage traces.
- Two same-seed ITF runs produced identical normalized state content after
  generated metadata were removed; each trace contained 13 states.
- A disposable copy with the terminal-begin guard removed failed
  `terminalBeginBlockedTest` with `QNT511`; a disposable copy with the
  partial-home durable-outcome guard removed failed
  `partialHomeCannotBeDurableTest` with `QNT508`; and a disposable copy
  admitting durable intent without the pending-intent guard violated
  `NoFalseClean` and `DurableWorkIsOwned`. No mutants are retained.
- The focused Rust transaction-machine library suite passed 17 tests. Two
  Quint Connect tests passed with seeds `22082026` and `1`, covering normal
  completion and uncertain-home reconciliation. The driver projects only
  intent/observation, home, recovery, obligation, invalidation, range
  ownership, terminal-pending-release, and release-marker state.
- The Connect projection is intentionally bounded, not a conformance bridge.
  Quint `mutate` advances one abstract region while Rust `mutate` batches
  read completion, parity computation, and write completion. Quint `fence`
  and `reconcileHome` are abstract observations with no independent Rust
  action in this driver; Rust's typed flush, checkpoint, crash, and
  reconciliation stages carry those effects. Rust also exposes typed fence,
  watermark, generation, topology, and result-class evidence that the model
  does not represent. Exact region sets, attempted/mutated coverage,
  private stage layout, and backend identity are not compared.

The target evidence remains bounded to `MaxDepth = 8`, two affected
regions, two stores, sampled execution, and the two mapped Connect paths. It
does not claim exhaustive arbitrary-width model checking, Rust
implementation correctness, exact region/checksum mapping, typed fence
admissibility, topology, persistence, operation-slot lifetime, frontend
behavior, physical durability, or unbounded recovery progress.

## VE-001 focused evidence

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
2.66 seconds and proves VP-002's bounded split/coverage arithmetic. Public
list/allocation and error-formatting behavior remains covered by the bounded
Rust tests and later property/fuzz layers. Kani diagnostics for
`caller_location` and foreign functions were emitted as successful checks,
not proof failures.

Kani claim map: the core and service harnesses support VP-002; the codec
parity, incremental-update, and single-erasure harnesses support VP-001.
These runs do not establish arbitrary-width `u64` proof, P/Q behavior,
concurrency, recovery ordering, filesystem/device I/O, or physical durability.

## VE-005 initial structured corpus evidence

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

This is initial VP-010 hostile trace-input and VP-011 reproducibility evidence.
It does not claim broad simulator operation/fault/crash/topology schedules,
parity generation, concurrency, filesystem behavior, or hardware durability.

## VE-005 seeded simulator schedule corpus evidence

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

This supports initial VP-005, VP-008, VP-010, and VP-011 evidence. It does not
claim exhaustive schedule coverage, topology generation, parity equivalence,
concurrency, filesystem behavior, or hardware durability.

## VE-005 bounded parity, topology, and envelope corpus evidence

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

This extends bounded VP-001, VP-002, VP-009, VP-010, and VP-011 evidence. The
corpus remains finite and does not claim arbitrary-width proof, exhaustive
topology space, concurrency, filesystem behavior, Linux behavior, or hardware
durability.


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

## Post-VE-001 integrated checkpoint

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